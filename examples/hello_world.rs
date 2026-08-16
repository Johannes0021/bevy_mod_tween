use bevy::prelude::*;
use bevy_mod_tween::prelude::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, TweenPlugin))
        .add_systems(Startup, setup)
        .run();
}

#[derive(Default)]
struct SpriteColorG;

fn setup(mut commands: Commands) {
    commands.spawn((Camera2d, Msaa::Off));

    commands.spawn((
        Sprite::sized(Vec2::splat(21.0)),
        // Pause all tweens on this entity:
        //TweenController::new().with_schedule_pause(),
        //
        // This example does not show all features. See the code for more details.
        //
        // target: Transform, property: Vec2, marker: TweenUpdate<()>,
        // There are also wrappers that implement Tweenable like:
        // TweenableFloor, TweenableRound, TweenableCeil, TweenStep, TweenStepAt
        Tween::<Transform, Vec2, TweenUpdate>::with_set(|t, p| t.translation = p.extend(0.0))
            .extend([
                TweenKey::new(Vec2::splat(-120.0))
                    .duration_secs(1.0)
                    .at(TweenFnAt::Start, |_| info!("1"))
                    .at(TweenFnAt::End, |_| info!("2")),
                TweenKey::new(Vec2::splat(120.0)),
                TweenKey::delay_secs(0.5)
                    .at(TweenFnAt::Start, |_| info!("3"))
                    .at(TweenFnAt::End, |_| info!("4")),
                TweenKey::default()
                    .at(TweenFnAt::Start, |_| info!("5"))
                    .at(TweenFnAt::Secs(0.0), |_| info!("6"))
                    .at(TweenFnAt::Secs(0.5), |_| info!("unreachable"))
                    .at(TweenFnAt::End, |_| info!("7")),
                TweenKey::new(Vec2::splat(120.0))
                    .duration_secs(2.0)
                    .ease_fn(EaseFunction::Elastic(21.0)),
                TweenKey::new(Vec2::new(120.0, -120.0)).duration_secs(1.0),
                TweenKey::new(Vec2::new(-120.0, 120.0)).duration_secs(2.0),
                TweenKey::new(Vec2::splat(-120.0)).duration_secs(1.0),
                TweenKey::delay_secs(2.0)
                    .at(TweenFnAt::Start, |_| info!("8"))
                    .at(TweenFnAt::End, |_| info!("9")),
            ])
            .at(TweenFnAt::Start, |_| {
                info!("start ----------------------------------------------")
            })
            .at(TweenFnAt::End, |_| {
                info!("end ------------------------------------------------")
            })
            .ease_single(EaseFunction::QuinticIn) // See also ease_timeline
            .repeating()
            .ping_pong(),
        // target: Sprite, property: f32, marker: TweenUpdate<SpriteColorG>,
        Tween::<Sprite, f32, TweenUpdate<SpriteColorG>>::with_set(|t, p| {
            t.color = Color::srgb(0.0, p, 0.0)
        })
        .extend([
            TweenKey::new(0.0).duration_secs(1.0),
            TweenKey::new(1.0),
            TweenKey::delay_secs(0.5),
            TweenKey::new(1.0).duration_secs(2.0),
            TweenKey::new(0.0).duration_secs(1.0),
            TweenKey::new(1.0).duration_secs(2.0),
            TweenKey::new(0.0).duration_secs(1.0),
            TweenKey::delay_secs(2.0),
        ])
        .repeating()
        .ping_pong()
        .pause_every_nth_cycle(3),
    ));
}
