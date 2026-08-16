use super::{Tween, checked_duration_from_secs_f64, marker::TweenMarker, tweenable::Tweenable};
use bevy_ecs::{
    change_detection::Mut,
    component::{Component, Mutable},
    entity::Entity,
    system::Commands,
};
use std::{cmp::Ordering, marker::PhantomData, num::NonZeroUsize, time::Duration};

//==================================================================================================
// TweenFn
//==================================================================================================

pub struct TweenContext<'a, 'cw, 'cs, 'tw, T, P, M>
where
    T: Component<Mutability = Mutable>,
    P: Tweenable,
    M: TweenMarker,
{
    pub entity: Entity,
    pub parent: Option<Entity>,
    pub target: Option<(Entity, &'a mut Mut<'tw, T>)>,
    pub plays_in_reverse: bool,
    pub is_start: bool,
    pub is_end: bool,
    pub cycles: usize,
    pub duration: Duration,
    pub from: Duration,
    pub to: Duration,
    pub fraction: f64,
    pub commands: &'a mut Commands<'cw, 'cs>,
    pub marker: &'a mut M,
    pub(super) _marker_p: PhantomData<P>,
}

impl<'a, 'cw, 'cs, 'tw, T, P, M> TweenContext<'a, 'cw, 'cs, 'tw, T, P, M>
where
    T: Component<Mutability = Mutable>,
    P: Tweenable,
    M: TweenMarker,
{
    pub fn delta(&self) -> Duration {
        self.to.saturating_sub(self.from)
    }

    pub fn commands_remove_tween_component(&mut self) {
        self.commands.entity(self.entity).remove::<Tween<T, P, M>>();
    }

    pub fn commands_despawn_tween_entiy(&mut self) {
        self.commands.entity(self.entity).despawn();
    }

    pub fn has_advanced(&self) -> bool {
        self.is_start || self.is_end || (self.from != self.to && !self.duration.is_zero())
    }
}

pub type TweenFn<T, P, M> = Box<dyn FnMut(TweenContext<'_, '_, '_, '_, T, P, M>) + Send + Sync>;

//==================================================================================================
// TweenKeyFn
//==================================================================================================

pub struct TweenKeyContext<'a, 'cw, 'cs, 'tw, T, P, M>
where
    T: Component<Mutability = Mutable>,
    P: Tweenable,
    M: TweenMarker,
{
    pub entity: Entity,
    pub parent: Option<Entity>,
    pub target: Option<(Entity, &'a mut Mut<'tw, T>)>,
    pub plays_in_reverse: bool,
    pub tween_is_start: bool,
    pub tween_is_end: bool,
    pub tween_cycles: usize,
    pub tween_duration: Duration,
    pub tween_from: Duration,
    pub tween_to: Duration,
    pub tween_fraction: f64,
    pub key_is_start: bool,
    pub key_is_end: bool,
    pub key_duration: Duration,
    pub key_from: Duration,
    pub key_to: Duration,
    pub key_fraction: f64,
    pub commands: &'a mut Commands<'cw, 'cs>,
    pub marker: &'a mut M,
    pub(super) _marker_p: PhantomData<P>,
}

impl<'a, 'cw, 'cs, 'tw, T, P, M> TweenKeyContext<'a, 'cw, 'cs, 'tw, T, P, M>
where
    T: Component<Mutability = Mutable>,
    P: Tweenable,
    M: TweenMarker,
{
    pub fn tween_delta(&self) -> Duration {
        self.tween_to.saturating_sub(self.tween_from)
    }

    pub fn key_delta(&self) -> Duration {
        self.key_to.saturating_sub(self.key_from)
    }

    pub fn commands_remove_tween_component(&mut self) {
        self.commands.entity(self.entity).remove::<Tween<T, P, M>>();
    }

    pub fn commands_despawn_tween_entiy(&mut self) {
        self.commands.entity(self.entity).despawn();
    }

    pub fn has_advanced(&self) -> bool {
        self.key_is_start
            || self.key_is_end
            || (self.key_from != self.key_to && !self.key_duration.is_zero())
    }
}

pub type TweenKeyFn<T, P, M> =
    Box<dyn FnMut(TweenKeyContext<'_, '_, '_, '_, T, P, M>) + Send + Sync>;

//==================================================================================================
// TweenFnAt
//==================================================================================================

// Start
// < EveryTick (Nth 1) | EveryNthTick(n)
// < Every(duration) | EverySecs(f64)
// < Duration(duration) | Secs(f64)
// < End
#[derive(Debug, Clone, Copy)]
pub enum TweenFnAt {
    Start,
    EveryTick,
    EveryNthTick(NonZeroUsize),
    Every(Duration),
    EverySecs(f64),
    Duration(Duration),
    Secs(f64),
    End,
}

