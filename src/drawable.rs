use miniquad::Context;

mod textured_quad;
mod wireframe_quad;
mod imgui;

pub use textured_quad::*;
pub use wireframe_quad::*;
use crate::input::InputState;

pub use self::imgui::*;

pub trait Drawable {
    fn update(&mut self, ctx: &mut Context, ui: &mut ::imgui::Ui, input_state: &InputState, time_delta: f64);
    fn draw(&mut self, ctx: &mut Context, ui: &mut ::imgui::Ui);
}

pub trait DebugDrawable {
    fn debug_draw(&mut self, ctx: &mut Context);
}
