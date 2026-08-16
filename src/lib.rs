use crate::{
    controller::{TweenController, TweenControllerCursor},
    ease::{TweenEase, TweenEaseKey, TweenEaseSample},
    function::{MinimalTweenFnAt, TweenContext, TweenFn, TweenFnAt},
    key::{TweenKey, TweenKeyUpdateArgs},
    marker::{TweenMarker, TweenSchedule},
    property::TweenPropertySet,
    target::{TweenTarget, TweenTargetOptions},
    tweenable::Tweenable,
};
use bevy_app::{App, FixedUpdate, MainScheduleOrder, Plugin, SpawnScene, Update};
use bevy_ecs::{
    change_detection::{Mut, Res},
    component::{Component, Mutable},
    entity::Entity,
    event::{EntityEvent, Event},
    hierarchy::ChildOf,
    lifecycle::HookContext,
    message::Message,
    query::{With, Without},
    resource::Resource,
    schedule::{IntoScheduleConfigs, Schedule, ScheduleLabel, SystemSet},
    system::{Commands, Query, SystemId},
    world::{DeferredWorld, World},
};
use bevy_log::warn;
use bevy_math::curve::EaseFunction;
use bevy_time::{Fixed, Time, Timer, TimerMode};
use std::{any::TypeId, collections::HashSet, marker::PhantomData, mem, time::Duration};

pub mod prelude {
    pub use super::{
        InitAddedTweens, Tween, TweenAutoPaused, TweenFinished, TweenPlugin, TweenSystems,
        controller::TweenController,
        ease::{TweenEase, TweenEaseKey, TweenEaseSample},
        function::{
            MinimalTweenFnAt, TweenContext, TweenFn, TweenFnAt, TweenKeyContext, TweenKeyFn,
        },
        key::TweenKey,
        marker::{TweenFixedUpdate, TweenMarker, TweenSchedule, TweenUpdate},
        property::TweenPropertySet,
        target::{TweenKeyTarget, TweenTarget},
        tweenable::{TweenStep, TweenStepAt, Tweenable},
    };
}

pub mod controller;
pub mod ease;
pub mod function;
pub mod key;
pub mod marker;
pub mod property;
pub mod target;
pub mod tweenable;

pub struct TweenPlugin;

#[derive(SystemSet, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TweenSystems;

#[derive(ScheduleLabel, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InitAddedTweens;

impl Plugin for TweenPlugin {
    fn build(&self, app: &mut App) {
        app.add_schedule(Schedule::new(InitAddedTweens));
        let mut main_schedule_order = app.world_mut().resource_mut::<MainScheduleOrder>();
        main_schedule_order.insert_after(SpawnScene, InitAddedTweens);

        app.init_resource::<TweenRegistry>()
            .add_systems(
                FixedUpdate,
                run_fixed_update_tween_systems.in_set(TweenSystems),
            )
            .add_systems(Update, run_update_tween_systems.in_set(TweenSystems))
            .add_systems(
                InitAddedTweens,
                (tween_controller_flush, run_init_added_tween_systems)
                    .chain()
                    .in_set(TweenSystems),
            );
    }
}

fn run_fixed_update_tween_systems(world: &mut World) {
    run_tween_systems(world, |registry| &mut registry.fixed_update_systems);
}

fn run_update_tween_systems(world: &mut World) {
    run_tween_systems(world, |registry| &mut registry.update_systems);
}

fn run_init_added_tween_systems(world: &mut World) {
    run_tween_systems(world, |registry| &mut registry.init_added_systems);
}