impl From<TweenFnAt> for MinimalTweenFnAt {
    fn from(value: TweenFnAt) -> Self {
        match value {
            TweenFnAt::Start => Self::Start,
            TweenFnAt::EveryTick => Self::EveryNthTick(NonZeroUsize::MIN),
            TweenFnAt::EveryNthTick(n) => Self::EveryNthTick(n),
            TweenFnAt::Every(duration) => Self::Every(duration),
            TweenFnAt::EverySecs(secs) => Self::Every(checked_duration_from_secs_f64(secs)),
            TweenFnAt::Duration(duration) => Self::Duration(duration),
            TweenFnAt::Secs(secs) => Self::Duration(checked_duration_from_secs_f64(secs)),
            TweenFnAt::End => Self::End,
        }
    }
}

impl TweenFnAt {
    fn rank(&self) -> u8 {
        match self {
            Self::Start => 0,
            Self::EveryTick | Self::EveryNthTick(_) => 1,
            Self::Every(_) | Self::EverySecs(_) => 2,
            Self::Duration(_) | Self::Secs(_) => 3,
            Self::End => 4,
        }
    }

    fn normalized_tick(&self) -> NonZeroUsize {
        match self {
            Self::EveryTick => NonZeroUsize::MIN,
            Self::EveryNthTick(n) => *n,
            _ => unreachable!(),
        }
    }

    fn normalized_duration(&self) -> Duration {
        match self {
            Self::Every(duration) | Self::Duration(duration) => *duration,
            Self::EverySecs(secs) | Self::Secs(secs) => checked_duration_from_secs_f64(*secs),
            _ => unreachable!(),
        }
    }
}

impl PartialEq for TweenFnAt {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for TweenFnAt {}

impl PartialOrd for TweenFnAt {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TweenFnAt {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.rank().cmp(&other.rank()) {
            Ordering::Equal => {}
            ordering => return ordering,
        }

        match self {
            Self::Start | Self::End => Ordering::Equal,

            Self::EveryTick | Self::EveryNthTick(_) => {
                self.normalized_tick().cmp(&other.normalized_tick())
            }

            Self::Every(_) | Self::EverySecs(_) => {
                self.normalized_duration().cmp(&other.normalized_duration())
            }

            Self::Duration(_) | Self::Secs(_) => {
                self.normalized_duration().cmp(&other.normalized_duration())
            }
        }
    }
}

//==================================================================================================
// MinimalTweenFnAt
//==================================================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MinimalTweenFnAt {
    Start,
    EveryNthTick(NonZeroUsize),
    Every(Duration),
    Duration(Duration),
    End,
}

impl MinimalTweenFnAt {
    pub fn finalize_impl<T, P, M, F>(self, mut tween_fn: F) -> TweenFn<T, P, M>
    where
        T: Component<Mutability = Mutable>,
        P: Tweenable,
        M: TweenMarker,
        F: FnMut(TweenContext<'_, '_, '_, '_, T, P, M>) + Send + Sync + 'static,
    {
        match self {
            Self::Start => Box::new(move |cx| {
                if cx.is_start {
                    tween_fn(cx);
                }
            }),

            Self::End => Box::new(move |cx| {
                if cx.is_end {
                    tween_fn(cx);
                }
            }),

            Self::EveryNthTick(NonZeroUsize::MIN) => Box::new(tween_fn),

            Self::EveryNthTick(nth_tick) => {
                let mut tick: usize = 0;
                Box::new(move |cx| {
                    let is_start_or_end = cx.is_start || cx.is_end;

                    tick += 1;
                    if tick.is_multiple_of(nth_tick.into()) {
                        tick = 0;
                        tween_fn(cx);
                    }

                    if is_start_or_end {
                        tick = 0;
                    }
                })
            }

            Self::Every(every_duration) => {
                let mut time = Duration::ZERO;
                Box::new(move |mut cx| {
                    time += cx.delta();
                    while time >= every_duration {
                        time -= every_duration;

                        if let Some((e, target)) = cx.target.take() {
                            tween_fn(TweenContext {
                                entity: cx.entity,
                                parent: cx.parent,
                                target: Some((e, target)),
                                plays_in_reverse: cx.plays_in_reverse,
                                is_start: cx.is_start,
                                is_end: cx.is_end,
                                cycles: cx.cycles,
                                duration: cx.duration,
                                from: cx.from,
                                to: cx.to,
                                fraction: cx.fraction,
                                commands: cx.commands,
                                marker: cx.marker,
                                _marker_p: cx._marker_p,
                            });
                            cx.target = Some((e, target));
                        } else {
                            tween_fn(TweenContext {
                                entity: cx.entity,
                                parent: cx.parent,
                                target: None,
                                plays_in_reverse: cx.plays_in_reverse,
                                is_start: cx.is_start,
                                is_end: cx.is_end,
                                cycles: cx.cycles,
                                duration: cx.duration,
                                from: cx.from,
                                to: cx.to,
                                fraction: cx.fraction,
                                commands: cx.commands,
                                marker: cx.marker,
                                _marker_p: cx._marker_p,
                            });
                        }
                    }

                    if cx.is_start || cx.is_end {
                        time = Duration::ZERO;
                    }
                })
            }

            Self::Duration(duration) => Box::new(move |cx| {
                if cx.has_advanced() && (duration >= cx.from && duration <= cx.to) {
                    tween_fn(cx);
                }
            }),
        }
    }

