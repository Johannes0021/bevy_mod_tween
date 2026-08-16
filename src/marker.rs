use std::marker::PhantomData;

//==================================================================================================
// TweenMarker
//==================================================================================================

pub trait TweenMarker: Send + Sync + Default + 'static {
    fn tween_schedule() -> TweenSchedule;
}

// TweenUpdate -------------------------------------------------------------------------------------

#[derive(Default)]
pub struct TweenUpdate<T = ()> {
    _marker_t: PhantomData<T>,
}

impl<T> TweenMarker for TweenUpdate<T>
where
    T: Send + Sync + Default + 'static,
{
    fn tween_schedule() -> TweenSchedule {
        TweenSchedule::Update
    }
}

// TweenFixedUpdate --------------------------------------------------------------------------------

#[derive(Default)]
pub struct TweenFixedUpdate<T = ()> {
    _marker_t: PhantomData<T>,
}

impl<T> TweenMarker for TweenFixedUpdate<T>
where
    T: Send + Sync + Default + 'static,
{
    fn tween_schedule() -> TweenSchedule {
        TweenSchedule::FixedUpdate
    }
}

//==================================================================================================
// TweenSchedule
//==================================================================================================

pub enum TweenSchedule {
    Update,
    FixedUpdate,
}
