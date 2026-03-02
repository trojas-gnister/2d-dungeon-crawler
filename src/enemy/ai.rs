use avian2d::prelude::*;
use bevy::prelude::*;
use bevy_spritesheet_animation::prelude::*;
use bevy_trauma_shake::prelude::*;

use crate::animation::CharacterAnims;
use crate::health::{DeathDespawnTimer, Health};
use crate::player::Player;

use super::{
    Enemy, EnemyAttackCooldown, EnemyAttackTimer, EnemySpeedMultiplier, EnemyState, FlyingState,
    HoverAltitude, IdleTimer, PatrolBounds, PatrolDirection, SwoopCooldown, SwoopTarget,
    SwoopTimer,
};

const PATROL_SPEED: f32 = 60.0;
const CHASE_SPEED: f32 = 120.0;
const DETECTION_RANGE: f32 = 200.0;
const CHASE_DROP_OFF: f32 = 350.0;
const MELEE_RANGE: f32 = 40.0;
const ATTACK_DURATION: f32 = 0.5;

// Flying AI constants
const HOVER_BOB_AMPLITUDE: f32 = 15.0;
const HOVER_BOB_SPEED: f32 = 2.0;
const HOVER_DRIFT_SPEED: f32 = 30.0;
const SWOOP_SPEED: f32 = 250.0;
const RETREAT_SPEED: f32 = 150.0;
const SWOOP_DETECTION_RANGE: f32 = 300.0;

// ---------------------------------------------------------------------------
// Ground enemy AI (unchanged logic, added Without<FlyingState> filter)
// ---------------------------------------------------------------------------

/// Decides state transitions for ground enemies.
pub fn enemy_ai_decision(
    time: Res<Time>,
    player_query: Query<&Transform, With<Player>>,
    mut enemy_query: Query<
        (
            Entity,
            &Transform,
            &mut EnemyState,
            &mut IdleTimer,
            &Health,
            &mut EnemyAttackCooldown,
            Option<&mut EnemyAttackTimer>,
        ),
        (With<Enemy>, Without<FlyingState>),
    >,
    mut commands: Commands,
    mut shakes: Shakes,
) {
    let Ok(player_tf) = player_query.single() else {
        return;
    };

    for (entity, tf, mut state, mut idle_timer, health, mut cooldown, attack_timer) in
        &mut enemy_query
    {
        // Dead enemies stay dead
        if health.is_dead() {
            if *state != EnemyState::Dead {
                *state = EnemyState::Dead;
                shakes.add_trauma(0.5);
                commands
                    .entity(entity)
                    .insert(DeathDespawnTimer(Timer::from_seconds(2.0, TimerMode::Once)));
            }
            continue;
        }

        let distance = player_tf.translation.xy().distance(tf.translation.xy());

        cooldown.tick(time.delta());

        match *state {
            EnemyState::Idle => {
                if distance < DETECTION_RANGE {
                    *state = EnemyState::Chase;
                    continue;
                }
                idle_timer.tick(time.delta());
                if idle_timer.is_finished() {
                    *state = EnemyState::Patrol;
                }
            }
            EnemyState::Patrol => {
                if distance < DETECTION_RANGE {
                    *state = EnemyState::Chase;
                }
            }
            EnemyState::Chase => {
                if distance < MELEE_RANGE && cooldown.is_finished() {
                    *state = EnemyState::Attacking;
                    continue;
                }
                if distance > CHASE_DROP_OFF {
                    *state = EnemyState::Idle;
                    idle_timer.reset();
                }
            }
            EnemyState::Attacking => {
                if let Some(mut timer) = attack_timer {
                    timer.tick(time.delta());
                    if timer.is_finished() {
                        *state = EnemyState::Chase;
                        cooldown.reset();
                    }
                }
            }
            EnemyState::Dead => {}
        }
    }

    // Second pass: insert attack timers for enemies that just entered Attacking state
    for (entity, state) in enemy_query
        .transmute_lens_filtered::<(Entity, &EnemyState), (With<Enemy>, Without<FlyingState>)>()
        .query()
        .iter()
    {
        match *state {
            EnemyState::Attacking => {
                commands
                    .entity(entity)
                    .entry::<EnemyAttackTimer>()
                    .or_insert(EnemyAttackTimer(Timer::from_seconds(
                        ATTACK_DURATION,
                        TimerMode::Once,
                    )));
            }
            EnemyState::Dead => {}
            _ => {
                commands.entity(entity).remove::<EnemyAttackTimer>();
            }
        }
    }
}