    pub fn finalize_key_impl<T, P, M, F>(self, mut tween_fn: F) -> TweenKeyFn<T, P, M>
    where
        T: Component<Mutability = Mutable>,
        P: Tweenable,
        M: TweenMarker,
        F: FnMut(TweenKeyContext<'_, '_, '_, '_, T, P, M>) + Send + Sync + 'static,
    {
        match self {
            Self::Start => Box::new(move |cx| {
                if cx.key_is_start {
                    tween_fn(cx);
                }
            }),

            Self::End => Box::new(move |cx| {
                if cx.key_is_end {
                    tween_fn(cx);
                }
            }),

            Self::EveryNthTick(NonZeroUsize::MIN) => Box::new(tween_fn),

            Self::EveryNthTick(nth_tick) => {
                let mut tick: usize = 0;
                Box::new(move |cx| {
                    let key_is_start_or_end = cx.key_is_start || cx.key_is_end;

                    tick += 1;
                    if tick.is_multiple_of(nth_tick.into()) {
                        tick = 0;
                        tween_fn(cx);
                    }

                    if key_is_start_or_end {
                        tick = 0;
                    }
                })
            }

            Self::Every(every_duration) => {
                let mut time = Duration::ZERO;
                Box::new(move |mut cx| {
                    time += cx.key_delta();

                    while time >= every_duration {
                        time -= every_duration;

                        if let Some((e, target)) = cx.target.take() {
                            tween_fn(TweenKeyContext {
                                entity: cx.entity,
                                parent: cx.parent,
                                target: Some((e, target)),
                                plays_in_reverse: cx.plays_in_reverse,
                                tween_is_start: cx.tween_is_start,
                                tween_is_end: cx.tween_is_end,
                                tween_cycles: cx.tween_cycles,
                                tween_duration: cx.tween_duration,
                                tween_from: cx.tween_from,
                                tween_to: cx.tween_to,
                                tween_fraction: cx.tween_fraction,
                                key_is_start: cx.key_is_start,
                                key_is_end: cx.key_is_end,
                                key_duration: cx.key_duration,
                                key_from: cx.key_from,
                                key_to: cx.key_to,
                                key_fraction: cx.key_fraction,
                                commands: cx.commands,
                                marker: cx.marker,
                                _marker_p: cx._marker_p,
                            });
                            cx.target = Some((e, target));
                        } else {
                            tween_fn(TweenKeyContext {
                                entity: cx.entity,
                                parent: cx.parent,
                                target: None,
                                plays_in_reverse: cx.plays_in_reverse,
                                tween_is_start: cx.tween_is_start,
                                tween_is_end: cx.tween_is_end,
                                tween_cycles: cx.tween_cycles,
                                tween_duration: cx.tween_duration,
                                tween_from: cx.tween_from,
                                tween_to: cx.tween_to,
                                tween_fraction: cx.tween_fraction,
                                key_is_start: cx.key_is_start,
                                key_is_end: cx.key_is_end,
                                key_duration: cx.key_duration,
                                key_from: cx.key_from,
                                key_to: cx.key_to,
                                key_fraction: cx.key_fraction,
                                commands: cx.commands,
                                marker: cx.marker,
                                _marker_p: cx._marker_p,
                            });
                        }
                    }

                    if cx.key_is_start || cx.key_is_end {
                        time = Duration::ZERO;
                    }
                })
            }

            Self::Duration(duration) => Box::new(move |cx| {
                if cx.has_advanced() && (duration >= cx.key_from && duration <= cx.key_to) {
                    tween_fn(cx);
                }
            }),
        }
    }
}
