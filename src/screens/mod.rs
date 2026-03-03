pub mod gameover;
pub mod gameplay;
pub mod title;

use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    app.init_state::<Screen>();
    app.add_plugins((title::plugin, gameplay::plugin, gameover::plugin));
}

/// Top-level game state.
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum Screen {
    #[default]
    Title,
    Gameplay,
    GameOver,
}
