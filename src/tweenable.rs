use bevy_asset::{Asset, Handle};
use bevy_color::{Color, Mix};
use bevy_derive::{Deref, DerefMut};
use bevy_math::{DVec2, DVec3, DVec4, FloatExt, IVec2, IVec3, IVec4, Rect, Vec2, Vec3, Vec4};
use bevy_transform::components::Transform;
use bevy_ui::Val;

pub trait Tweenable: Send + Sync + Clone + 'static {
    fn tween(&self, other: &Self, t: f64) -> Self;
}

impl Tweenable for () {
    fn tween(&self, _other: &Self, _t: f64) -> Self {}
}

impl Tweenable for bool {
    fn tween(&self, other: &Self, t: f64) -> Self {
        if t < 0.5 { *self } else { *other }
    }
}

impl<T> Tweenable for Option<T>
where
    T: Tweenable,
{
    fn tween(&self, other: &Self, t: f64) -> Self {
        match (self, other) {
            (Some(a), Some(b)) => Some(a.tween(b, t)),
            (this, _) if t < 0.5 => this.clone(),
            (_, other) => other.clone(),
        }
    }
}

impl Tweenable for f32 {
    fn tween(&self, other: &Self, t: f64) -> Self {
        self.lerp(*other, t as f32)
    }
}

impl Tweenable for f64 {
    fn tween(&self, other: &Self, t: f64) -> Self {
        self.lerp(*other, t)
    }
}

impl Tweenable for Vec2 {
    fn tween(&self, other: &Self, t: f64) -> Self {
        self.lerp(*other, t as f32)
    }
}

impl Tweenable for Vec3 {
    fn tween(&self, other: &Self, t: f64) -> Self {
        self.lerp(*other, t as f32)
    }
}

impl Tweenable for Vec4 {
    fn tween(&self, other: &Self, t: f64) -> Self {
        self.lerp(*other, t as f32)
    }
}

impl Tweenable for DVec2 {
    fn tween(&self, other: &Self, t: f64) -> Self {
        self.lerp(*other, t)
    }
}

impl Tweenable for DVec3 {
    fn tween(&self, other: &Self, t: f64) -> Self {
        self.lerp(*other, t)
    }
}

impl Tweenable for DVec4 {
    fn tween(&self, other: &Self, t: f64) -> Self {
        self.lerp(*other, t)
    }
}

impl Tweenable for i8 {
    fn tween(&self, other: &Self, t: f64) -> Self {
        ((*self as f64).lerp(*other as f64, t)).round() as Self
    }
}

impl Tweenable for i16 {
    fn tween(&self, other: &Self, t: f64) -> Self {
        ((*self as f64).lerp(*other as f64, t)).round() as Self
    }
}

impl Tweenable for i32 {
    fn tween(&self, other: &Self, t: f64) -> Self {
        ((*self as f64).lerp(*other as f64, t)).round() as Self
    }
}

impl Tweenable for i64 {
    fn tween(&self, other: &Self, t: f64) -> Self {
        ((*self as f64).lerp(*other as f64, t)).round() as Self
    }
}

impl Tweenable for u8 {
    fn tween(&self, other: &Self, t: f64) -> Self {
        ((*self as f64).lerp(*other as f64, t)).round() as Self
    }
}

impl Tweenable for u16 {
    fn tween(&self, other: &Self, t: f64) -> Self {
        ((*self as f64).lerp(*other as f64, t)).round() as Self
    }
}

impl Tweenable for u32 {
    fn tween(&self, other: &Self, t: f64) -> Self {
        ((*self as f64).lerp(*other as f64, t)).round() as Self
    }
}

impl Tweenable for u64 {
    fn tween(&self, other: &Self, t: f64) -> Self {
        ((*self as f64).lerp(*other as f64, t)).round() as Self
    }
}

impl Tweenable for IVec2 {
    fn tween(&self, other: &Self, t: f64) -> Self {
        (self.as_dvec2().lerp(other.as_dvec2(), t))
            .round()
            .as_ivec2()
    }
}

impl Tweenable for IVec3 {
    fn tween(&self, other: &Self, t: f64) -> Self {
        (self.as_dvec3().lerp(other.as_dvec3(), t))
            .round()
            .as_ivec3()
    }
}

impl Tweenable for IVec4 {
    fn tween(&self, other: &Self, t: f64) -> Self {
        (self.as_dvec4().lerp(other.as_dvec4(), t))
            .round()
            .as_ivec4()
    }
}

impl Tweenable for Transform {
    fn tween(&self, other: &Self, t: f64) -> Self {
        Self {
            translation: self.translation.lerp(other.translation, t as f32),
            rotation: self.rotation.slerp(other.rotation, t as f32),
            scale: self.scale.lerp(other.scale, t as f32),
        }
    }
}

impl Tweenable for Color {
    fn tween(&self, other: &Self, t: f64) -> Self {
        self.mix(other, t as f32)
    }
}

impl Tweenable for Rect {
    fn tween(&self, other: &Self, t: f64) -> Self {
        Self {
            min: self.min.lerp(other.min, t as f32),
            max: self.max.lerp(other.max, t as f32),
        }
    }
}

impl Tweenable for Val {
    fn tween(&self, other: &Self, t: f64) -> Self {
        match (self, other) {
            (Val::Px(a), Val::Px(b)) => Val::Px(a.lerp(*b, t as f32)),
            (Val::Percent(a), Val::Percent(b)) => Val::Percent(a.lerp(*b, t as f32)),
            (Val::Vw(a), Val::Vw(b)) => Val::Vw(a.lerp(*b, t as f32)),
            (Val::Vh(a), Val::Vh(b)) => Val::Vh(a.lerp(*b, t as f32)),
            (Val::VMin(a), Val::VMin(b)) => Val::VMin(a.lerp(*b, t as f32)),
            (Val::VMax(a), Val::VMax(b)) => Val::VMax(a.lerp(*b, t as f32)),
            (this, _) if t < 0.5 => *this,
            (_, other) => *other,
        }
    }
}

impl<T> Tweenable for Handle<T>
where
    T: Asset,
{
    fn tween(&self, other: &Self, t: f64) -> Self {
        if t < 0.5 { self.clone() } else { other.clone() }
    }
}

//==================================================================================================
// TweenStep
//==================================================================================================

#[derive(Clone, Deref, DerefMut)]
pub struct TweenStep<T>(pub T)
where
    T: Clone;

impl<T> Tweenable for TweenStep<T>
where
    T: Send + Sync + Clone + 'static,
{
    fn tween(&self, other: &Self, t: f64) -> Self {
        if t < 0.5 { self.clone() } else { other.clone() }
    }
}

//==================================================================================================
// TweenStepAt
//==================================================================================================

#[derive(Clone, Deref, DerefMut)]
pub struct TweenStepAt<T> {
    #[deref]
    pub value: T,
    pub at: f64,
}

impl<T> Tweenable for TweenStepAt<T>
where
    T: Send + Sync + Clone + 'static,
{
    fn tween(&self, other: &Self, t: f64) -> Self {
        if t < self.at {
            self.clone()
        } else {
            other.clone()
        }
    }
}
