use bevy::prelude::*;
use bevy_spritesheet_animation::prelude::*;
use bevy_tnua::prelude::*;
use leafwing_input_manager::prelude::*;

use crate::animation::CharacterAnims;
use crate::combat::Attacking;
use crate::input::PlayerAction;

use super::{Facing, Player, PlayerControlScheme, PlayerControlSchemeActionDiscriminant};

const MOVE_SPEED: f32 = 200.0;
const SPRINT_MULTIPLIER: f32 = 1.8;

pub fn player_movement(
    anims: Res<CharacterAnims>,
    mut query: Query<
        (
            &ActionState<PlayerAction>,
            &mut TnuaController<PlayerControlScheme>,
            &mut Facing,
            &mut Sprite,
            &mut SpritesheetAnimation,
            Option<&Attacking>,
        ),
        With<Player>,
    >,
) {
    let Ok((action, mut controller, mut facing, mut sprite, mut anim, attacking)) =
        query.single_mut()
    else {
        return;
    };

    // Disable player input during attack or knockback
    let is_knockback = controller.action_discriminant()
        == Some(PlayerControlSchemeActionDiscriminant::Knockback);
    let direction = if attacking.is_some() || is_knockback {
        0.0
    } else {
        action.clamped_value(&PlayerAction::Move)
    };

    let speed = if action.pressed(&PlayerAction::Sprint) {
        MOVE_SPEED * SPRINT_MULTIPLIER
    } else {
        MOVE_SPEED
    };

    // Always set the basis — Tnua needs this every frame
    controller.basis = TnuaBuiltinWalk {
        desired_motion: Vec3::new(direction * speed, 0.0, 0.0),
        ..default()
    };

    controller.initiate_action_feeding();

    // Feed jump while button held (variable-height via shorten_extra_gravity on release)
    if !is_knockback && action.pressed(&PlayerAction::Jump) {
        controller.action(PlayerControlScheme::Jump(TnuaBuiltinJump {
            allow_in_air: true,
            ..default()
        }));
    }

    // Update facing direction and flip sprite
    if direction < 0.0 {
        *facing = Facing::Left;
        sprite.flip_x = true;
    } else if direction > 0.0 {
        *facing = Facing::Right;
        sprite.flip_x = false;
    }

    // Switch animation: idle vs walk
    let target = if direction != 0.0 {
        &anims.walk
    } else {
        &anims.idle
    };
    if anim.animation != *target {
        anim.switch(target.clone());
    }
}

/// Camera smoothly follows the player position.
pub fn camera_follow(
    player: Query<&Transform, With<Player>>,
    mut camera: Query<&mut Transform, (With<Camera2d>, Without<Player>)>,
) {
    let Ok(player_tf) = player.single() else {
        return;
    };
    let Ok(mut cam_tf) = camera.single_mut() else {
        return;
    };

    let target = Vec3::new(
        player_tf.translation.x,
        player_tf.translation.y + 50.0,
        cam_tf.translation.z,
    );
    cam_tf.translation = cam_tf.translation.lerp(target, 0.1);
}
