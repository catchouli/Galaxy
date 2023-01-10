use miniquad::Context;

pub trait Drawable {
    fn update(&mut self, ctx: &mut Context);
    fn draw(&mut self, ctx: &mut Context);
}

pub trait DebugDrawable {
    fn debug_draw(&mut self, ctx: &mut Context);
}
