use crate::{
    controller::TweenController,
    function::{MinimalTweenFnAt, TweenContext, TweenFn, TweenFnAt},
    key::{TweenKey, TweenKeyUpdateArgs},
    marker::{TweenMarker, TweenSchedule},
    property::TweenPropertySet,
    target::{TweenTarget, TweenTargetOptions},
    tweenable::Tweenable,
};
use bevy_app::{App, FixedUpdate, Last, Plugin, Update};
use bevy_ecs::{
    change_detection::Res,
    component::{Component, Mutable},
    entity::Entity,
    event::{EntityEvent, Event},
    hierarchy::ChildOf,
    lifecycle::HookContext,
    message::Message,
    resource::Resource,
    schedule::{IntoScheduleConfigs, SystemSet},
    system::{Commands, Query, SystemId},
    world::{DeferredWorld, World},
};
use bevy_log::warn;
use bevy_math::curve::{Curve, EaseFunction};
use bevy_time::{Time, Timer, TimerMode};
use std::{any::TypeId, collections::HashMap, marker::PhantomData, mem, time::Duration};

pub mod prelude {
    pub use super::{
        Tween, TweenFinished, TweenPlugin, TweenSystems,
        controller::TweenController,
        function::{
            MinimalTweenFnAt, TweenContext, TweenFn, TweenFnAt, TweenKeyContext, TweenKeyFn,
        },
        key::{TweenKey, TweenKeyFinished},
        marker::{TweenFixedUpdate, TweenMarker, TweenSchedule, TweenUpdate},
        property::TweenPropertySet,
        target::{TweenKeyTarget, TweenTarget},
        tweenable::{TweenStep, TweenStepAt, Tweenable},
    };
}

pub mod controller;
pub mod function;
pub mod key;
pub mod marker;
pub mod property;
pub mod target;
pub mod tweenable;

pub struct TweenPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TweenSystems;

impl Plugin for TweenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TweenRegistry>()
            .add_systems(
                FixedUpdate,
                run_fixed_update_tween_systems.in_set(TweenSystems),
            )
            .add_systems(Update, run_update_tween_systems.in_set(TweenSystems))
            .add_systems(Last, tween_controller_discard_pending_schedules);
    }
}

fn run_update_tween_systems(world: &mut World) {
    run_tween_systems(world, |registry| &mut registry.update_systems);
}

fn run_fixed_update_tween_systems(world: &mut World) {
    run_tween_systems(world, |registry| &mut registry.fixed_update_systems);
}

fn tween_controller_discard_pending_schedules(tween_controllers: Query<&mut TweenController>) {
    for mut tween_controller in tween_controllers {
        if tween_controller.read_by_tween {
            tween_controller.discard_pending_schedules();
        }
    }
}

fn run_tween_systems(
    world: &mut World,
    systems_selector: impl Fn(&mut TweenRegistry) -> &mut Vec<SystemId>,
) {
    let mut systems = {
        let mut registry = world.resource_mut::<TweenRegistry>();
        mem::take(systems_selector(&mut registry))
    };

    systems.retain(|system_id| match world.run_system(*system_id) {
        Ok(()) => true,
        Err(err) => {
            warn!("Tween system failed and was removed: {err:?}");
            false
        }
    });

    let mut registry = world.resource_mut::<TweenRegistry>();
    let selector_systems = systems_selector(&mut registry);
    if !selector_systems.is_empty() {
        systems.append(selector_systems);
    }
    *selector_systems = systems;
}

#[allow(clippy::type_complexity)]
fn update_tweens<T, P, M>(
    time: Res<Time>,
    tweens: Query<(
        Entity,
        &mut Tween<T, P, M>,
        Option<&ChildOf>,
        Option<&mut TweenController>,
    )>,
    mut targets: Query<&mut T>,
    mut commands: Commands,
) where
    T: Component<Mutability = Mutable>,
    P: Tweenable + Send + Sync + 'static,
    M: TweenMarker + Send + Sync + 'static,
{
    let delta = time.delta();
    for (entity, mut tween, maybe_child_of, maybe_tween_controller) in tweens {
        if let Some(mut tween_controller) = maybe_tween_controller {
            tween_controller.apply_to(&mut tween);
        }

        let target_options = TweenTargetOptions {
            this: entity,
            parent: maybe_child_of.map(|c| c.parent()),
        };
        tween.update(delta, &mut targets, target_options, &mut commands);
    }
}

