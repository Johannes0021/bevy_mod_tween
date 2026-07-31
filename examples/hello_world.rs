use bevy::prelude::*;
use bevy_mod_tween::prelude::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, TweenPlugin))
        .add_systems(Startup, setup)
        .run();
}

struct SpriteColorG;

fn setup(mut commands: Commands) {
    commands.spawn((Camera2d, Msaa::Off));

    commands.spawn((
        Sprite {
            color: Color::srgb(0.0, 1.0, 0.0),
            custom_size: Some(Vec2 { x: 21.0, y: 21.0 }),
            ..default()
        },
        // See also TweenController

        // target: Transform, property: Vec3, marker: TweenUpdate<()>,
        Tween::<Transform, Vec3, TweenUpdate>::with_set(|t, p| t.translation = p)
            .extend([
                TweenKey::new(Vec3::new(-120.0, -120.0, 0.0))
                    .duration_secs(1.0)
                    .at(TweenFnAt::Start, |_| info!("1"))
                    .at(TweenFnAt::End, |_| info!("2")),
                TweenKey::new(Vec3::new(120.0, 120.0, 0.0)),
                TweenKey::delay_secs(0.5)
                    .at(TweenFnAt::Start, |_| info!("3"))
                    .at(TweenFnAt::End, |_| info!("4")),
                TweenKey::new(Vec3::new(120.0, 120.0, 0.0)).duration_secs(2.0),
                TweenKey::new(Vec3::new(120.0, -120.0, 0.0)).duration_secs(1.0),
                TweenKey::new(Vec3::new(-120.0, 120.0, 0.0)).duration_secs(2.0),
                TweenKey::new(Vec3::new(-120.0, -120.0, 0.0)).duration_secs(1.0),
                TweenKey::delay_secs(2.0)
                    .at(TweenFnAt::Start, |_| info!("5"))
                    .at(TweenFnAt::End, |_| info!("6")),
            ])
            .at(TweenFnAt::Start, |_| {
                info!("start ----------------------------------------------")
            })
            .at(TweenFnAt::End, |_| {
                info!("end ------------------------------------------------")
            })
            .ease_fn(EaseFunction::QuinticIn)
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
        .ping_pong(),
    ));
}