fn tween_controller_flush(tween_controllers: Query<&mut TweenController>) {
    for mut tween_controller in tween_controllers {
        tween_controller.flush();
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
        Err(e) => {
            warn!("Tween system failed and will be removed: {e:?}");
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
    tweens: Query<
        (
            Entity,
            &mut Tween<T, P, M>,
            Option<&ChildOf>,
            Option<&mut TweenController>,
        ),
        Without<InitTween<T, P, M>>,
    >,
    mut targets: Query<&mut T>,
    mut commands: Commands,
) where
    T: Component<Mutability = Mutable>,
    P: Tweenable,
    M: TweenMarker,
{
    let delta = time.delta();
    for (entity, mut tween, maybe_child_of, mut maybe_tween_controller) in tweens {
        update_tween(
            delta,
            entity,
            &mut tween,
            maybe_child_of,
            maybe_tween_controller.as_mut(),
            &mut targets,
            &mut commands,
        );
    }
}

#[allow(clippy::type_complexity)]
fn fixed_update_tweens<T, P, M>(
    time: Res<Time<Fixed>>,
    tweens: Query<
        (
            Entity,
            &mut Tween<T, P, M>,
            Option<&ChildOf>,
            Option<&mut TweenController>,
        ),
        Without<InitTween<T, P, M>>,
    >,
    mut targets: Query<&mut T>,
    mut commands: Commands,
) where
    T: Component<Mutability = Mutable>,
    P: Tweenable,
    M: TweenMarker,
{
    let delta = time.delta();
    for (entity, mut tween, maybe_child_of, mut maybe_tween_controller) in tweens {
        update_tween(
            delta,
            entity,
            &mut tween,
            maybe_child_of,
            maybe_tween_controller.as_mut(),
            &mut targets,
            &mut commands,
        );
    }
}

#[allow(clippy::type_complexity)]
fn init_added_tweens<T, P, M>(
    tweens: Query<
        (
            Entity,
            &mut Tween<T, P, M>,
            Option<&ChildOf>,
            Option<&mut TweenController>,
        ),
        With<InitTween<T, P, M>>,
    >,
    mut targets: Query<&mut T>,
    mut commands: Commands,
) where
    T: Component<Mutability = Mutable>,
    P: Tweenable,
    M: TweenMarker,
{
    let delta = Duration::ZERO;
    for (entity, mut tween, maybe_child_of, mut maybe_tween_controller) in tweens {
        tween.controller_cursor = TweenControllerCursor::default();

        update_tween(
            delta,
            entity,
            &mut tween,
            maybe_child_of,
            maybe_tween_controller.as_mut(),
            &mut targets,
            &mut commands,
        );

        commands.entity(entity).remove::<InitTween<T, P, M>>();
    }
}

fn update_tween<T, P, M>(
    delta: Duration,
    entity: Entity,
    tween: &mut Mut<Tween<T, P, M>>,
    maybe_child_of: Option<&ChildOf>,
    maybe_tween_controller: Option<&mut Mut<TweenController>>,
    targets: &mut Query<&mut T>,
    commands: &mut Commands,
) where
    T: Component<Mutability = Mutable>,
    P: Tweenable,
    M: TweenMarker,
{
    if let Some(tween_controller) = maybe_tween_controller {
        tween_controller.apply_to(tween);
    }

    let target_options = TweenTargetOptions {
        this: entity,
        parent: maybe_child_of.map(|c| c.parent()),
    };
    tween.update(delta, targets, target_options, commands);
}

//==================================================================================================
// TweenRegistry
//==================================================================================================

#[derive(Resource, Default)]
struct TweenRegistry {
    types: HashSet<TypeId>,
    init_added_systems: Vec<SystemId>,
    update_systems: Vec<SystemId>,
    fixed_update_systems: Vec<SystemId>,
}

//==================================================================================================
// TweenFinished
//==================================================================================================

#[derive(EntityEvent, Debug, Copy, PartialEq, Eq, Hash)]
pub struct TweenFinished<T> {
    pub entity: Entity,
    pub cycles: usize,
    pub played_forward: bool,
    _marker_tween_type: PhantomData<T>,
}

impl<T> Clone for TweenFinished<T> {
    fn clone(&self) -> Self {
        Self {
            entity: self.entity,
            cycles: self.cycles,
            played_forward: self.played_forward,
            _marker_tween_type: PhantomData,
        }
    }
}

//==================================================================================================
// TweenAutoPaused
//==================================================================================================

#[derive(EntityEvent, Debug, Copy, PartialEq, Eq, Hash)]
pub struct TweenAutoPaused<T> {
    pub entity: Entity,
    pub cycles: usize,
    _marker_tween_type: PhantomData<T>,
}

impl<T> Clone for TweenAutoPaused<T> {
    fn clone(&self) -> Self {
        Self {
            entity: self.entity,
            cycles: self.cycles,
            _marker_tween_type: PhantomData,
        }
    }
}

//==================================================================================================
// Tween
//==================================================================================================

#[derive(Component)]
#[component(on_add = tween_on_add::<T, P, M>, on_remove = tween_on_remove::<T, P, M>)]
#[require(InitTween::<T, P, M>)]
pub struct Tween<T, P, M>
where
    T: Component<Mutability = Mutable>,
    P: Tweenable,
    M: TweenMarker,
{
    pub(crate) controller_cursor: TweenControllerCursor,
    pub set_property_fn: TweenPropertySet<T, P>,
    keys: Vec<(Duration, TweenKey<T, P, M>)>,
    seek_from_to_unchecked: Vec<(Duration, Duration)>,
    current: usize,
    last_update_elapsed_forward: Duration,
    last_seek_from_to: Option<(Duration, Duration)>,
    timer: Timer,
    pub time_scale: f64,
    cycles: usize,
    /// `0` disables cycle pauses.
    pub pause_every_nth_cycle: usize,
    tween_fns: Vec<(TweenFnAt, TweenFn<T, P, M>)>,
    pub target: TweenTarget,
    pub ease: TweenEase,
    pub ping_pong: bool,
    pub marker: M,
}

fn tween_on_add<T, P, M>(mut world: DeferredWorld<'_>, _cx: HookContext)
where
    T: Component<Mutability = Mutable>,
    P: Tweenable,
    M: TweenMarker,
{
    let type_id = TypeId::of::<Tween<T, P, M>>();
    let mut registry = world.resource_mut::<TweenRegistry>();

    if registry.types.contains(&type_id) {
        return;
    }

    registry.types.insert(type_id);

    world.commands().queue(move |world: &mut World| {
        let tween_schedule = M::tween_schedule();

        let init_added_system_id = world.register_system(init_added_tweens::<T, P, M>);

        let update_system_id = match tween_schedule {
            TweenSchedule::Update => world.register_system(update_tweens::<T, P, M>),
            TweenSchedule::FixedUpdate => world.register_system(fixed_update_tweens::<T, P, M>),
        };

        let mut registry = world.resource_mut::<TweenRegistry>();

        registry.init_added_systems.push(init_added_system_id);

        match tween_schedule {
            TweenSchedule::Update => {
                registry.update_systems.push(update_system_id);
            }
            TweenSchedule::FixedUpdate => {
                registry.fixed_update_systems.push(update_system_id);
            }
        }
    });
}

fn tween_on_remove<T, P, M>(mut world: DeferredWorld<'_>, cx: HookContext)
where
    T: Component<Mutability = Mutable>,
    P: Tweenable,
    M: TweenMarker,
{
    if world.entity(cx.entity).contains::<InitTween<T, P, M>>() {
        world
            .commands()
            .entity(cx.entity)
            .remove::<InitTween<T, P, M>>();
    }
}

impl<T, M> Default for Tween<T, T, M>
where
    T: Component<Mutability = Mutable> + Tweenable,
    M: TweenMarker,
{
    fn default() -> Self {
        Self::with_set(|t, v| *t = v)
    }
}

impl<T, M> Tween<T, T, M>
where
    T: Component<Mutability = Mutable> + Tweenable,
    M: TweenMarker,
{
    pub fn new() -> Self {
        Self::default()
    }
}

impl<T, P, M> Tween<T, P, M>
where
    T: Component<Mutability = Mutable>,
    P: Tweenable,
    M: TweenMarker,
{
    pub fn with_set(set_property_fn: TweenPropertySet<T, P>) -> Self {
        Self {
            controller_cursor: TweenControllerCursor::default(),
            set_property_fn,
            keys: Vec::default(),
            seek_from_to_unchecked: Vec::default(),
            current: 0,
            last_update_elapsed_forward: Duration::ZERO,
            last_seek_from_to: None,
            timer: Timer::new(Duration::ZERO, TimerMode::Once),
            time_scale: 1.0,
            cycles: 0,
            pause_every_nth_cycle: 0,
            tween_fns: Vec::default(),
            target: TweenTarget::This,
            ease: TweenEase::default(),
            ping_pong: false,
            marker: M::default(),
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

    pub fn paused(mut self) -> Self {
        self.pause();
        self
    }

    pub fn pause_every_nth_cycle(mut self, cycle: usize) -> Self {
        self.pause_every_nth_cycle = cycle;
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

    pub fn ease(mut self, ease: TweenEase) -> Self {
        self.ease = ease;
        self
    }

    pub fn ease_single(self, ease_fn: EaseFunction) -> Self {
        self.ease(TweenEase::Single(ease_fn))
    }

    pub fn ease_timeline<I>(self, keys: I) -> Self
    where
        I: IntoIterator<Item = TweenEaseKey>,
    {
        self.ease(TweenEase::Timeline(keys.into_iter().collect()))
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

    pub fn marker(mut self, marker: M) -> Self {
        self.marker = marker;
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
        let from = self
            .seek_from_to_unchecked
            .last()
            .map(|(_, to)| *to)
            .unwrap_or_else(|| self.elapsed_as_if_forward());

        if from == time {
            return;
        }

        self.seek_from_to_unchecked.push((from, time));
    }

    pub fn schedule_seek_elapsed_secs(&mut self, secs: f64) {
        self.schedule_seek_elapsed(checked_duration_from_secs_f64(secs))
    }

    pub fn schedule_finish(&mut self) {
        self.schedule_seek_elapsed(self.duration());
    }

    pub fn schedule_reset(&mut self) {
        self.schedule_seek_elapsed(Duration::ZERO);
    }

    pub fn elapsed(&self) -> Duration {
        let plays_forward = self.plays_forward();
        let total_duration = self.duration();
        let timer_elapsed = self.timer.elapsed();

        if timer_elapsed == total_duration {
            if plays_forward {
                return total_duration;
            } else {
                return Duration::ZERO;
            }
        }

        if timer_elapsed.is_zero() {
            if plays_forward {
                return Duration::ZERO;
            } else {
                return total_duration;
            }
        }

        let linear_elapsed = if self.plays_forward() {
            timer_elapsed
        } else {
            self.timer.remaining()
        };

        let elapsed = if self.ease.is(EaseFunction::Linear, linear_elapsed) {
            linear_elapsed
        } else {
            checked_duration_from_secs_f64(self.duration_secs() * self.fraction())
        };

        elapsed.min(self.duration())
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

    pub fn fraction(&self) -> f64 {
        let plays_forward = self.plays_forward();
        let total_duration = self.duration();
        let timer_elapsed = self.timer.elapsed();

        if timer_elapsed == total_duration {
            return if plays_forward { 1.0 } else { 0.0 };
        }

        if timer_elapsed.is_zero() {
            return if plays_forward { 0.0 } else { 1.0 };
        }

        let linear_elapsed = if self.plays_forward() {
            timer_elapsed
        } else {
            self.timer.remaining()
        };

        self.ease.sample_clamped(TweenEaseSample {
            linear_elapsed,
            total_duration,
        })
    }

    pub fn fraction_remaining(&self) -> f64 {
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

        let scaled_delta = if self.time_scale.abs() == 1.0 {
            delta
        } else {
            delta.mul_f64(self.time_scale.abs())
        };
        self.timer.tick(scaled_delta);
        let times_finished_this_tick: u32 = self.timer.times_finished_this_tick();

        if times_finished_this_tick == 0 && self.timer.is_finished() {
            self.last_update_elapsed_forward = self.elapsed_as_if_forward();
            return;
        }

        let timer_elapsed = self.timer.elapsed();

        if times_finished_this_tick > 0 {
            self.timer.set_elapsed(self.duration());
            self.seek_from_to(
                before_tick_forward,
                self.duration(),
                targets,
                target_options,
                commands,
            );

            if self.handle_cycle_pause(target_options.this, commands) {
                self.last_update_elapsed_forward = self.elapsed_as_if_forward();
                return;
            }
        }

        for _ in 0..times_finished_this_tick.saturating_sub(1) {
            self.timer.set_elapsed(self.duration());
            self.seek_from_to(
                Duration::ZERO,
                self.duration(),
                targets,
                target_options,
                commands,
            );

            if self.handle_cycle_pause(target_options.this, commands) {
                self.last_update_elapsed_forward = self.elapsed_as_if_forward();
                return;
            }
        }

        if self.timer.elapsed() != timer_elapsed {
            self.timer.set_elapsed(timer_elapsed);
        }

        let after_tick_forward = self.elapsed_as_if_forward();

        if times_finished_this_tick > 0 {
            if after_tick_forward != self.duration() {
                self.seek_from_to(
                    Duration::ZERO,
                    after_tick_forward,
                    targets,
                    target_options,
                    commands,
                );
            }
        } else if before_tick_forward <= after_tick_forward {
            self.seek_from_to(
                before_tick_forward,
                after_tick_forward,
                targets,
                target_options,
                commands,
            );
        }
        //} else {
        //    self.time_scale = -self.time_scale;
        //    self.seek_from_to(
        //        after_tick_forward,
        //        before_tick_forward,
        //        targets,
        //        target_options,
        //        commands,
        //    );
        //    self.time_scale = -self.time_scale;
        //}

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
            let timer_elapsed = self.timer.elapsed();

            self.timer.set_elapsed(total_duration);
            self.seek_from_to(from, total_duration, targets, target_options, commands);

            if self.timer.elapsed() != timer_elapsed {
                self.timer.set_elapsed(timer_elapsed);
            }

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

        if self.keys.is_empty() {
            return;
        }

        let total_duration = self.duration();

        let from = raw_from.min(total_duration);
        let to = raw_to.min(total_duration);

        if to < from {
            return;
        }

        let can_be_start_or_end = if let Some((last_from, last_to)) = self.last_seek_from_to {
            last_from != last_to || from != last_to || total_duration.is_zero()
        } else {
            true
        };

        let set_property_fn = self.set_property_fn;
        let tween_fraction = self.fraction();
        let plays_in_reverse = self.plays_in_reverse();

        let is_start = can_be_start_or_end && from.is_zero();
        let is_end = self.timer.elapsed() == total_duration;
        let (is_start_with_dir, is_end_with_dir) = if plays_in_reverse {
            (is_end, is_start)
        } else {
            (is_start, is_end)
        };

        let tween_target = self.target;
        let (from_with_dir, to_with_dir) = if plays_in_reverse {
            let from_reversed = total_duration.saturating_sub(to);
            let to_reversed = total_duration.saturating_sub(from);
            (from_reversed, to_reversed)
        } else {
            (from, to)
        };

        let keys_len = self.keys.len();
        let start_current = self.current;
        let mut already_seeked_to = from;
        loop {
            let current_with_dir = if plays_in_reverse {
                keys_len.saturating_sub(self.current + 1)
            } else {
                self.current
            };

            let (before_keys, after_keys) = self.keys.split_at_mut(current_with_dir);
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

            let key_from = already_seeked_to
                .saturating_sub(current_start)
                .min(current_duration);
            let key_to = to.saturating_sub(current_start).min(current_duration);

            let key_is_start = if self.current == 0 {
                is_start
            } else {
                can_be_start_or_end && key_from.is_zero()
            };
            let key_is_end = if self.current + 1 == keys_len {
                is_end
            } else {
                can_be_start_or_end && current_end <= to
            };
            let (key_is_start_with_dir, key_is_end_with_dir) = if plays_in_reverse {
                (key_is_end, key_is_start)
            } else {
                (key_is_start, key_is_end)
            };

            current.update_from_to(TweenKeyUpdateArgs {
                set_property_fn,
                targets,
                target_options,
                plays_in_reverse,
                tween_is_start: is_start_with_dir,
                tween_is_end: is_end_with_dir,
                tween_target,
                tween_cycles: self.cycles,
                tween_duration: total_duration,
                tween_from_with_dir: from_with_dir,
                tween_to_with_dir: to_with_dir,
                tween_fraction,
                key_is_start_with_dir,
                key_is_end_with_dir,
                key_from,
                key_to,
                previous_key,
                next_key,
                commands,
                marker: &mut self.marker,
            });

            if key_is_end {
                already_seeked_to = current_end;
                if self.current + 1 < keys_len {
                    self.current += 1;
                } else {
                    self.current = 0;
                }
            } else {
                already_seeked_to = to;
            }

            if start_current == self.current
                || (!current_duration.is_zero() && already_seeked_to >= to)
            {
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
                is_start: is_start_with_dir,
                is_end: is_end_with_dir,
                cycles: self.cycles,
                duration: total_duration,
                from: from_with_dir,
                to: to_with_dir,
                fraction: tween_fraction,
                commands,
                marker: &mut self.marker,
                _marker_p: PhantomData,
            });
        };
        if plays_in_reverse {
            self.tween_fns.iter_mut().rev().for_each(&mut tick_tween_fn);
        } else {
            self.tween_fns.iter_mut().for_each(&mut tick_tween_fn);
        }

        if is_end {
            let played_forward = self.plays_forward();
            self.cycles += 1;
            commands.trigger(TweenFinished::<Self> {
                entity: target_options.this,
                cycles: self.cycles,
                played_forward,
                _marker_tween_type: PhantomData,
            });
        }

        self.last_seek_from_to = Some((from, to))
    }

    fn handle_cycle_pause(&mut self, entity: Entity, commands: &mut Commands) -> bool {
        debug_assert_eq!(self.timer.elapsed(), self.duration());

        let pause = self.pause_every_nth_cycle != 0
            && self.cycles.is_multiple_of(self.pause_every_nth_cycle);

        if pause {
            self.pause();
            commands.trigger(TweenAutoPaused::<Self> {
                entity,
                cycles: self.cycles,
                _marker_tween_type: PhantomData,
            });
        }

        pause
    }
}

//==================================================================================================
// InitTween
//==================================================================================================

#[derive(Component)]
struct InitTween<T, P, M> {
    _marker_t: PhantomData<T>,
    _marker_p: PhantomData<P>,
    _marker_m: PhantomData<M>,
}

impl<T, P, M> Default for InitTween<T, P, M> {
    fn default() -> Self {
        Self {
            _marker_t: PhantomData,
            _marker_p: PhantomData,
            _marker_m: PhantomData,
        }
    }
}

//==================================================================================================
// helper functions
//==================================================================================================

pub(crate) fn checked_duration_from_secs_f64(secs: f64) -> Duration {
    debug_assert!(secs >= 0.0, "Duration must be non-negative");
    Duration::from_secs_f64(secs.max(0.0))
}
