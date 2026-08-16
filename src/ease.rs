use super::checked_duration_from_secs_f64;
use bevy_math::curve::{Curve, EaseFunction};
use std::time::Duration;

//==================================================================================================
// TweenEase
//==================================================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum TweenEase {
    Single(EaseFunction),
    Timeline(Vec<TweenEaseKey>),
}

impl Default for TweenEase {
    fn default() -> Self {
        Self::Single(EaseFunction::Linear)
    }
}

impl TweenEase {
    pub fn get(&self, time: Duration) -> EaseFunction {
        match self {
            Self::Single(ease) => *ease,

            Self::Timeline(keys) => {
                let mut elapsed = Duration::ZERO;

                for key in keys {
                    elapsed += key.duration;

                    if time < elapsed {
                        return key.ease_fn;
                    }
                }

                keys.last()
                    .map(|key| key.ease_fn)
                    .unwrap_or(EaseFunction::Linear)
            }
        }
    }

    pub fn is(&self, ease: EaseFunction, time: Duration) -> bool {
        ease == self.get(time)
    }

    pub fn sample_clamped(
        &self,
        TweenEaseSample {
            linear_elapsed,
            total_duration,
        }: TweenEaseSample,
    ) -> f64 {
        if linear_elapsed.is_zero() {
            return 0.0;
        }

        if linear_elapsed >= total_duration {
            return 1.0;
        }

        debug_assert!(!total_duration.is_zero());

        match self {
            Self::Single(ease) => {
                let t = linear_elapsed.as_secs_f64() / total_duration.as_secs_f64();
                ease.sample_clamped(t as f32) as f64
            }

            Self::Timeline(keys) => {
                let mut elapsed = Duration::ZERO;

                for key in keys {
                    let segment_start = elapsed;
                    let segment_end = elapsed + key.duration;

                    if linear_elapsed < segment_end {
                        debug_assert!(!key.duration.is_zero());

                        let segment_t = linear_elapsed.saturating_sub(segment_start).as_secs_f64()
                            / key.duration.as_secs_f64();

                        let eased_t: f32 =
                            key.ease_fn.sample_clamped(segment_t.clamp(0.0, 1.0) as f32);

                        let start = segment_start.as_secs_f64() / total_duration.as_secs_f64();

                        let end = segment_end.as_secs_f64() / total_duration.as_secs_f64();

                        return start + ((end - start) * eased_t as f64);
                    }

                    elapsed = segment_end;
                }

                linear_elapsed.as_secs_f64() / total_duration.as_secs_f64()
            }
        }
    }
}

//==================================================================================================
// TweenEaseSample
//==================================================================================================

pub struct TweenEaseSample {
    pub linear_elapsed: Duration,
    pub total_duration: Duration,
}

//==================================================================================================
// TweenEaseKey
//==================================================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TweenEaseKey {
    pub ease_fn: EaseFunction,
    pub duration: Duration,
}

impl TweenEaseKey {
    pub fn duration(ease_fn: EaseFunction, duration: Duration) -> Self {
        Self { ease_fn, duration }
    }

    pub fn duration_secs(ease_fn: EaseFunction, secs: f64) -> Self {
        Self::duration(ease_fn, checked_duration_from_secs_f64(secs))
    }
}
