use std::collections::HashSet;

use avian2d::prelude::*;
use bevy::prelude::*;
use rand::Rng;

use crate::player::Player;
use crate::screens::Screen;

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::Gameplay), setup_level);
    app.add_systems(OnExit(Screen::Gameplay), cleanup_level_resources);
    app.add_systems(
        Update,
        (track_player_progress, generate_chunks, despawn_old_chunks)
            .chain()
            .run_if(in_state(Screen::Gameplay)),
    );
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Height of each chunk in pixels.
pub const CHUNK_HEIGHT: f32 = 600.0;

/// How many chunks ahead of the player to generate.
const GENERATE_AHEAD: i32 = 2;

/// How many chunks behind the player before despawning.
const DESPAWN_BEHIND: i32 = 2;

/// Horizontal range for platform placement.
const PLATFORM_X_RANGE: f32 = 500.0;

/// Chunks per difficulty tier.
const CHUNKS_PER_TIER: i32 = 5;

// ---------------------------------------------------------------------------
// Components & Resources
// ---------------------------------------------------------------------------

/// Marker for ground/platform entities.
#[derive(Component)]
pub struct Ground;

/// Tags an entity with the chunk it belongs to, for batch despawn.
#[derive(Component)]
pub struct ChunkId(pub i32);

/// Tracks which chunks have been generated so we don't double-create.
#[derive(Resource, Default)]
pub struct ChunkTracker {
    pub generated: HashSet<i32>,
}

/// Tracks the player's highest Y position (used for chunk generation).
#[derive(Resource)]
pub struct PlayerProgress {
    pub highest_y: f32,
}

impl Default for PlayerProgress {
    fn default() -> Self {
        Self { highest_y: -200.0 }
    }
}

/// Buffer of newly generated chunks — enemy spawner drains this each frame.
#[derive(Resource, Default)]
pub struct NewChunks {
    pub chunks: Vec<NewChunkData>,
}

/// Data about a freshly generated chunk, consumed by enemy spawning.
pub struct NewChunkData {
    pub chunk_index: i32,
    pub tier: u32,
    /// (x, y, width) for each platform in the chunk.
    pub platforms: Vec<(f32, f32, f32)>,
}

// ---------------------------------------------------------------------------
// Difficulty tier
// ---------------------------------------------------------------------------

/// Returns the difficulty tier for a chunk index (new tier every 5 chunks).
pub fn difficulty_tier(chunk_index: i32) -> u32 {
    (chunk_index.max(0) / CHUNKS_PER_TIER) as u32
}

/// Platform color darkens with tier.
fn tier_platform_color(tier: u32) -> Color {
    match tier {
        0 => Color::srgb(0.5, 0.35, 0.2),  // Brown
        1 => Color::srgb(0.4, 0.28, 0.16), // Darker brown
        2 => Color::srgb(0.4, 0.4, 0.4),   // Gray
        _ => Color::srgb(0.3, 0.3, 0.3),   // Dark gray
    }
}

/// Minimum platform width shrinks with tier.
fn tier_min_platform_width(tier: u32) -> f32 {
    match tier {
        0 => 120.0,
        1 => 100.0,
        2 => 80.0,
        _ => 60.0,
    }
}

/// Maximum platform width also shrinks slightly.
fn tier_max_platform_width(tier: u32) -> f32 {
    match tier {
        0 => 200.0,
        1 => 180.0,
        2 => 160.0,
        _ => 140.0,
    }
}

