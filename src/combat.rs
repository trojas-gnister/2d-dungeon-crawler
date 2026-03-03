use std::f32::consts::PI;

use bevy::prelude::*;
use bevy_tnua::builtins::TnuaBuiltinKnockback;
use bevy_tnua::prelude::*;
use bevy_trauma_shake::prelude::*;
use leafwing_input_manager::prelude::*;

use avian2d::prelude::*;

use crate::enemy::{Enemy, EnemyAttackTimer, EnemyKind, EnemyState, FlyingState};
use crate::health::{EnemyKillCount, Health};
use crate::input::PlayerAction;
use crate::player::{
    Facing, Player, PlayerControlScheme, PlayerControlSchemeActionDiscriminant,
};
use crate::screens::Screen;

const FLYER_CONTACT_RANGE: f32 = 30.0;
const FLYER_CONTACT_DAMAGE: f32 = 10.0;

// Sword arc visual constants
const SWORD_ARC_OFFSET_X: f32 = 24.0;
const SWORD_ARC_OFFSET_Y: f32 = 8.0;
const SWORD_ARC_Z: f32 = 0.5;
const SWORD_ARC_WIDTH: f32 = 40.0;
const SWORD_ARC_HEIGHT: f32 = 6.0;
// Right-facing swing: start at +45° (blade up), end at -90° (blade down)
const SWING_START_RIGHT: f32 = PI / 4.0;
const SWING_END_RIGHT: f32 = -PI / 2.0;
// Left-facing swing: mirrored
const SWING_START_LEFT: f32 = 3.0 * PI / 4.0;
const SWING_END_LEFT: f32 = 3.0 * PI / 2.0;

// Enemy knockback constants
const ENEMY_KNOCKBACK_STRENGTH: f32 = 280.0;
const ENEMY_KNOCKBACK_UP: f32 = 180.0;
const ENEMY_STAGGER_DURATION: f32 = 0.35;
const FLYER_KNOCKBACK_STRENGTH: f32 = 120.0;
const FLYER_KNOCKBACK_UP: f32 = 60.0;
const FLYER_STAGGER_DURATION: f32 = 0.25;

pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            player_attack_input,
            spawn_sword_arc,
            player_attack_tick,
            animate_sword_arc,
            player_hit_detection,
            stagger_tick,
            enemy_hit_player,
            flying_enemy_contact_damage,
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

/// Marker for the sword slash visual child entity.
#[derive(Component)]
pub struct SwordArc;

/// Pauses AI-driven velocity for its duration (enemy is staggering from a hit).
#[derive(Component, Deref, DerefMut)]
pub struct Staggered(pub Timer);

/// Velocity override for flying enemies during stagger (they don't use LinearVelocity).
#[derive(Component)]
pub struct FlyingKnockbackVelocity(pub Vec2);

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

/// Spawn a child sword arc sprite when the player starts attacking.
fn spawn_sword_arc(
    player_query: Query<(Entity, &Facing), (With<Player>, With<Attacking>)>,
    existing_arcs: Query<&ChildOf, With<SwordArc>>,
    mut commands: Commands,
) {
    let Ok((player_entity, facing)) = player_query.single() else {
        return;
    };

    // Don't spawn a second arc if one already exists as our child
    let already_has_arc = existing_arcs
        .iter()
        .any(|child_of| child_of.parent() == player_entity);
    if already_has_arc {
        return;
    }

    let (offset_x, rotation) = match facing {
        Facing::Right => (SWORD_ARC_OFFSET_X, SWING_START_RIGHT),
        Facing::Left => (-SWORD_ARC_OFFSET_X, SWING_START_LEFT),
    };

    commands.entity(player_entity).with_child((
        SwordArc,
        Sprite {
            color: Color::srgba(1.0, 0.9, 0.3, 0.75),
            custom_size: Some(Vec2::new(SWORD_ARC_WIDTH, SWORD_ARC_HEIGHT)),
            ..default()
        },
        Transform::from_xyz(offset_x, SWORD_ARC_OFFSET_Y, SWORD_ARC_Z)
            .with_rotation(Quat::from_rotation_z(rotation)),
    ));
}

/// Animate the sword arc rotation and alpha over the attack duration.
fn animate_sword_arc(
    player_query: Query<(&Attacking, &Facing), With<Player>>,
    mut arc_query: Query<(&mut Transform, &mut Sprite), With<SwordArc>>,
) {
    let Ok((attacking, facing)) = player_query.single() else {
        return;
    };

    let fraction = attacking.fraction();
    let (start, end) = match facing {
        Facing::Right => (SWING_START_RIGHT, SWING_END_RIGHT),
        Facing::Left => (SWING_START_LEFT, SWING_END_LEFT),
    };

    let angle = start + (end - start) * fraction;
    // Fade out alpha in the second half of the swing
    let alpha = if fraction > 0.5 {
        1.0 - (fraction - 0.5) * 2.0
    } else {
        0.75
    };

    for (mut tf, mut sprite) in &mut arc_query {
        tf.rotation = Quat::from_rotation_z(angle);
        sprite.color = Color::srgba(1.0, 0.9, 0.3, alpha);
    }
}

