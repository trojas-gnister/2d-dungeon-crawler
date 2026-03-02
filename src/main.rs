use avian2d::prelude::*;
use bevy::{prelude::*, ecs::error::error};
use bevy_tnua::prelude::*;
use bevy_tnua_avian2d::prelude::*;
use bevy_trauma_shake::prelude::*;

use player::PlayerControlScheme;

mod animation;
mod combat;
mod health;
mod input;
mod level;
mod enemy;
mod player;
pub mod screens;

#[cfg(feature = "dev")]
use bevy_inspector_egui::quick::WorldInspectorPlugin;

fn main() {
    let mut app = App::new();
    app.set_error_handler(error);

    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "2D Combat".to_string(),
                    resolution: (1280, 720).into(),
                    ..default()
                }),
                ..default()
            })
            .set(ImagePlugin::default_nearest()),
    );

    // Physics — default gravity is ~9.81 px/s² which is far too weak for pixel-scale 2D.
    // 980 px/s² gives snappy, game-feel-good jumps.
    app.add_plugins(PhysicsPlugins::default());
    app.insert_resource(Gravity(Vec2::new(0.0, -1400.0)));

    // Our plugins
    app.add_plugins((
        screens::plugin,
        input::plugin,
        animation::plugin,
        combat::plugin,
        health::plugin,
        level::plugin,
        player::plugin,
        enemy::plugin,
    ));

    // Character controller (Tnua + Avian2D integration)
    app.add_plugins((
        TnuaControllerPlugin::<PlayerControlScheme>::new(FixedUpdate),
        TnuaAvian2dPlugin::new(FixedUpdate),
    ));

    // Screen shake
    app.add_plugins(TraumaPlugin);

    // Type registration for inspector
    app.register_type::<health::Health>();
    app.register_type::<enemy::EnemyState>();
    app.register_type::<combat::Attacking>();

    // Debug inspector (dev builds only)
    #[cfg(feature = "dev")]
    {
        app.add_plugins(bevy_inspector_egui::bevy_egui::EguiPlugin::default());
        app.add_plugins(WorldInspectorPlugin::new());
    }

    // Spawn camera
    app.add_systems(Startup, spawn_camera);

    app.run();
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Shake::default(),
        ShakeSettings {
            amplitude: 40.0,
            decay_per_second: 1.2,
            ..default()
        },
    ));
}