/// Speed multiplier for enemies at this tier.
pub fn tier_speed_multiplier(tier: u32) -> f32 {
    1.0 + tier as f32 * 0.15
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

fn setup_level(mut commands: Commands) {
    commands.init_resource::<ChunkTracker>();
    commands.init_resource::<PlayerProgress>();
    commands.init_resource::<NewChunks>();

    // Wide starting platform so the player has a safe landing spot
    commands.spawn((
        Ground,
        ChunkId(-1),
        DespawnOnExit(Screen::Gameplay),
        Name::new("Starting Platform"),
        Sprite {
            color: Color::srgb(0.2, 0.7, 0.2),
            custom_size: Some(Vec2::new(400.0, 40.0)),
            ..default()
        },
        Transform::from_xyz(0.0, -300.0, 0.0),
        RigidBody::Static,
        Collider::rectangle(400.0, 40.0),
    ));
}

fn cleanup_level_resources(mut commands: Commands) {
    commands.remove_resource::<ChunkTracker>();
    commands.remove_resource::<PlayerProgress>();
    commands.remove_resource::<NewChunks>();
}

/// Update highest_y based on the player's current position.
fn track_player_progress(
    player_query: Query<&Transform, With<Player>>,
    mut progress: ResMut<PlayerProgress>,
) {
    let Ok(tf) = player_query.single() else {
        return;
    };
    if tf.translation.y > progress.highest_y {
        progress.highest_y = tf.translation.y;
    }
}

/// Generate new chunks ahead of the player.
fn generate_chunks(
    mut commands: Commands,
    progress: Res<PlayerProgress>,
    mut tracker: ResMut<ChunkTracker>,
    mut new_chunks: ResMut<NewChunks>,
) {
    let current_chunk = (progress.highest_y / CHUNK_HEIGHT).floor() as i32;

    for chunk_index in (current_chunk - 1)..=(current_chunk + GENERATE_AHEAD) {
        if tracker.generated.contains(&chunk_index) {
            continue;
        }
        tracker.generated.insert(chunk_index);

        let tier = difficulty_tier(chunk_index);
        let platforms = spawn_chunk_platforms(&mut commands, chunk_index, tier);
        new_chunks.chunks.push(NewChunkData {
            chunk_index,
            tier,
            platforms,
        });
    }
}

/// Despawn chunks far below the player.
fn despawn_old_chunks(
    mut commands: Commands,
    progress: Res<PlayerProgress>,
    mut tracker: ResMut<ChunkTracker>,
    query: Query<(Entity, &ChunkId)>,
) {
    let current_chunk = (progress.highest_y / CHUNK_HEIGHT).floor() as i32;
    let min_chunk = current_chunk - DESPAWN_BEHIND;

    for (entity, chunk_id) in &query {
        if chunk_id.0 < min_chunk {
            commands.entity(entity).despawn();
            tracker.generated.remove(&chunk_id.0);
        }
    }
}

// ---------------------------------------------------------------------------
// Chunk generation helpers
// ---------------------------------------------------------------------------

/// Spawns platforms for a single chunk with tier-based difficulty.
fn spawn_chunk_platforms(
    commands: &mut Commands,
    chunk_index: i32,
    tier: u32,
) -> Vec<(f32, f32, f32)> {
    let mut rng = rand::rng();
    let chunk_base_y = chunk_index as f32 * CHUNK_HEIGHT;

    let platform_count = rng.random_range(4..=6);
    let platform_height = 20.0;
    let color = tier_platform_color(tier);
    let min_w = tier_min_platform_width(tier);
    let max_w = tier_max_platform_width(tier);

    let mut platforms = Vec::with_capacity(platform_count);

    for i in 0..platform_count {
        // Spread platforms evenly within the chunk with some jitter
        let spacing = CHUNK_HEIGHT / platform_count as f32;
        let base_y = chunk_base_y + (i as f32 * spacing) + spacing * 0.5;
        let y = base_y + rng.random_range(-20.0..=20.0);

        // First platform in each chunk is the "path" platform — guaranteed reachable
        let (x, width) = if i == 0 {
            let path_x = rng.random_range(-200.0..=200.0);
            let path_w = min_w.max(120.0); // path platform is always reasonably wide
            (path_x, path_w)
        } else {
            let w = rng.random_range(min_w..=max_w);
            let x = rng.random_range(-PLATFORM_X_RANGE..=PLATFORM_X_RANGE);
            (x, w)
        };

        commands.spawn((
            Ground,
            ChunkId(chunk_index),
            DespawnOnExit(Screen::Gameplay),
            Name::new(format!("Platform c{chunk_index}_{i}")),
            Sprite {
                color,
                custom_size: Some(Vec2::new(width, platform_height)),
                ..default()
            },
            Transform::from_xyz(x, y, 0.0),
            RigidBody::Static,
            Collider::rectangle(width, platform_height),
        ));

        platforms.push((x, y, width));
    }

    platforms
}