/// Moves ground enemies based on their current state, scaled by speed multiplier.
pub fn enemy_movement(
    anims: Res<CharacterAnims>,
    player_query: Query<&Transform, With<Player>>,
    mut enemy_query: Query<
        (
            &Transform,
            &EnemyState,
            &EnemySpeedMultiplier,
            &mut LinearVelocity,
            &PatrolBounds,
            &mut PatrolDirection,
            &mut Sprite,
            &mut SpritesheetAnimation,
        ),
        (With<Enemy>, Without<FlyingState>),
    >,
) {
    let Ok(player_tf) = player_query.single() else {
        return;
    };

    for (tf, state, speed_mult, mut velocity, bounds, mut patrol_dir, mut sprite, mut anim) in
        &mut enemy_query
    {
        let mult = speed_mult.0;
        match state {
            EnemyState::Idle | EnemyState::Dead => {
                velocity.x = 0.0;
                switch_anim(&mut anim, &anims.idle);
            }
            EnemyState::Patrol => {
                if tf.translation.x <= bounds.left {
                    patrol_dir.0 = 1.0;
                } else if tf.translation.x >= bounds.right {
                    patrol_dir.0 = -1.0;
                }
                velocity.x = patrol_dir.0 * PATROL_SPEED * mult;
                sprite.flip_x = patrol_dir.0 < 0.0;
                switch_anim(&mut anim, &anims.walk);
            }
            EnemyState::Chase => {
                let dir = (player_tf.translation.x - tf.translation.x).signum();
                velocity.x = dir * CHASE_SPEED * mult;
                sprite.flip_x = dir < 0.0;
                switch_anim(&mut anim, &anims.walk);
            }
            EnemyState::Attacking => {
                velocity.x = 0.0;
                switch_anim(&mut anim, &anims.idle);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Flying enemy AI
// ---------------------------------------------------------------------------

/// Hover/swoop/retreat state machine for flying enemies.
pub fn flying_ai(
    time: Res<Time>,
    player_query: Query<&Transform, With<Player>>,
    mut flyer_query: Query<
        (
            Entity,
            &mut Transform,
            &mut FlyingState,
            &HoverAltitude,
            &Health,
            &EnemySpeedMultiplier,
            &mut SwoopCooldown,
            &mut SwoopTimer,
            &mut Sprite,
            Option<&SwoopTarget>,
        ),
        (With<Enemy>, Without<Player>),
    >,
    mut commands: Commands,
) {
    let Ok(player_tf) = player_query.single() else {
        return;
    };
    let player_pos = player_tf.translation.xy();
    let elapsed_secs = time.elapsed_secs();
    let dt = time.delta_secs();

    for (entity, mut tf, mut state, hover_alt, health, speed_mult, mut cooldown, mut swoop_timer, mut sprite, swoop_target) in
        &mut flyer_query
    {
        let mult = speed_mult.0;
        // Dead flyers: trigger death despawn
        if health.is_dead() {
            if *state != FlyingState::Hover {
                // Just reset to avoid continued movement
                *state = FlyingState::Hover;
            }
            // Death is handled by the shared death_despawn_tick system via Health check
            // Insert despawn timer if not already present
            commands
                .entity(entity)
                .entry::<DeathDespawnTimer>()
                .or_insert(DeathDespawnTimer(Timer::from_seconds(2.0, TimerMode::Once)));
            continue;
        }

        let distance = player_pos.distance(tf.translation.xy());

        match *state {
            FlyingState::Hover => {
                // Sinusoidal bob at hover altitude
                let bob = (elapsed_secs * HOVER_BOB_SPEED).sin() * HOVER_BOB_AMPLITUDE;
                tf.translation.y = hover_alt.0 + bob;

                // Drift toward player X slowly
                let dx = player_pos.x - tf.translation.x;
                tf.translation.x += dx.signum() * HOVER_DRIFT_SPEED * mult * dt
                    * (dx.abs().min(1.0));
                sprite.flip_x = dx < 0.0;

                // Check if we should swoop
                cooldown.tick(time.delta());
                if cooldown.is_finished() && distance < SWOOP_DETECTION_RANGE {
                    *state = FlyingState::Swoop;
                    swoop_timer.reset();
                    // Capture player position for the dive
                    commands.entity(entity).insert(SwoopTarget(player_pos));
                }
            }
            FlyingState::Swoop => {
                swoop_timer.tick(time.delta());

                if let Some(target) = swoop_target {
                    let dir = (target.0 - tf.translation.xy()).normalize_or_zero();
                    tf.translation.x += dir.x * SWOOP_SPEED * mult * dt;
                    tf.translation.y += dir.y * SWOOP_SPEED * mult * dt;
                    sprite.flip_x = dir.x < 0.0;

                    // Reached target or timer expired → retreat
                    let dist_to_target = target.0.distance(tf.translation.xy());
                    if dist_to_target < 20.0 || swoop_timer.is_finished() {
                        *state = FlyingState::Retreat;
                        commands.entity(entity).remove::<SwoopTarget>();
                    }
                } else {
                    // No target somehow — retreat
                    *state = FlyingState::Retreat;
                }
            }
            FlyingState::Retreat => {
                // Fly back up to hover altitude
                let target_y = hover_alt.0;
                let dy = target_y - tf.translation.y;

                if dy.abs() < 5.0 {
                    tf.translation.y = target_y;
                    *state = FlyingState::Hover;
                    cooldown.reset();
                } else {
                    tf.translation.y += dy.signum() * RETREAT_SPEED * mult * dt;
                }
            }
        }
    }
}

/// Switch animation only if it's not already the target (avoids resetting mid-animation).
fn switch_anim(anim: &mut SpritesheetAnimation, target: &Handle<Animation>) {
    if anim.animation != *target {
        anim.switch(target.clone());
    }
}
