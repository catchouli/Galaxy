use miniquad::Context;

mod textured_quad;
mod wireframe_quad;

pub use textured_quad::*;
pub use wireframe_quad::*;
use crate::input::InputState;

pub trait Drawable {
    fn update(&mut self, ctx: &mut Context, input_state: &InputState, time_delta: f64);
    fn draw(&mut self, ctx: &mut Context);

    /// Build this drawable's egui windows. Called once per frame.
    fn ui(&mut self, _egui_ctx: &egui::Context) {}
}

pub trait DebugDrawable {
    fn debug_draw(&mut self, ctx: &mut Context);
}