//==================================================================================================
// TweenRegistry
//==================================================================================================

#[derive(Resource, Default)]
struct TweenRegistry {
    system_map: HashMap<TypeId, SystemId>,
    update_systems: Vec<SystemId>,
    fixed_update_systems: Vec<SystemId>,
}

//==================================================================================================
// TweenFinished
//==================================================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EntityEvent)]
pub struct TweenFinished {
    pub entity: Entity,
    pub cycles: usize,
    pub played_forward: bool,
}

//==================================================================================================
// Tween
//==================================================================================================

#[derive(Component)]
#[component(on_add = tween_on_add::<T, P, M>)]
pub struct Tween<T, P, M>
where
    T: Component<Mutability = Mutable>,
    P: Tweenable + Send + Sync + 'static,
    M: TweenMarker + Send + Sync + 'static,
{
    pub set_property_fn: TweenPropertySet<T, P>,
    keys: Vec<(Duration, TweenKey<T, P, M>)>,
    seek_from_to_unchecked: Vec<(Duration, Duration)>,
    current: usize,
    last_update_elapsed_forward: Duration,
    timer: Timer,
    pub time_scale: f64,
    cycles: usize,
    tween_fns: Vec<(TweenFnAt, TweenFn<T, P, M>)>,
    pub target: TweenTarget,
    pub ease_fn: EaseFunction,
    pub ping_pong: bool,
}

fn tween_on_add<T, P, M>(mut world: DeferredWorld<'_>, _cx: HookContext)
where
    T: Component<Mutability = Mutable>,
    P: Tweenable + Send + Sync + 'static,
    M: TweenMarker + Send + Sync + 'static,
{
    let type_id = TypeId::of::<Tween<T, P, M>>();

    if world
        .resource::<TweenRegistry>()
        .system_map
        .contains_key(&type_id)
    {
        return;
    }

    world.commands().queue(move |world: &mut World| {
        if world
            .resource::<TweenRegistry>()
            .system_map
            .contains_key(&type_id)
        {
            return;
        }

        let system_id = world.register_system(update_tweens::<T, P, M>);

        let mut registry = world.resource_mut::<TweenRegistry>();
        registry.system_map.insert(type_id, system_id);
        match M::tween_schedule() {
            TweenSchedule::Update => {
                registry.update_systems.push(system_id);
            }
            TweenSchedule::FixedUpdate => {
                registry.fixed_update_systems.push(system_id);
            }
        }
    });
}

impl<T, M> Default for Tween<T, T, M>
where
    T: Component<Mutability = Mutable> + Tweenable + Send + Sync + 'static,
    M: TweenMarker + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::with_set(|t, v| *t = v)
    }
}

impl<T, M> Tween<T, T, M>
where
    T: Component<Mutability = Mutable> + Tweenable + Send + Sync + 'static,
    M: TweenMarker + Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self::default()
    }
}

