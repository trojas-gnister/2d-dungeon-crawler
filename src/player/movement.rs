use bevy::prelude::*;
use bevy_spritesheet_animation::prelude::*;
use bevy_tnua::control_helpers::TnuaActionsCounter;
use bevy_tnua::prelude::*;
use leafwing_input_manager::prelude::*;

use crate::animation::CharacterAnims;
use crate::combat::Attacking;
use crate::input::PlayerAction;

use super::{
    Facing, Player, PlayerAirActions, PlayerControlScheme,
    PlayerControlSchemeActionDiscriminant,
};

const MOVE_SPEED: f32 = 110.0;
const SPRINT_MULTIPLIER: f32 = 1.6;

pub fn player_movement(
    anims: Res<CharacterAnims>,
    mut query: Query<
        (
            &ActionState<PlayerAction>,
            &mut TnuaController<PlayerControlScheme>,
            &TnuaActionsCounter<PlayerAirActions>,
            &mut Facing,
            &mut Sprite,
            &mut SpritesheetAnimation,
            Option<&Attacking>,
        ),
        With<Player>,
    >,
) {
    let Ok((action, mut controller, air_actions, mut facing, mut sprite, mut anim, attacking)) =
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

    // Feed jump while button held. Tnua's TnuaActionsCounter handles
    // air jump counting — count_for() returns 0 on ground, 1 for the
    // first air jump, etc. allow_in_air <= 1 means double jump.
    if !is_knockback && action.pressed(&PlayerAction::Jump) {
        controller.action(PlayerControlScheme::Jump(TnuaBuiltinJump {
            allow_in_air: air_actions
                .count_for(PlayerControlSchemeActionDiscriminant::Jump)
                <= 1,
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

/// Camera smoothly follows the player position with a vertical bias for climbing.
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

    // Offset camera upward so the player sees more of what's above
    let target = Vec3::new(
        player_tf.translation.x,
        player_tf.translation.y + 100.0,
        cam_tf.translation.z,
    );

    // Faster vertical lerp (0.12) vs horizontal (0.08) so camera keeps up while climbing
    let new_x = cam_tf.translation.x + (target.x - cam_tf.translation.x) * 0.08;
    let new_y = cam_tf.translation.y + (target.y - cam_tf.translation.y) * 0.12;
    cam_tf.translation.x = new_x;
    cam_tf.translation.y = new_y;
}
