use bevy::prelude::*;

use crate::enemy::{Enemy, EnemyState};
use crate::level::{ChunkTracker, PlayerProgress, CHUNK_HEIGHT};
use crate::player::Player;
use crate::screens::Screen;

pub fn plugin(app: &mut App) {
    app.init_resource::<BestScore>();

    app.add_systems(OnEnter(Screen::Gameplay), init_kill_count);
    app.add_systems(OnExit(Screen::GameOver), cleanup_run_resources);

    app.add_systems(
        Update,
        (death_despawn_tick, void_death_check, player_death_check)
            .chain()
            .run_if(in_state(Screen::Gameplay)),
    );
}

fn init_kill_count(mut commands: Commands) {
    commands.init_resource::<EnemyKillCount>();
}

fn cleanup_run_resources(mut commands: Commands) {
    commands.remove_resource::<RunStats>();
    commands.remove_resource::<EnemyKillCount>();
}

/// Health component shared by player and enemies.
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

impl Health {
    pub fn new(max: f32) -> Self {
        Self { current: max, max }
    }

    pub fn fraction(&self) -> f32 {
        (self.current / self.max).clamp(0.0, 1.0)
    }

    pub fn is_dead(&self) -> bool {
        self.current <= 0.0
    }
}

/// Attached to dead enemies. Counts down then despawns the entity.
#[derive(Component, Deref, DerefMut)]
pub struct DeathDespawnTimer(pub Timer);

/// Tracks enemies killed during the current run.
#[derive(Resource, Default)]
pub struct EnemyKillCount(pub u32);

/// Snapshot of a completed run, used by the GameOver screen.
#[derive(Resource)]
pub struct RunStats {
    pub height_meters: i32,
    pub enemies_killed: u32,
    pub is_new_best: bool,
}

/// All-time best height in meters. Persists across the entire app lifetime.
#[derive(Resource, Default)]
pub struct BestScore {
    pub height_meters: i32,
}

/// Fade dead enemies and despawn when timer expires.
fn death_despawn_tick(
    time: Res<Time>,
    mut query: Query<(Entity, &mut DeathDespawnTimer, &mut Sprite, &EnemyState), With<Enemy>>,
    mut commands: Commands,
) {
    for (entity, mut timer, mut sprite, state) in &mut query {
        if *state != EnemyState::Dead {
            continue;
        }

        timer.tick(time.delta());

        // Fade alpha based on remaining time
        let alpha = 1.0 - timer.fraction();
        sprite.color = sprite.color.with_alpha(alpha);

        if timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

/// Kill the player if they fall far below the lowest active chunk.
fn void_death_check(
    tracker: Res<ChunkTracker>,
    mut player_query: Query<(&Transform, &mut Health), With<Player>>,
) {
    let Ok((tf, mut health)) = player_query.single_mut() else {
        return;
    };

    if health.is_dead() {
        return;
    }

    // Find the lowest generated chunk
    let lowest_chunk = tracker
        .generated
        .iter()
        .copied()
        .min()
        .unwrap_or(0);
    let void_y = lowest_chunk as f32 * CHUNK_HEIGHT - 1000.0;

    if tf.translation.y < void_y {
        health.current = 0.0;
    }
}

/// When player health reaches 0, snapshot stats and transition to GameOver.
fn player_death_check(
    query: Query<&Health, With<Player>>,
    progress: Option<Res<PlayerProgress>>,
    kill_count: Option<Res<EnemyKillCount>>,
    mut best_score: ResMut<BestScore>,
    mut next_state: ResMut<NextState<Screen>>,
    mut commands: Commands,
) {
    let Ok(health) = query.single() else {
        return;
    };

    if !health.is_dead() {
        return;
    }

    let height_meters = progress
        .map(|p| (p.highest_y / 100.0).max(0.0) as i32)
        .unwrap_or(0);

    let enemies_killed = kill_count.map(|k| k.0).unwrap_or(0);

    let is_new_best = height_meters > best_score.height_meters;
    if is_new_best {
        best_score.height_meters = height_meters;
    }

    commands.insert_resource(RunStats {
        height_meters,
        enemies_killed,
        is_new_best,
    });

    next_state.set(Screen::GameOver);
}
