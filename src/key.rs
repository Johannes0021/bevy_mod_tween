use super::{
    checked_duration_from_secs_f64,
    function::{MinimalTweenFnAt, TweenFnAt, TweenKeyContext, TweenKeyFn},
    marker::TweenMarker,
    property::TweenPropertySet,
    target::{TweenKeyTarget, TweenTarget, TweenTargetOptions},
    tweenable::Tweenable,
};
use bevy_ecs::{
    component::{Component, Mutable},
    event::Event,
    message::Message,
    system::{Commands, Query},
};
use bevy_math::curve::{Curve, EaseFunction};
use std::{marker::PhantomData, time::Duration};

//==================================================================================================
// TweenKey
//==================================================================================================

pub struct TweenKey<T, P, M>
where
    T: Component<Mutability = Mutable>,
    P: Tweenable + Send + Sync + 'static,
    M: TweenMarker + Send + Sync + 'static,
{
    pub value: Option<P>,
    pub duration: Duration,
    tween_fns: Vec<(TweenFnAt, TweenKeyFn<T, P, M>)>,
    pub target: TweenKeyTarget,
    pub ease_fn: EaseFunction,
}

impl<T, P, M> Default for TweenKey<T, P, M>
where
    T: Component<Mutability = Mutable>,
    P: Tweenable + Send + Sync + 'static,
    M: TweenMarker + Send + Sync + 'static,
{
    fn default() -> Self {
        Self {
            value: None,
            duration: Duration::ZERO,
            tween_fns: Vec::default(),
            target: TweenKeyTarget::Derive,
            ease_fn: EaseFunction::Linear,
        }
    }
}

