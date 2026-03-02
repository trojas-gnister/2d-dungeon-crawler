use bevy::prelude::*;
use bevy_spritesheet_animation::prelude::*;

/// Shared animation handles for all characters (player + enemies share the same frame layout).
#[derive(Resource)]
pub struct CharacterAnims {
    pub idle: Handle<Animation>,
    pub walk: Handle<Animation>,
}

pub fn plugin(app: &mut App) {
    app.add_plugins(SpritesheetAnimationPlugin);
    app.add_systems(Startup, setup_animations);
}

fn setup_animations(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut animations: ResMut<Assets<Animation>>,
) {
    // Both player and enemy use 10x1 grids with the same frame layout
    let image = asset_server.load("sprites/player.png");
    let spritesheet = Spritesheet::new(&image, 10, 1);

    let idle = animations.add(
        spritesheet
            .create_animation()
            .add_horizontal_strip(0, 0, 4) // frames 0-3
            .set_duration(AnimationDuration::PerFrame(150))
            .set_repetitions(AnimationRepeat::Loop)
            .build(),
    );

    let walk = animations.add(
        spritesheet
            .create_animation()
            .add_horizontal_strip(4, 0, 6) // frames 4-9
            .set_duration(AnimationDuration::PerFrame(150))
            .set_repetitions(AnimationRepeat::Loop)
            .build(),
    );

    commands.insert_resource(CharacterAnims { idle, walk });
}
