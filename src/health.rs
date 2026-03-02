use bevy::prelude::*;

use crate::enemy::{Enemy, EnemyState};
use crate::level::{ChunkTracker, CHUNK_HEIGHT};
use crate::player::Player;
use crate::screens::Screen;

pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (death_despawn_tick, void_death_check, player_death_check)
            .chain()
            .run_if(in_state(Screen::Gameplay)),
    );
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

/// When player health reaches 0, go back to title screen.
fn player_death_check(
    query: Query<&Health, With<Player>>,
    mut next_state: ResMut<NextState<Screen>>,
) {
    let Ok(health) = query.single() else {
        return;
    };

    if health.is_dead() {
        next_state.set(Screen::Title);
    }
}
