use super::{Tween, marker::TweenMarker, target::TweenTarget, tweenable::Tweenable};
use bevy_ecs::component::{Component, Mutable};
use bevy_math::curve::EaseFunction;
use bevy_time::TimerMode;
use std::time::Duration;

#[derive(Debug, Default, Clone, Component)]
pub struct TweenController {
    pub(super) read_by_tween: bool,
    actions: Vec<ScheduleTweenAction>,
}

impl TweenController {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn discard_pending_schedules(&mut self) {
        *self = Self::default();
    }

    pub fn with_schedule_set_time_scale(mut self, time_scale: f64) -> Self {
        self.schedule_set_time_scale(time_scale);
        self
    }

    pub fn schedule_set_time_scale(&mut self, time_scale: f64) {
        self.actions
            .push(ScheduleTweenAction::TimeScale(time_scale));
    }

    pub fn with_schedule_set_reverse(mut self) -> Self {
        self.schedule_set_reverse();
        self
    }

    pub fn schedule_set_reverse(&mut self) {
        self.actions.push(ScheduleTweenAction::Reverse);
    }

    pub fn with_schedule_set_target(mut self, target: TweenTarget) -> Self {
        self.schedule_set_target(target);
        self
    }

    pub fn schedule_set_target(&mut self, target: TweenTarget) {
        self.actions.push(ScheduleTweenAction::Target(target));
    }

    pub fn with_schedule_set_mode(mut self, mode: TimerMode) -> Self {
        self.schedule_set_mode(mode);
        self
    }

    pub fn schedule_set_mode(&mut self, mode: TimerMode) {
        self.actions.push(ScheduleTweenAction::Mode(mode));
    }

    pub fn with_schedule_set_ease_fn(mut self, ease_fn: EaseFunction) -> Self {
        self.schedule_set_ease_fn(ease_fn);
        self
    }

    pub fn schedule_set_ease_fn(&mut self, ease_fn: EaseFunction) {
        self.actions
            .push(ScheduleTweenAction::EaseFunction(ease_fn));
    }

    pub fn with_schedule_set_ping_pong(mut self, ping_pong: bool) -> Self {
        self.schedule_set_ping_pong(ping_pong);
        self
    }

    pub fn schedule_set_ping_pong(&mut self, ping_pong: bool) {
        self.actions.push(ScheduleTweenAction::PingPong(ping_pong));
    }

    pub fn with_schedule_seek_elapsed(mut self, time: Duration) -> Self {
        self.schedule_seek_elapsed(time);
        self
    }

    pub fn schedule_seek_elapsed(&mut self, time: Duration) {
        self.actions
            .push(ScheduleTweenAction::Seek(ScheduleSeek::Elapsed(time)));
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
            .push(ScheduleTweenAction::Seek(ScheduleSeek::Finish));
    }

    pub fn with_schedule_reset(mut self) -> Self {
        self.schedule_reset();
        self
    }

    pub fn schedule_reset(&mut self) {
        self.actions
            .push(ScheduleTweenAction::Seek(ScheduleSeek::Reset));
    }

    pub fn with_schedule_pause(mut self) -> Self {
        self.schedule_pause();
        self
    }

    pub fn schedule_pause(&mut self) {
        self.actions
            .push(ScheduleTweenAction::Timer(ScheduleTimer::Pause));
    }

    pub fn with_schedule_unpause(mut self) -> Self {
        self.schedule_unpause();
        self
    }

    pub fn schedule_unpause(&mut self) {
        self.actions
            .push(ScheduleTweenAction::Timer(ScheduleTimer::Unpause));
    }

    pub(super) fn apply_to<T, P, M>(&mut self, tween: &mut Tween<T, P, M>)
    where
        T: Component<Mutability = Mutable>,
        P: Tweenable + Send + Sync + 'static,
        M: TweenMarker + Send + Sync + 'static,
    {
        self.read_by_tween = true;

        for action in &self.actions {
            match action {
                ScheduleTweenAction::TimeScale(time_scale) => {
                    tween.time_scale = *time_scale;
                }

                ScheduleTweenAction::Reverse => {
                    tween.time_scale = -tween.time_scale;
                }

                ScheduleTweenAction::Target(target) => {
                    tween.target = *target;
                }

                ScheduleTweenAction::Mode(mode) => {
                    tween.set_mode(*mode);
                }

                ScheduleTweenAction::EaseFunction(ease_fn) => {
                    tween.ease_fn = *ease_fn;
                }

                ScheduleTweenAction::PingPong(ping_pong) => {
                    tween.ping_pong = *ping_pong;
                }

                ScheduleTweenAction::Seek(seek) => match seek {
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

                ScheduleTweenAction::Timer(timer) => match timer {
                    ScheduleTimer::Pause => tween.pause(),
                    ScheduleTimer::Unpause => tween.unpause(),
                },
            }
        }
    }
}

#[derive(Debug, Clone)]
enum ScheduleTweenAction {
    TimeScale(f64),
    Reverse,
    Target(TweenTarget),
    Mode(TimerMode),
    EaseFunction(EaseFunction),
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
