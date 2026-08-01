use super::{
    Tween, TweenEase, TweenEaseKey, marker::TweenMarker, target::TweenTarget, tweenable::Tweenable,
};
use bevy_ecs::component::{Component, Mutable};
use bevy_math::curve::EaseFunction;
use bevy_time::TimerMode;
use std::time::Duration;

#[derive(Component, Debug, Default, Clone)]
pub struct TweenController {
    actions: Vec<ScheduleTweenAction>,
}

impl TweenController {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn discard_pending_schedules_read_by_any_tween(&mut self) {
        self.actions.retain(|a| !a.was_read_by_tween);
    }

    pub fn with_schedule_set_time_scale(mut self, time_scale: f64) -> Self {
        self.schedule_set_time_scale(time_scale);
        self
    }

    pub fn schedule_set_time_scale(&mut self, time_scale: f64) {
        self.actions
            .push(ScheduleTweenAction::new(TweenAction::TimeScale(time_scale)));
    }

    pub fn with_schedule_set_pause_every_nth_cycle(mut self, cycle: usize) -> Self {
        self.schedule_set_pause_every_nth_cycle(cycle);
        self
    }

    pub fn schedule_set_pause_every_nth_cycle(&mut self, cycle: usize) {
        self.actions
            .push(ScheduleTweenAction::new(TweenAction::PauseEveryNthCycle(
                cycle,
            )));
    }

    pub fn with_schedule_set_reverse(mut self) -> Self {
        self.schedule_set_reverse();
        self
    }

    pub fn schedule_set_reverse(&mut self) {
        self.actions
            .push(ScheduleTweenAction::new(TweenAction::Reverse));
    }

    pub fn with_schedule_set_target(mut self, target: TweenTarget) -> Self {
        self.schedule_set_target(target);
        self
    }

    pub fn schedule_set_target(&mut self, target: TweenTarget) {
        self.actions
            .push(ScheduleTweenAction::new(TweenAction::Target(target)));
    }

    pub fn with_schedule_set_mode(mut self, mode: TimerMode) -> Self {
        self.schedule_set_mode(mode);
        self
    }

    pub fn schedule_set_mode(&mut self, mode: TimerMode) {
        self.actions
            .push(ScheduleTweenAction::new(TweenAction::Mode(mode)));
    }

    pub fn with_schedule_set_ease(mut self, ease: TweenEase) -> Self {
        self.schedule_set_ease(ease);
        self
    }

    pub fn schedule_set_ease(&mut self, ease: TweenEase) {
        self.actions
            .push(ScheduleTweenAction::new(TweenAction::Ease(ease)));
    }

    pub fn with_schedule_set_ease_single(mut self, ease_fn: EaseFunction) -> Self {
        self.schedule_set_ease_single(ease_fn);
        self
    }

    pub fn schedule_set_ease_single(&mut self, ease_fn: EaseFunction) {
        self.schedule_set_ease(TweenEase::Single(ease_fn));
    }

    pub fn with_schedule_set_ease_timeline<I>(mut self, keys: I) -> Self
    where
        I: IntoIterator<Item = TweenEaseKey>,
    {
        self.schedule_set_ease_timeline(keys);
        self
    }

    pub fn schedule_set_ease_timeline<I>(&mut self, keys: I)
    where
        I: IntoIterator<Item = TweenEaseKey>,
    {
        self.schedule_set_ease(TweenEase::Timeline(keys.into_iter().collect()));
    }

    pub fn with_schedule_set_ping_pong(mut self, ping_pong: bool) -> Self {
        self.schedule_set_ping_pong(ping_pong);
        self
    }

    pub fn schedule_set_ping_pong(&mut self, ping_pong: bool) {
        self.actions
            .push(ScheduleTweenAction::new(TweenAction::PingPong(ping_pong)));
    }

    pub fn with_schedule_seek_elapsed(mut self, time: Duration) -> Self {
        self.schedule_seek_elapsed(time);
        self
    }

    pub fn schedule_seek_elapsed(&mut self, time: Duration) {
        self.actions
            .push(ScheduleTweenAction::new(TweenAction::Seek(
                ScheduleSeek::Elapsed(time),
            )));
    }

