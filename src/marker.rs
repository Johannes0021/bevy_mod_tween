use std::marker::PhantomData;

//==================================================================================================
// TweenMarker
//==================================================================================================

pub trait TweenMarker: Send + Sync + 'static {
    fn tween_schedule() -> TweenSchedule;
}

// TweenUpdate -------------------------------------------------------------------------------------

pub struct TweenUpdate<T = ()> {
    _marker: PhantomData<T>,
}

impl<T> TweenMarker for TweenUpdate<T>
where
    T: Send + Sync + 'static,
{
    fn tween_schedule() -> TweenSchedule {
        TweenSchedule::Update
    }
}

// TweenFixedUpdate --------------------------------------------------------------------------------

pub struct TweenFixedUpdate<T = ()> {
    _marker: PhantomData<T>,
}

impl<T> TweenMarker for TweenFixedUpdate<T>
where
    T: Send + Sync + 'static,
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
