use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

pub fn plugin(app: &mut App) {
    app.add_plugins(InputManagerPlugin::<PlayerAction>::default());
}

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect)]
pub enum PlayerAction {
    #[actionlike(Axis)]
    Move,
    Jump,
    Sprint,
    Attack,
}

impl PlayerAction {
    pub fn default_input_map() -> InputMap<Self> {
        InputMap::default()
            // Keyboard: horizontal movement
            .with_axis(Self::Move, VirtualAxis::ad())
            .with_axis(Self::Move, VirtualAxis::horizontal_arrow_keys())
            // Keyboard: actions
            .with(Self::Jump, KeyCode::Space)
            .with(Self::Sprint, KeyCode::ShiftLeft)
            .with(Self::Sprint, KeyCode::ShiftRight)
            .with(Self::Attack, KeyCode::KeyF)
            // Gamepad: movement
            .with_axis(Self::Move, GamepadControlAxis::LEFT_X)
            .with_axis(Self::Move, VirtualAxis::dpad_x())
            // Gamepad: actions
            .with(Self::Jump, GamepadButton::South)
            .with(Self::Sprint, GamepadButton::LeftTrigger2)
            .with(Self::Attack, GamepadButton::West)
    }
}