/// Tick the attack timer and clean up when it expires.
fn player_attack_tick(
    time: Res<Time>,
    mut query: Query<(Entity, &mut Attacking), With<Player>>,
    hit_markers: Query<Entity, With<HitByCurrentSwing>>,
    sword_arcs: Query<Entity, With<SwordArc>>,
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
        for arc_entity in &sword_arcs {
            commands.entity(arc_entity).despawn();
        }
    }
}

/// Check if any enemies are in range and in front of the player, then deal damage + knockback.
fn player_hit_detection(
    player_query: Query<(&Transform, &Facing), (With<Player>, With<Attacking>)>,
    mut enemy_query: Query<
        (
            Entity,
            &Transform,
            &mut Health,
            Option<&mut LinearVelocity>,
            Option<&EnemyKind>,
        ),
        (With<Enemy>, Without<HitByCurrentSwing>),
    >,
    mut commands: Commands,
    mut shakes: Shakes,
    mut kill_count: Option<ResMut<EnemyKillCount>>,
) {
    let Ok((player_tf, facing)) = player_query.single() else {
        return;
    };

    let push_dir_x: f32 = match facing {
        Facing::Right => 1.0,
        Facing::Left => -1.0,
    };

    for (enemy_entity, enemy_tf, mut health, velocity, kind) in &mut enemy_query {
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

        if health.is_dead() {
            if let Some(ref mut count) = kill_count {
                count.0 += 1;
            }
        }

        shakes.add_trauma(0.15);

        // Apply knockback based on enemy type
        let is_flyer = matches!(kind, Some(EnemyKind::Flying));
        if is_flyer {
            commands.entity(enemy_entity).insert((
                Staggered(Timer::from_seconds(FLYER_STAGGER_DURATION, TimerMode::Once)),
                FlyingKnockbackVelocity(Vec2::new(
                    push_dir_x * FLYER_KNOCKBACK_STRENGTH,
                    FLYER_KNOCKBACK_UP,
                )),
            ));
        } else {
            if let Some(mut vel) = velocity {
                vel.x = push_dir_x * ENEMY_KNOCKBACK_STRENGTH;
                vel.y = ENEMY_KNOCKBACK_UP;
            }
            commands.entity(enemy_entity).insert(Staggered(Timer::from_seconds(
                ENEMY_STAGGER_DURATION,
                TimerMode::Once,
            )));
        }

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

/// During swoop, flying enemies deal contact damage if close to player.
fn flying_enemy_contact_damage(
    flyer_query: Query<(&Transform, &FlyingState), With<Enemy>>,
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

    if player_health.is_dead() {
        return;
    }

    // Don't stack knockbacks
    if controller.action_discriminant()
        == Some(PlayerControlSchemeActionDiscriminant::Knockback)
    {
        return;
    }

    for (enemy_tf, fly_state) in &flyer_query {
        if *fly_state != FlyingState::Swoop {
            continue;
        }

        let delta = player_tf.translation.xy() - enemy_tf.translation.xy();
        if delta.length() > FLYER_CONTACT_RANGE {
            continue;
        }

        let direction_x = delta.x.signum();
        player_health.current -= FLYER_CONTACT_DAMAGE;
        shakes.add_trauma(0.2);

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

/// Tick stagger timers on enemies; remove Staggered + FlyingKnockbackVelocity when expired.
fn stagger_tick(
    time: Res<Time>,
    mut query: Query<(Entity, &mut Staggered)>,
    mut commands: Commands,
) {
    for (entity, mut stagger) in &mut query {
        stagger.tick(time.delta());
        if stagger.is_finished() {
            commands
                .entity(entity)
                .remove::<(Staggered, FlyingKnockbackVelocity)>();
        }
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

/// Flash hit enemies red, then fade back to normal (purple for flyers, white for ground).
fn hit_flash_decay(
    time: Res<Time>,
    mut query: Query<(Entity, &mut Sprite, &mut HitFlash, Option<&EnemyKind>), With<Enemy>>,
    mut commands: Commands,
) {
    for (entity, mut sprite, mut flash, kind) in &mut query {
        flash.tick(time.delta());
        if flash.is_finished() {
            sprite.color = match kind {
                Some(EnemyKind::Flying) => Color::srgb(0.7, 0.4, 1.0),
                _ => Color::WHITE,
            };
            commands.entity(entity).remove::<HitFlash>();
        } else {
            sprite.color = Color::srgb(1.0, 0.3, 0.3);
        }
    }
}
