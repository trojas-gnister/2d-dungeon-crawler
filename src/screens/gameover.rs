use bevy::prelude::*;

use crate::health::{BestScore, RunStats};

use super::Screen;

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::GameOver), spawn_gameover_ui);
    app.add_systems(
        Update,
        handle_gameover_buttons.run_if(in_state(Screen::GameOver)),
    );
}

#[derive(Component)]
struct RetryButton;

#[derive(Component)]
struct TitleButton;

fn spawn_gameover_ui(
    mut commands: Commands,
    run_stats: Option<Res<RunStats>>,
    best_score: Res<BestScore>,
) {
    let (height, kills, is_new_best) = run_stats
        .map(|s| (s.height_meters, s.enemies_killed, s.is_new_best))
        .unwrap_or((0, 0, false));

    commands
        .spawn((
            DespawnOnExit(Screen::GameOver),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(16.0),
                ..default()
            },
        ))
        .with_children(|parent| {
            // "GAME OVER" title
            parent.spawn((
                Text::new("GAME OVER"),
                TextFont {
                    font_size: 64.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.2, 0.2)),
            ));

            // Height reached (large)
            parent.spawn((
                Text::new(format!("{}m", height)),
                TextFont {
                    font_size: 48.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));

            // "NEW BEST!" — only visible when it's a new record
            parent.spawn((
                Text::new("NEW BEST!"),
                TextFont {
                    font_size: 32.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.85, 0.0)),
                Node {
                    display: if is_new_best {
                        Display::Flex
                    } else {
                        Display::None
                    },
                    ..default()
                },
            ));

            // Enemies killed
            parent.spawn((
                Text::new(format!("Enemies killed: {}", kills)),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
            ));

            // Best height
            parent.spawn((
                Text::new(format!("Best: {}m", best_score.height_meters)),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
            ));

            // Spacer
            parent.spawn(Node {
                height: Val::Px(20.0),
                ..default()
            });

            // Button row
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(24.0),
                    ..default()
                })
                .with_children(|row| {
                    // Retry button
                    row.spawn((
                        RetryButton,
                        Interaction::None,
                        Node {
                            padding: UiRect::axes(Val::Px(40.0), Val::Px(15.0)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.2, 0.6, 0.3)),
                    ))
                    .with_child((
                        Text::new("Retry"),
                        TextFont {
                            font_size: 30.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));

                    // Title button
                    row.spawn((
                        TitleButton,
                        Interaction::None,
                        Node {
                            padding: UiRect::axes(Val::Px(40.0), Val::Px(15.0)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.4, 0.4, 0.5)),
                    ))
                    .with_child((
                        Text::new("Title"),
                        TextFont {
                            font_size: 30.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
        });
}

fn handle_gameover_buttons(
    retry_q: Query<&Interaction, (Changed<Interaction>, With<RetryButton>)>,
    title_q: Query<&Interaction, (Changed<Interaction>, With<TitleButton>)>,
    mut next_state: ResMut<NextState<Screen>>,
) {
    for &interaction in &retry_q {
        if interaction == Interaction::Pressed {
            next_state.set(Screen::Gameplay);
        }
    }
    for &interaction in &title_q {
        if interaction == Interaction::Pressed {
            next_state.set(Screen::Title);
        }
    }
}
