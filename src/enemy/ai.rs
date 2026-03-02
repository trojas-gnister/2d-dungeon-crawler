use avian2d::prelude::*;
use bevy::prelude::*;
use bevy_spritesheet_animation::prelude::*;
use bevy_trauma_shake::prelude::*;

use crate::animation::CharacterAnims;
use crate::health::{DeathDespawnTimer, Health};
use crate::player::Player;

use super::{
    Enemy, EnemyAttackCooldown, EnemyAttackTimer, EnemyState, IdleTimer, PatrolBounds,
    PatrolDirection,
};

const PATROL_SPEED: f32 = 60.0;
const CHASE_SPEED: f32 = 120.0;
const DETECTION_RANGE: f32 = 200.0;
const CHASE_DROP_OFF: f32 = 350.0;
const MELEE_RANGE: f32 = 40.0;
const ATTACK_DURATION: f32 = 0.5;

/// Decides state transitions based on distance to player, timers, and health.
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
        With<Enemy>,
    >,
    mut commands: Commands,
    mut shakes: Shakes,
) {
    let Ok(player_tf) = player_query.single() else {
        return;
    };

    for (entity, tf, mut state, mut idle_timer, health, mut cooldown, attack_timer) in &mut enemy_query {
        // Dead enemies stay dead
        if health.is_dead() {
            if *state != EnemyState::Dead {
                *state = EnemyState::Dead;
                shakes.add_trauma(0.5);
                commands.entity(entity).insert(
                    DeathDespawnTimer(Timer::from_seconds(2.0, TimerMode::Once)),
                );
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
        .transmute_lens_filtered::<(Entity, &EnemyState), With<Enemy>>()
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

/// Moves enemies based on their current state.
pub fn enemy_movement(
    anims: Res<CharacterAnims>,
    player_query: Query<&Transform, With<Player>>,
    mut enemy_query: Query<
        (
            &Transform,
            &EnemyState,
            &mut LinearVelocity,
            &PatrolBounds,
            &mut PatrolDirection,
            &mut Sprite,
            &mut SpritesheetAnimation,
        ),
        With<Enemy>,
    >,
) {
    let Ok(player_tf) = player_query.single() else {
        return;
    };

    for (tf, state, mut velocity, bounds, mut patrol_dir, mut sprite, mut anim) in
        &mut enemy_query
    {
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
                velocity.x = patrol_dir.0 * PATROL_SPEED;
                sprite.flip_x = patrol_dir.0 < 0.0;
                switch_anim(&mut anim, &anims.walk);
            }
            EnemyState::Chase => {
                let dir = (player_tf.translation.x - tf.translation.x).signum();
                velocity.x = dir * CHASE_SPEED;
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

/// Switch animation only if it's not already the target (avoids resetting mid-animation).
fn switch_anim(anim: &mut SpritesheetAnimation, target: &Handle<Animation>) {
    if anim.animation != *target {
        anim.switch(target.clone());
    }
}