impl<T, P, M> Tween<T, P, M>
where
    T: Component<Mutability = Mutable>,
    P: Tweenable + Send + Sync + 'static,
    M: TweenMarker + Send + Sync + 'static,
{
    pub fn with_set(set_property_fn: TweenPropertySet<T, P>) -> Self {
        Self {
            set_property_fn,
            keys: Default::default(),
            seek_from_to_unchecked: Default::default(),
            current: 0,
            last_update_elapsed_forward: Duration::ZERO,
            timer: Timer::new(Duration::ZERO, TimerMode::Once),
            time_scale: 1.0,
            cycles: 0,
            tween_fns: Default::default(),
            target: TweenTarget::This,
            ease_fn: EaseFunction::Linear,
            ping_pong: false,
        }
    }

    pub fn push(mut self, key: TweenKey<T, P, M>) -> Self {
        let start = self.timer.duration();

        if !key.duration.is_zero() {
            let new_duration = start + key.duration;
            self.timer.set_duration(new_duration);
        }

        self.keys.push((start, key));

        self
    }

    pub fn extend<I>(mut self, keys: I) -> Self
    where
        I: IntoIterator<Item = TweenKey<T, P, M>>,
    {
        let mut new_duration = self.timer.duration();

        for key in keys {
            let start = new_duration;
            new_duration += key.duration;
            self.keys.push((start, key));
        }

        if new_duration != self.timer.duration() {
            self.timer.set_duration(new_duration);
        }

        self
    }

    pub fn time_scale(mut self, time_scale: f64) -> Self {
        self.time_scale = time_scale;
        self
    }

    pub fn reverse(mut self) -> Self {
        self.time_scale = -self.time_scale;
        self
    }

    pub fn at<F>(mut self, at: TweenFnAt, tween_fn: F) -> Self
    where
        F: FnMut(TweenContext<'_, '_, '_, '_, T, P, M>) + Send + Sync + 'static,
    {
        let idx = self
            .tween_fns
            .partition_point(|(existing, _)| existing <= &at);
        let minimal_at: MinimalTweenFnAt = at.into();
        self.tween_fns
            .insert(idx, (at, minimal_at.finalize_impl(tween_fn)));

        self
    }

    pub fn write_message_at<Msg>(self, at: TweenFnAt, message: Msg) -> Self
    where
        Msg: Message + Clone,
    {
        self.at(at, move |cx| {
            cx.commands.write_message(message.clone());
        })
    }

    pub fn trigger_event_at<'a>(
        self,
        at: TweenFnAt,
        event: impl Event<Trigger<'a>: Default> + Clone,
    ) -> Self {
        self.at(at, move |cx| cx.commands.trigger(event.clone()))
    }

    pub fn target(mut self, target: TweenTarget) -> Self {
        self.target = target;
        self
    }

    pub fn ease_fn(mut self, ease_fn: EaseFunction) -> Self {
        self.ease_fn = ease_fn;
        self
    }

    pub fn repeating(mut self) -> Self {
        self.set_mode(TimerMode::Repeating);
        self
    }

    pub fn with_mode(mut self, mode: TimerMode) -> Self {
        self.set_mode(mode);
        self
    }

    pub fn ping_pong(mut self) -> Self {
        self.ping_pong = true;
        self
    }

    pub fn cycles(&self) -> usize {
        self.cycles
    }

    pub fn with_schedule_seek_elapsed(mut self, time: Duration) -> Self {
        self.schedule_seek_elapsed(time);
        self
    }

    pub fn with_schedule_seek_elapsed_secs(mut self, secs: f64) -> Self {
        self.schedule_seek_elapsed_secs(secs);
        self
    }

    pub fn plays_forward(&self) -> bool {
        let reversed = self.time_scale < 0.0;
        let ping_pong_reversed = self.ping_pong && !self.cycles.is_multiple_of(2);

        reversed == ping_pong_reversed
    }

    pub fn plays_in_reverse(&self) -> bool {
        !self.plays_forward()
    }

    pub fn is_finished(&self) -> bool {
        self.timer.is_finished()
    }

    pub fn schedule_seek_elapsed(&mut self, time: Duration) {
        let elapsed_forward = self.elapsed_as_if_forward();

        if elapsed_forward == time && self.seek_from_to_unchecked.is_empty() {
            return;
        }

        self.seek_from_to_unchecked.push((elapsed_forward, time));
    }

    pub fn schedule_seek_elapsed_secs(&mut self, secs: f64) {
        self.schedule_seek_elapsed(Duration::from_secs_f64(secs))
    }

    pub fn schedule_finish(&mut self) {
        self.schedule_seek_elapsed(self.duration());
    }

    pub fn schedule_reset(&mut self) {
        self.schedule_seek_elapsed(Duration::ZERO);
    }

    pub fn elapsed(&self) -> Duration {
        let fraction: f32 = self.fraction();

        if fraction == 0.0 {
            return Duration::ZERO;
        } else if fraction == 1.0 {
            return self.duration();
        }

        let elapsed = if self.ease_fn == EaseFunction::Linear {
            if self.plays_forward() {
                self.timer.elapsed()
            } else {
                self.timer.remaining()
            }
        } else {
            Duration::from_secs_f64(self.duration_secs() * fraction as f64)
        };

        const MIN_ELAPSED: Duration = Duration::from_micros(100); // 1e-4s

        elapsed.max(MIN_ELAPSED).min(self.duration())
    }

    fn elapsed_as_if_forward(&self) -> Duration {
        let elapsed = self.elapsed();

        if self.plays_forward() {
            elapsed
        } else {
            self.duration().saturating_sub(elapsed)
        }
    }

    pub fn elapsed_secs(&self) -> f64 {
        self.elapsed().as_secs_f64()
    }

    pub fn duration(&self) -> Duration {
        self.timer.duration()
    }

    pub fn duration_secs(&self) -> f64 {
        self.duration().as_secs_f64()
    }

    pub fn set_mode(&mut self, mode: TimerMode) {
        self.timer.set_mode(mode)
    }

    pub fn mode(&self) -> TimerMode {
        self.timer.mode()
    }

    pub fn pause(&mut self) {
        self.timer.pause()
    }

    pub fn unpause(&mut self) {
        self.timer.unpause()
    }

    pub fn is_paused(&self) -> bool {
        self.timer.is_paused()
    }

    pub fn fraction(&self) -> f32 {
        let plays_forward = self.plays_forward();

        let elapsed = self.timer.elapsed();
        if elapsed.is_zero() {
            return if plays_forward { 0.0 } else { 1.0 };
        } else if elapsed == self.duration() {
            return if plays_forward { 1.0 } else { 0.0 };
        }

        let fraction = if plays_forward {
            self.timer.fraction()
        } else {
            self.timer.fraction_remaining()
        };

        self.ease_fn.sample_clamped(fraction)
    }

    pub fn fraction_remaining(&self) -> f32 {
        1.0 - self.fraction()
    }

    pub fn remaining(&self) -> Duration {
        self.duration().saturating_sub(self.elapsed())
    }

    pub fn remaining_secs(&self) -> f64 {
        self.remaining().as_secs_f64()
    }

    fn update(
        &mut self,
        delta: Duration,
        targets: &mut Query<&mut T>,
        target_options: TweenTargetOptions,
        commands: &mut Commands,
    ) {
        // Fix state
        {
            let before_tick_forward = self.elapsed_as_if_forward();

            if self.last_update_elapsed_forward != before_tick_forward {
                self.wrapping_seek_from_to(
                    self.last_update_elapsed_forward,
                    before_tick_forward,
                    targets,
                    target_options,
                    commands,
                );
            }
        }

        // Seek
        for (from, to) in mem::take(&mut self.seek_from_to_unchecked) {
            self.timer.set_elapsed(to.min(self.duration()));
            self.wrapping_seek_from_to(from, to, targets, target_options, commands);
        }

        // Update
        if self.is_paused() {
            self.last_update_elapsed_forward = self.elapsed_as_if_forward();
            return;
        }

        let before_tick_forward = self.elapsed_as_if_forward();

        if self.time_scale.abs() == 1.0 {
            self.timer.tick(delta);
        } else {
            let scaled_delta = delta.mul_f64(self.time_scale.abs());
            self.timer.tick(scaled_delta);
        }
        let times_finished_this_tick: u32 = self.timer.times_finished_this_tick();

        if times_finished_this_tick == 0 && self.timer.is_finished() {
            self.last_update_elapsed_forward = self.elapsed_as_if_forward();
            return;
        }

        if times_finished_this_tick > 0 {
            self.seek_from_to(
                before_tick_forward,
                self.duration(),
                targets,
                target_options,
                commands,
            );
        }

        for _ in 0..times_finished_this_tick.saturating_sub(1) {
            self.seek_from_to(
                Duration::ZERO,
                self.duration(),
                targets,
                target_options,
                commands,
            );
        }

        let after_tick_forward = self.elapsed_as_if_forward();

        if times_finished_this_tick > 0 {
            self.seek_from_to(
                Duration::ZERO,
                after_tick_forward,
                targets,
                target_options,
                commands,
            );
        } else {
            if before_tick_forward <= after_tick_forward {
                self.seek_from_to(
                    before_tick_forward,
                    after_tick_forward,
                    targets,
                    target_options,
                    commands,
                );
            } else {
                self.time_scale = -self.time_scale;
                self.seek_from_to(
                    after_tick_forward,
                    before_tick_forward,
                    targets,
                    target_options,
                    commands,
                );
                self.time_scale = -self.time_scale;
            }
        }

        self.last_update_elapsed_forward = after_tick_forward;
    }

    fn wrapping_seek_from_to(
        &mut self,
        raw_from: Duration,
        raw_to: Duration,
        targets: &mut Query<&mut T>,
        target_options: TweenTargetOptions,
        commands: &mut Commands,
    ) {
        if self.keys.is_empty() {
            return;
        }

        let total_duration = self.duration();

        let from = raw_from.min(total_duration);
        let to = raw_to.min(total_duration);

        if from <= to {
            self.seek_from_to(from, to, targets, target_options, commands);
        } else {
            self.seek_from_to(from, self.duration(), targets, target_options, commands);
            self.seek_from_to(Duration::ZERO, to, targets, target_options, commands);
        }
    }

    fn seek_from_to(
        &mut self,
        raw_from: Duration,
        raw_to: Duration,
        targets: &mut Query<&mut T>,
        target_options: TweenTargetOptions,
        commands: &mut Commands,
    ) {
        debug_assert!(raw_to >= raw_from);

        let total_duration = self.duration();

        let from = raw_from.min(total_duration);
        let to = raw_to.min(total_duration);

        if to <= from {
            return;
        }

        let set_property_fn = self.set_property_fn;
        let tween_fraction = self.fraction();
        let plays_in_reverse = self.plays_in_reverse();
        let tween_target = self.target;
        let (from_with_dir, to_with_dir) = if plays_in_reverse {
            let from_reversed = total_duration.saturating_sub(to);
            let to_reversed = total_duration.saturating_sub(from);
            (from_reversed, to_reversed)
        } else {
            (from, to)
        };

        let start_current = self.current;
        let mut time = from;
        loop {
            let current_idx = if plays_in_reverse {
                self.keys.len().saturating_sub(self.current + 1)
            } else {
                self.current
            };

            let (before_keys, after_keys) = self.keys.split_at_mut(current_idx);
            let ((current_start, current), after_keys) = {
                let ((current_start, current), after_keys) = after_keys.split_first_mut().unwrap();

                if plays_in_reverse {
                    let reversed_current_start =
                        total_duration.saturating_sub(*current_start + current.duration);
                    ((reversed_current_start, current), after_keys)
                } else {
                    ((*current_start, current), after_keys)
                }
            };
            let previous_key = before_keys.last().map(|k| &k.1);
            let next_key = after_keys.first().map(|k| &k.1);

            let current_duration = current.duration;
            let current_end = current_start + current_duration;

            let key_from = time.saturating_sub(current_start).min(current_duration);
            let key_to = to.saturating_sub(current_start).min(current_duration);

            current.update_from_to(TweenKeyUpdateArgs {
                set_property_fn,
                targets,
                target_options,
                plays_in_reverse,
                tween_target,
                tween_duration: total_duration,
                tween_from_with_dir: from_with_dir,
                tween_to_with_dir: to_with_dir,
                tween_fraction,
                key_from,
                key_to,
                previous_key,
                next_key,
                commands,
            });

            if current_end <= to {
                time = current_end;
                if self.current + 1 < self.keys.len() {
                    self.current += 1;
                } else {
                    self.current = 0;
                }
            } else {
                time = to;
            }

            if start_current == self.current || (!current_duration.is_zero() && time >= to) {
                break;
            }
        }

        let target_entity = target_options.select(tween_target);
        let mut target = target_entity.and_then(|e| targets.get_mut(e).ok());
        let mut tick_tween_fn = |(_, tween_fn): &mut (TweenFnAt, TweenFn<T, P, M>)| {
            tween_fn(TweenContext {
                entity: target_options.this,
                parent: target_options.parent,
                target: target_entity.zip(target.as_mut()),
                plays_in_reverse,
                duration: total_duration,
                from: from_with_dir,
                to: to_with_dir,
                fraction: tween_fraction,
                commands,
                _marker0: PhantomData,
                _marker1: PhantomData,
            });
        };
        if plays_in_reverse {
            self.tween_fns.iter_mut().rev().for_each(&mut tick_tween_fn);
        } else {
            self.tween_fns.iter_mut().for_each(&mut tick_tween_fn);
        }

        if to == total_duration {
            let played_forward = self.plays_forward();
            self.cycles += 1;
            commands.trigger(TweenFinished {
                entity: target_options.this,
                cycles: self.cycles,
                played_forward,
            });
        }
    }
}
