pub mod ai;

use avian2d::prelude::*;
use bevy::prelude::*;
use bevy_spritesheet_animation::prelude::*;

use crate::animation::CharacterAnims;
use crate::health::Health;
use crate::screens::Screen;

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::Gameplay), spawn_enemies);
    app.add_systems(
        Update,
        (ai::enemy_ai_decision, ai::enemy_movement)
            .chain()
            .run_if(in_state(Screen::Gameplay)),
    );
}

/// Marker for enemy entities.
#[derive(Component)]
pub struct Enemy;

/// Current AI state.
#[derive(Component, Default, PartialEq, Eq, Debug, Reflect)]
#[reflect(Component)]
pub enum EnemyState {
    #[default]
    Idle,
    Patrol,
    Chase,
    Attacking,
    Dead,
}

/// Left/right bounds for patrol movement (world x coordinates).
#[derive(Component)]
pub struct PatrolBounds {
    pub left: f32,
    pub right: f32,
}

/// Timer used to transition from Idle → Patrol after a short pause.
#[derive(Component, Deref, DerefMut)]
pub struct IdleTimer(pub Timer);

/// Timer for attack duration.
#[derive(Component, Deref, DerefMut)]
pub struct EnemyAttackTimer(pub Timer);

/// Cooldown before the enemy can attack again.
#[derive(Component, Deref, DerefMut)]
pub struct EnemyAttackCooldown(pub Timer);

/// Which direction the enemy is currently walking (1.0 = right, -1.0 = left).
#[derive(Component)]
pub struct PatrolDirection(pub f32);

/// (x, y, patrol_left, patrol_right) for each enemy spawn.
const ENEMY_SPAWNS: &[(f32, f32, f32, f32)] = &[
    (-250.0, -250.0, -350.0, -150.0),
    (200.0, -250.0, 100.0, 300.0),
    (500.0, -250.0, 400.0, 650.0),
];

fn spawn_enemies(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    anims: Res<CharacterAnims>,
) {
    let texture: Handle<Image> = asset_server.load("sprites/enemy.png");
    let layout = TextureAtlasLayout::from_grid(UVec2::splat(32), 10, 1, None, None);
    let layout_handle = layouts.add(layout);

    for (i, &(x, y, patrol_left, patrol_right)) in ENEMY_SPAWNS.iter().enumerate() {
        commands
            .spawn((
                Enemy,
                EnemyState::Idle,
                PatrolBounds {
                    left: patrol_left,
                    right: patrol_right,
                },
                PatrolDirection(1.0),
                IdleTimer(Timer::from_seconds(2.0, TimerMode::Once)),
                EnemyAttackCooldown({
                    let mut t = Timer::from_seconds(1.5, TimerMode::Once);
                    t.tick(std::time::Duration::from_secs(2));
                    t
                }),
                Health::new(100.0),
                Name::new(format!("Enemy {i}")),
                DespawnOnExit(Screen::Gameplay),
            ))
            .insert((
                Sprite::from_atlas_image(
                    texture.clone(),
                    TextureAtlas {
                        layout: layout_handle.clone(),
                        index: 0,
                    },
                ),
                Transform::from_xyz(x, y, 1.0),
                SpritesheetAnimation::new(anims.idle.clone()),
                RigidBody::Dynamic,
                Collider::rectangle(32.0, 32.0),
                LockedAxes::ROTATION_LOCKED,
                LinearVelocity::ZERO,
            ));
    }
}
