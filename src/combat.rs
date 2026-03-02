use bevy::prelude::*;
use bevy_tnua::builtins::TnuaBuiltinKnockback;
use bevy_tnua::prelude::*;
use bevy_trauma_shake::prelude::*;
use leafwing_input_manager::prelude::*;

use crate::enemy::{Enemy, EnemyAttackTimer, EnemyState};
use crate::health::Health;
use crate::input::PlayerAction;
use crate::player::{
    Facing, Player, PlayerControlScheme, PlayerControlSchemeActionDiscriminant,
};
use crate::screens::Screen;

pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            player_attack_input,
            player_attack_tick,
            player_hit_detection,
            enemy_hit_player,
            attack_visual_feedback,
            hit_flash_decay,
        )
            .chain()
            .run_if(in_state(Screen::Gameplay)),
    );
}

const ATTACK_DURATION: f32 = 0.4;
const ATTACK_RANGE: f32 = 60.0;
const ATTACK_DAMAGE: f32 = 34.0;
const ENEMY_ATTACK_DAMAGE: f32 = 15.0;
const MELEE_RANGE: f32 = 50.0;
const KNOCKBACK_STRENGTH: f32 = 300.0;
const KNOCKBACK_UP: f32 = 150.0;

/// Marks the player as currently attacking.
#[derive(Component, Deref, DerefMut, Reflect)]
#[reflect(Component)]
pub struct Attacking(pub Timer);

/// Prevents an enemy from being hit twice by the same swing.
#[derive(Component)]
pub struct HitByCurrentSwing;

/// Brief color flash when an entity is hit.
#[derive(Component, Deref, DerefMut)]
pub struct HitFlash(pub Timer);

/// Start an attack when the Attack action is pressed and the player isn't already attacking.
fn player_attack_input(
    query: Query<(Entity, &ActionState<PlayerAction>), (With<Player>, Without<Attacking>)>,
    mut commands: Commands,
) {
    let Ok((entity, action)) = query.single() else {
        return;
    };
    if action.just_pressed(&PlayerAction::Attack) {
        commands.entity(entity).insert(Attacking(Timer::from_seconds(
            ATTACK_DURATION,
            TimerMode::Once,
        )));
    }
}

/// Tick the attack timer and clean up when it expires.
fn player_attack_tick(
    time: Res<Time>,
    mut query: Query<(Entity, &mut Attacking), With<Player>>,
    hit_markers: Query<Entity, With<HitByCurrentSwing>>,
    mut commands: Commands,
) {
    let Ok((entity, mut attacking)) = query.single_mut() else {
        return;
    };

    attacking.tick(time.delta());

    if attacking.is_finished() {
        commands.entity(entity).remove::<Attacking>();
        for marker_entity in &hit_markers {
            commands.entity(marker_entity).remove::<HitByCurrentSwing>();
        }
    }
}

/// Check if any enemies are in range and in front of the player, then deal damage.
fn player_hit_detection(
    player_query: Query<(&Transform, &Facing), (With<Player>, With<Attacking>)>,
    mut enemy_query: Query<
        (Entity, &Transform, &mut Health),
        (With<Enemy>, Without<HitByCurrentSwing>),
    >,
    mut commands: Commands,
    mut shakes: Shakes,
) {
    let Ok((player_tf, facing)) = player_query.single() else {
        return;
    };

    for (enemy_entity, enemy_tf, mut health) in &mut enemy_query {
        let delta = enemy_tf.translation.xy() - player_tf.translation.xy();

        if delta.length() > ATTACK_RANGE {
            continue;
        }

        let in_front = match facing {
            Facing::Right => delta.x > 0.0,
            Facing::Left => delta.x < 0.0,
        };

        if !in_front {
            continue;
        }

        health.current -= ATTACK_DAMAGE;
        shakes.add_trauma(0.15);
        commands.entity(enemy_entity).insert((
            HitByCurrentSwing,
            HitFlash(Timer::from_seconds(0.15, TimerMode::Once)),
        ));
    }
}

/// When an enemy's attack timer finishes, deal damage and knockback to the player.
fn enemy_hit_player(
    enemy_query: Query<(&Transform, &EnemyState, &EnemyAttackTimer), With<Enemy>>,
    mut player_query: Query<
        (Entity, &Transform, &mut Health, &mut TnuaController<PlayerControlScheme>),
        With<Player>,
    >,
    mut commands: Commands,
    mut shakes: Shakes,
) {
    let Ok((player_entity, player_tf, mut player_health, mut controller)) =
        player_query.single_mut()
    else {
        return;
    };

    // Don't stack knockbacks
    if controller.action_discriminant()
        == Some(PlayerControlSchemeActionDiscriminant::Knockback)
    {
        return;
    }

    for (enemy_tf, state, attack_timer) in &enemy_query {
        if *state != EnemyState::Attacking {
            continue;
        }
        // Deal damage at the halfway point of the attack animation
        let elapsed = attack_timer.elapsed_secs();
        let half = attack_timer.duration().as_secs_f32() / 2.0;
        if !(elapsed >= half && elapsed < half + 0.05) {
            continue;
        }

        let delta = player_tf.translation.xy() - enemy_tf.translation.xy();
        if delta.length() > MELEE_RANGE {
            continue;
        }

        let direction_x = delta.x.signum();
        player_health.current -= ENEMY_ATTACK_DAMAGE;
        shakes.add_trauma(0.3);

        controller.action_interrupt(PlayerControlScheme::Knockback(TnuaBuiltinKnockback {
            shove: Vec3::new(direction_x * KNOCKBACK_STRENGTH, KNOCKBACK_UP, 0.0),
            ..default()
        }));

        commands
            .entity(player_entity)
            .insert(HitFlash(Timer::from_seconds(0.2, TimerMode::Once)));
        break;
    }
}

/// Tint the player yellow-white while attacking, red when hit.
fn attack_visual_feedback(
    mut player_query: Query<
        (&mut Sprite, Option<&Attacking>, Option<&HitFlash>),
        With<Player>,
    >,
) {
    let Ok((mut sprite, attacking, hit_flash)) = player_query.single_mut() else {
        return;
    };

    if hit_flash.is_some() {
        sprite.color = Color::srgb(1.0, 0.3, 0.3);
    } else if attacking.is_some() {
        sprite.color = Color::srgb(1.0, 1.0, 0.5);
    } else {
        sprite.color = Color::WHITE;
    }
}

/// Flash hit enemies red, then fade back to normal.
fn hit_flash_decay(
    time: Res<Time>,
    mut query: Query<(Entity, &mut Sprite, &mut HitFlash), With<Enemy>>,
    mut commands: Commands,
) {
    for (entity, mut sprite, mut flash) in &mut query {
        flash.tick(time.delta());
        if flash.is_finished() {
            sprite.color = Color::WHITE;
            commands.entity(entity).remove::<HitFlash>();
        } else {
            sprite.color = Color::srgb(1.0, 0.3, 0.3);
        }
    }
}
