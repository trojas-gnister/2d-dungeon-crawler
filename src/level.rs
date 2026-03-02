use avian2d::prelude::*;
use bevy::prelude::*;

use crate::screens::Screen;

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::Gameplay), spawn_level);
}

/// Marker for ground/platform entities.
#[derive(Component)]
pub struct Ground;

/// (x, y, width, height) for each platform.
const PLATFORMS: &[(f32, f32, f32, f32)] = &[
    // Main ground
    (0.0, -300.0, 2000.0, 40.0),
    // Floating platforms
    (-300.0, -180.0, 160.0, 20.0),
    (100.0, -100.0, 200.0, 20.0),
    (400.0, -180.0, 120.0, 20.0),
    (-500.0, -50.0, 140.0, 20.0),
    (600.0, -80.0, 180.0, 20.0),
];

fn spawn_level(mut commands: Commands) {
    for (i, &(x, y, w, h)) in PLATFORMS.iter().enumerate() {
        let color = if i == 0 {
            Color::srgb(0.2, 0.7, 0.2)
        } else {
            Color::srgb(0.5, 0.35, 0.2)
        };

        commands.spawn((
            Ground,
            DespawnOnExit(Screen::Gameplay),
            Name::new(if i == 0 {
                "Ground".to_string()
            } else {
                format!("Platform {i}")
            }),
            Sprite {
                color,
                custom_size: Some(Vec2::new(w, h)),
                ..default()
            },
            Transform::from_xyz(x, y, 0.0),
            RigidBody::Static,
            Collider::rectangle(w, h),
        ));
    }
}