impl<T, P, M> TweenKey<T, P, M>
where
    T: Component<Mutability = Mutable>,
    P: Tweenable + Send + Sync + 'static,
    M: TweenMarker + Send + Sync + 'static,
{
    pub fn new(value: P) -> Self {
        Self::default().value(value)
    }

    pub fn delay(duration: Duration) -> Self {
        Self::default().duration(duration)
    }

    pub fn delay_secs(secs: f64) -> Self {
        Self::default().duration_secs(secs)
    }

    pub fn value(mut self, value: P) -> Self {
        self.value = Some(value);
        self
    }

    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    pub fn duration_secs(mut self, secs: f64) -> Self {
        self.duration = checked_duration_from_secs_f64(secs);
        self
    }

    pub fn at<F>(mut self, at: TweenFnAt, tween_fn: F) -> Self
    where
        F: FnMut(TweenKeyContext<'_, '_, '_, '_, T, P, M>) + Send + Sync + 'static,
    {
        let idx = self
            .tween_fns
            .partition_point(|(existing, _)| existing <= &at);
        let minimal_at: MinimalTweenFnAt = at.into();
        self.tween_fns
            .insert(idx, (at, minimal_at.finalize_key_impl(tween_fn)));

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
        self.target = TweenKeyTarget::Custom(target);
        self
    }

    pub fn ease_fn(mut self, ease_fn: EaseFunction) -> Self {
        self.ease_fn = ease_fn;
        self
    }

    pub(super) fn update_from_to(
        &mut self,
        TweenKeyUpdateArgs {
            set_property_fn,
            targets,
            target_options,
            plays_in_reverse,
            tween_is_start,
            tween_is_end,
            tween_target,
            tween_cycles,
            tween_duration,
            tween_from_with_dir,
            tween_to_with_dir,
            tween_fraction,
            key_is_start_with_dir,
            key_is_end_with_dir,
            key_from: raw_key_from,
            key_to: raw_key_to,
            previous_key,
            next_key,
            commands,
        }: TweenKeyUpdateArgs<T, P, M>,
    ) {
        debug_assert!(raw_key_to >= raw_key_from);

        let key_from = raw_key_from.min(self.duration);
        let key_to = raw_key_to.min(self.duration);

        if key_to < key_from {
            return;
        }

        let target_entity = match self.target {
            TweenKeyTarget::Derive => target_options.select(tween_target),
            TweenKeyTarget::Custom(custom_target) => target_options.select(custom_target),
        };
        let mut target = target_entity.and_then(|e| targets.get_mut(e).ok());

        let key_fraction = {
            let linear_key_fraction = if key_to == self.duration {
                1.0
            } else if key_to.is_zero() {
                0.0
            } else {
                key_to.as_secs_f32() / self.duration.as_secs_f32()
            };

            let linear_key_fraction_with_dir = if plays_in_reverse {
                1.0 - linear_key_fraction
            } else {
                linear_key_fraction
            };

            self.ease_fn.sample_clamped(linear_key_fraction_with_dir)
        };

        if let Some(target) = &mut target {
            match (self.value.as_ref(), next_key.and_then(|k| k.value.as_ref())) {
                (Some(value), Some(next_value)) => {
                    set_property_fn(&mut *target, value.tween(next_value, key_fraction));
                }
                (Some(value), None) => {
                    set_property_fn(&mut *target, value.clone());
                }
                (None, _) => {
                    let value = if plays_in_reverse {
                        next_key.and_then(|k| k.value.as_ref())
                    } else {
                        previous_key.and_then(|k| k.value.as_ref())
                    };

                    if let Some(value) = value {
                        set_property_fn(&mut *target, value.clone());
                    }
                }
            }
        }

        let (key_from_with_dir, key_to_with_dir) = if plays_in_reverse {
            let key_from_reversed = self.duration.saturating_sub(key_to);
            let key_to_reversed = self.duration.saturating_sub(key_from);
            (key_from_reversed, key_to_reversed)
        } else {
            (key_from, key_to)
        };
        let mut tick_tween_fn = |(_, tween_fn): &mut (TweenFnAt, TweenKeyFn<T, P, M>)| {
            tween_fn(TweenKeyContext {
                entity: target_options.this,
                parent: target_options.parent,
                target: target_entity.zip(target.as_mut()),
                plays_in_reverse,
                tween_is_start,
                tween_is_end,
                tween_cycles,
                tween_duration,
                tween_from: tween_from_with_dir,
                tween_to: tween_to_with_dir,
                tween_fraction,
                key_is_start: key_is_start_with_dir,
                key_is_end: key_is_end_with_dir,
                key_duration: self.duration,
                key_from: key_from_with_dir,
                key_to: key_to_with_dir,
                key_fraction,
                commands,
                _marker_p: PhantomData,
                _marker_m: PhantomData,
            });
        };
        if plays_in_reverse {
            self.tween_fns.iter_mut().rev().for_each(&mut tick_tween_fn);
        } else {
            self.tween_fns.iter_mut().for_each(&mut tick_tween_fn);
        }
    }
}

pub(super) struct TweenKeyUpdateArgs<'a, 'cw, 'cs, 'qw, 'qs, 'qt, T, P, M>
where
    T: Component<Mutability = Mutable>,
    P: Tweenable + Send + Sync + 'static,
    M: TweenMarker + Send + Sync + 'static,
{
    pub set_property_fn: TweenPropertySet<T, P>,
    pub targets: &'a mut Query<'qw, 'qs, &'qt mut T>,
    pub target_options: TweenTargetOptions,
    pub plays_in_reverse: bool,
    pub tween_is_start: bool,
    pub tween_is_end: bool,
    pub tween_target: TweenTarget,
    pub tween_cycles: usize,
    pub tween_duration: Duration,
    pub tween_from_with_dir: Duration,
    pub tween_to_with_dir: Duration,
    pub tween_fraction: f32,
    pub key_is_start_with_dir: bool,
    pub key_is_end_with_dir: bool,
    pub key_from: Duration,
    pub key_to: Duration,
    pub previous_key: Option<&'a TweenKey<T, P, M>>,
    pub next_key: Option<&'a TweenKey<T, P, M>>,
    pub commands: &'a mut Commands<'cw, 'cs>,
}
