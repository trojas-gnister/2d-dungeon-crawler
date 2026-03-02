pub mod movement;

use avian2d::prelude::*;
use bevy::prelude::*;
use bevy_spritesheet_animation::prelude::*;
use bevy_tnua::builtins::{
    TnuaBuiltinJumpConfig, TnuaBuiltinKnockback, TnuaBuiltinKnockbackConfig,
    TnuaBuiltinWalkConfig,
};
use bevy_tnua::control_helpers::{TnuaActionSlots, TnuaAirActionsPlugin};
use bevy_tnua::prelude::*;
use bevy_tnua_avian2d::prelude::*;

use crate::animation::CharacterAnims;
use crate::health::Health;
use crate::input::PlayerAction;
use crate::screens::Screen;

/// Tnua control scheme: walk basis + jump/knockback actions.
#[derive(TnuaScheme)]
#[scheme(basis = TnuaBuiltinWalk)]
pub enum PlayerControlScheme {
    Jump(TnuaBuiltinJump),
    Knockback(TnuaBuiltinKnockback),
}

/// Defines air action counting slots for double jump.
#[derive(TnuaActionSlots)]
#[slots(scheme = PlayerControlScheme)]
pub struct PlayerAirActions {
    #[slots(Jump)]
    jump: usize,
}

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::Gameplay), spawn_player);
    app.add_systems(
        FixedUpdate,
        movement::player_movement.run_if(in_state(Screen::Gameplay)),
    );
    app.add_systems(
        Update,
        movement::camera_follow.run_if(in_state(Screen::Gameplay)),
    );
    // Air action counting for double jump
    app.add_plugins(TnuaAirActionsPlugin::<PlayerAirActions>::new(FixedUpdate));
}

/// Marker component identifying the player entity.
#[derive(Component)]
pub struct Player;

/// Which direction the player is facing.
#[derive(Component, PartialEq, Eq)]
pub enum Facing {
    Left,
    Right,
}

fn spawn_player(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut configs: ResMut<Assets<PlayerControlSchemeConfig>>,
    anims: Res<CharacterAnims>,
) {
    let texture: Handle<Image> = asset_server.load("sprites/player.png");
    let layout = TextureAtlasLayout::from_grid(UVec2::splat(32), 10, 1, None, None);
    let layout_handle = layouts.add(layout);

    let config = configs.add(PlayerControlSchemeConfig {
        basis: TnuaBuiltinWalkConfig {
            speed: 150.0,
            float_height: 17.0,
            acceleration: 900.0,
            air_acceleration: 400.0,
            coyote_time: 0.10,
            free_fall_extra_gravity: 800.0,
            spring_strength: 600.0,
            spring_dampening: 1.8,
            cling_distance: 4.0,
            ..default()
        },
        jump: TnuaBuiltinJumpConfig {
            height: 140.0,
            input_buffer_time: 0.15,
            takeoff_extra_gravity: 400.0,
            takeoff_above_velocity: 150.0,
            fall_extra_gravity: 600.0,
            shorten_extra_gravity: 1400.0,
            peak_prevention_at_upward_velocity: 80.0,
            peak_prevention_extra_gravity: 400.0,
            ..default()
        },
        knockback: TnuaBuiltinKnockbackConfig {
            no_push_timeout: 0.3,
            ..default()
        },
    });

    commands
        .spawn((
            Player,
            Facing::Right,
            Name::new("Player"),
            DespawnOnExit(Screen::Gameplay),
            Sprite::from_atlas_image(
                texture,
                TextureAtlas {
                    layout: layout_handle,
                    index: 0,
                },
            ),
            Transform::from_xyz(0.0, -200.0, 1.0),
            SpritesheetAnimation::new(anims.idle.clone()),
        ))
        .insert((
            RigidBody::Dynamic,
            Collider::rectangle(32.0, 32.0),
            LockedAxes::ROTATION_LOCKED,
            Health::new(100.0),
            PlayerAction::default_input_map(),
            TnuaController::<PlayerControlScheme>::default(),
            TnuaConfig::<PlayerControlScheme>(config),
            TnuaAvian2dSensorShape(Collider::rectangle(30.0, 0.0)),
        ));
}
