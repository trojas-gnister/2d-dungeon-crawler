use bevy::prelude::*;

use crate::health::Health;
use crate::player::Player;

use super::Screen;

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::Gameplay), spawn_hud);
    app.add_systems(Update, update_health_bar.run_if(in_state(Screen::Gameplay)));
}

/// Marker for the health bar fill (the red inner bar that shrinks).
#[derive(Component)]
struct HealthBarFill;

fn spawn_hud(mut commands: Commands) {
    commands
        .spawn((
            DespawnOnExit(Screen::Gameplay),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(20.0),
                top: Val::Px(20.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                ..default()
            },
        ))
        .with_children(|parent| {
            // "HP" label
            parent.spawn((
                Text::new("HP"),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));

            // Health bar background (dark)
            parent
                .spawn((
                    Node {
                        width: Val::Px(200.0),
                        height: Val::Px(16.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
                ))
                .with_child((
                    // Health bar fill (red)
                    HealthBarFill,
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.8, 0.15, 0.15)),
                ));
        });
}

/// Update health bar width based on player health.
fn update_health_bar(
    player_query: Query<&Health, With<Player>>,
    mut bar_query: Query<&mut Node, With<HealthBarFill>>,
) {
    let Ok(health) = player_query.single() else {
        return;
    };
    let Ok(mut bar_node) = bar_query.single_mut() else {
        return;
    };

    bar_node.width = Val::Percent(health.fraction() * 100.0);
}
