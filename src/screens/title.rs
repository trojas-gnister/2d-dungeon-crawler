use bevy::prelude::*;

use super::Screen;

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::Title), spawn_title_ui);
    app.add_systems(Update, handle_play_button.run_if(in_state(Screen::Title)));
}

/// Marker for the Play button so we can query interactions.
#[derive(Component)]
struct PlayButton;

fn spawn_title_ui(mut commands: Commands) {
    commands
        .spawn((
            DespawnOnExit(Screen::Title),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(20.0),
                ..default()
            },
        ))
        .with_children(|parent| {
            // Title text
            parent.spawn((
                Text::new("2D Combat"),
                TextFont {
                    font_size: 60.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));

            // Play button — a clickable Node with Interaction
            parent
                .spawn((
                    PlayButton,
                    Interaction::None,
                    Node {
                        padding: UiRect::axes(Val::Px(40.0), Val::Px(15.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.2, 0.5, 0.8)),
                ))
                .with_child((
                    Text::new("Play"),
                    TextFont {
                        font_size: 30.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
        });
}

fn handle_play_button(
    interaction: Query<&Interaction, (Changed<Interaction>, With<PlayButton>)>,
    mut next_state: ResMut<NextState<Screen>>,
) {
    for &interaction in &interaction {
        if interaction == Interaction::Pressed {
            next_state.set(Screen::Gameplay);
        }
    }
}
