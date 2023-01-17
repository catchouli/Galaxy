/// A simple struct for storing input state, so that not everything has to hook into countless
/// messages to respond to input.
#[derive(Default)]
pub struct InputState {
    /// The difference in mousewheel movement this update.
    pub mouse_wheel_dy: f32,

    /// The current position of the mouse in window coordinates.
    pub mouse_pos: (f32, f32),

    /// The difference in mouse position since last update, if any.
    pub mouse_diff: (f32, f32),

    /// Whether the left mouse button is down.
    pub left_mouse_button_down: bool,

    /// Whether the right mouse button is down.
    pub right_mouse_button_down: bool,

    /// Whether the middle mouse button is down.
    pub middle_mouse_button_down: bool,
}