    pub fn with_schedule_seek_elapsed_secs(mut self, secs: f64) -> Self {
        self.schedule_seek_elapsed_secs(secs);
        self
    }

    pub fn schedule_seek_elapsed_secs(&mut self, secs: f64) {
        self.schedule_seek_elapsed(Duration::from_secs_f64(secs));
    }

    pub fn with_schedule_finish(mut self) -> Self {
        self.schedule_finish();
        self
    }

    pub fn schedule_finish(&mut self) {
        self.actions
            .push(ScheduleTweenAction::new(TweenAction::Seek(
                ScheduleSeek::Finish,
            )));
    }

    pub fn with_schedule_reset(mut self) -> Self {
        self.schedule_reset();
        self
    }

    pub fn schedule_reset(&mut self) {
        self.actions
            .push(ScheduleTweenAction::new(TweenAction::Seek(
                ScheduleSeek::Reset,
            )));
    }

    pub fn with_schedule_pause(mut self) -> Self {
        self.schedule_pause();
        self
    }

    pub fn schedule_pause(&mut self) {
        self.actions
            .push(ScheduleTweenAction::new(TweenAction::Timer(
                ScheduleTimer::Pause,
            )));
    }

    pub fn with_schedule_unpause(mut self) -> Self {
        self.schedule_unpause();
        self
    }

    pub fn schedule_unpause(&mut self) {
        self.actions
            .push(ScheduleTweenAction::new(TweenAction::Timer(
                ScheduleTimer::Unpause,
            )));
    }

    pub(super) fn apply_to<T, P, M>(&mut self, tween: &mut Tween<T, P, M>)
    where
        T: Component<Mutability = Mutable>,
        P: Tweenable + Send + Sync + 'static,
        M: TweenMarker + Send + Sync + 'static,
    {
        for action in &mut self.actions {
            action.was_read_by_tween = true;

            match &action.action {
                TweenAction::TimeScale(time_scale) => {
                    tween.time_scale = *time_scale;
                }

                TweenAction::PauseEveryNthCycle(cycle) => {
                    tween.pause_every_nth_cycle = *cycle;
                }

                TweenAction::Reverse => {
                    tween.time_scale = -tween.time_scale;
                }

                TweenAction::Target(target) => {
                    tween.target = *target;
                }

                TweenAction::Mode(mode) => {
                    tween.set_mode(*mode);
                }

                TweenAction::Ease(ease) => {
                    tween.ease = ease.clone();
                }

                TweenAction::PingPong(ping_pong) => {
                    tween.ping_pong = *ping_pong;
                }

                TweenAction::Seek(seek) => match seek {
                    ScheduleSeek::Elapsed(duration) => {
                        tween.schedule_seek_elapsed(*duration);
                    }
                    ScheduleSeek::Finish => {
                        tween.schedule_finish();
                    }
                    ScheduleSeek::Reset => {
                        tween.schedule_reset();
                    }
                },

                TweenAction::Timer(timer) => match timer {
                    ScheduleTimer::Pause => tween.pause(),
                    ScheduleTimer::Unpause => tween.unpause(),
                },
            }
        }
    }
}

#[derive(Debug, Clone)]
struct ScheduleTweenAction {
    was_read_by_tween: bool,
    action: TweenAction,
}

impl ScheduleTweenAction {
    pub fn new(action: TweenAction) -> Self {
        Self {
            was_read_by_tween: false,
            action,
        }
    }
}

#[derive(Debug, Clone)]
enum TweenAction {
    TimeScale(f64),
    PauseEveryNthCycle(usize),
    Reverse,
    Target(TweenTarget),
    Mode(TimerMode),
    Ease(TweenEase),
    PingPong(bool),
    Seek(ScheduleSeek),
    Timer(ScheduleTimer),
}

#[derive(Debug, Clone)]
enum ScheduleSeek {
    Elapsed(Duration),
    Finish,
    Reset,
}

#[derive(Debug, Clone)]
enum ScheduleTimer {
    Pause,
    Unpause,
}
