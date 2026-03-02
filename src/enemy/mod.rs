pub mod ai;

use avian2d::prelude::*;
use bevy::prelude::*;
use bevy_spritesheet_animation::prelude::*;
use rand::Rng;

use crate::animation::CharacterAnims;
use crate::health::Health;
use crate::level::{self, ChunkId, NewChunks};
use crate::screens::Screen;

pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            spawn_enemies_for_new_chunks,
            ai::enemy_ai_decision,
            ai::enemy_movement,
            ai::flying_ai,
        )
            .chain()
            .run_if(in_state(Screen::Gameplay)),
    );
}

/// Marker for enemy entities.
#[derive(Component)]
pub struct Enemy;

/// Distinguishes ground vs flying enemies for query filtering.
#[derive(Component, PartialEq, Eq, Debug)]
pub enum EnemyKind {
    Ground,
    Flying,
}

/// Current AI state for ground enemies.
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

/// Scales patrol/chase speed per difficulty tier.
#[derive(Component)]
pub struct EnemySpeedMultiplier(pub f32);

// ---------------------------------------------------------------------------
// Flying enemy components
// ---------------------------------------------------------------------------

/// State machine for flying enemies: hover → swoop → retreat → hover.
#[derive(Component, Default, PartialEq, Eq, Debug)]
pub enum FlyingState {
    #[default]
    Hover,
    Swoop,
    Retreat,
}

/// The altitude the flyer hovers at (world Y).
#[derive(Component)]
pub struct HoverAltitude(pub f32);

/// Captured player position when swoop begins.
#[derive(Component)]
pub struct SwoopTarget(pub Vec2);

/// Cooldown between swoops.
#[derive(Component, Deref, DerefMut)]
pub struct SwoopCooldown(pub Timer);

/// Max duration of a single swoop before forced retreat.
#[derive(Component, Deref, DerefMut)]
pub struct SwoopTimer(pub Timer);

/// Purple tint color for flying enemies.
const FLYER_COLOR: Color = Color::srgb(0.7, 0.4, 1.0);

/// No flyers in the first N chunks.
const FLYER_MIN_CHUNK: i32 = 5;

/// Drains `NewChunks` and spawns ground + flying enemies with tier-based scaling.
fn spawn_enemies_for_new_chunks(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    anims: Res<CharacterAnims>,
    mut new_chunks: ResMut<NewChunks>,
) {
    if new_chunks.chunks.is_empty() {
        return;
    }

    let texture: Handle<Image> = asset_server.load("sprites/enemy.png");
    let layout = TextureAtlasLayout::from_grid(UVec2::splat(32), 10, 1, None, None);
    let layout_handle = layouts.add(layout);

    let mut rng = rand::rng();

    for chunk_data in new_chunks.chunks.drain(..) {
        let tier = chunk_data.tier;
        let speed_mult = level::tier_speed_multiplier(tier);

        // Tier-based max ground enemies per chunk
        let max_ground = match tier {
            0 => 2,
            1 => 3,
            2 => 4,
            _ => 4,
        };

        let mut ground_spawned = 0;

        for &(px, py, pw) in &chunk_data.platforms {
            if ground_spawned >= max_ground {
                break;
            }

            // ~40% chance to spawn a ground enemy on each platform
            if rng.random_range(0.0..1.0) > 0.4 {
                continue;
            }

            // Skip platforms too narrow for meaningful patrol
            if pw < 64.0 {
                continue;
            }

            let half_w = pw / 2.0;
            let patrol_left = px - half_w + 16.0;
            let patrol_right = px + half_w - 16.0;
            // Spawn enemy on top of the platform
            let enemy_y = py + 26.0;

            commands
                .spawn((
                    Enemy,
                    EnemyKind::Ground,
                    EnemyState::Idle,
                    EnemySpeedMultiplier(speed_mult),
                    ChunkId(chunk_data.chunk_index),
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
                    Name::new(format!(
                        "Enemy c{}_{ground_spawned}",
                        chunk_data.chunk_index
                    )),
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
                    Transform::from_xyz(px, enemy_y, 1.0),
                    SpritesheetAnimation::new(anims.idle.clone()),
                    RigidBody::Dynamic,
                    Collider::rectangle(32.0, 32.0),
                    LockedAxes::ROTATION_LOCKED,
                    LinearVelocity::ZERO,
                ));

            ground_spawned += 1;
        }

        // Spawn flying enemies in air (not on platforms), only in later chunks
        if chunk_data.chunk_index >= FLYER_MIN_CHUNK {
            let max_flyers: usize = match tier {
                0 => 0,
                1 => 1,
                2 => 2,
                _ => 2,
            };

            for fi in 0..max_flyers {
                // Each flyer has its own chance
                let fly_chance = 0.4 + (tier as f32 * 0.1).min(0.3);
                if rng.random_range(0.0..1.0) > fly_chance {
                    continue;
                }

                let chunk_base_y = chunk_data.chunk_index as f32 * level::CHUNK_HEIGHT;
                let fly_x = rng.random_range(-400.0..=400.0);
                let fly_y = chunk_base_y + rng.random_range(200.0..=500.0);

                spawn_flying_enemy(
                    &mut commands,
                    &texture,
                    &layout_handle,
                    &anims,
                    chunk_data.chunk_index,
                    fly_x,
                    fly_y,
                    ground_spawned + fi,
                    speed_mult,
                );
            }
        }
    }
}

fn spawn_flying_enemy(
    commands: &mut Commands,
    texture: &Handle<Image>,
    layout_handle: &Handle<TextureAtlasLayout>,
    anims: &CharacterAnims,
    chunk_index: i32,
    x: f32,
    y: f32,
    index: usize,
    speed_mult: f32,
) {
    commands
        .spawn((
            Enemy,
            EnemyKind::Flying,
            EnemySpeedMultiplier(speed_mult),
            FlyingState::Hover,
            HoverAltitude(y),
            SwoopCooldown(Timer::from_seconds(3.0, TimerMode::Once)),
            SwoopTimer(Timer::from_seconds(1.2, TimerMode::Once)),
            ChunkId(chunk_index),
            Health::new(60.0),
            Name::new(format!("Flyer c{chunk_index}_{index}")),
            DespawnOnExit(Screen::Gameplay),
        ))
        .insert((
            Sprite {
                color: FLYER_COLOR,
                ..Sprite::from_atlas_image(
                    texture.clone(),
                    TextureAtlas {
                        layout: layout_handle.clone(),
                        index: 0,
                    },
                )
            },
            Transform::from_xyz(x, y, 1.0),
            SpritesheetAnimation::new(anims.idle.clone()),
            RigidBody::Kinematic,
            Collider::rectangle(32.0, 32.0),
        ));
}
