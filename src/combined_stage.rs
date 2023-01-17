use miniquad::EventHandler;

/// A simple helper struct that lets you combine stages and execute them in sequence.
pub struct CombinedStage {
    stages: Vec<Box<dyn EventHandler>>,
}

impl CombinedStage {
    /// Create a new CombinedStage from the provided list of stages.
    pub fn new(stages: Vec<Box<dyn EventHandler>>) -> Self {
        Self {
            stages,
        }
    }
}

impl EventHandler for CombinedStage {
    fn update(&mut self, ctx: &mut miniquad::Context) {
        for stage in &mut self.stages {
            stage.update(ctx);
        }
    }

    fn draw(&mut self, ctx: &mut miniquad::Context) {
        for stage in &mut self.stages {
            stage.draw(ctx);
        }
    }

    fn char_event(&mut self,
                  ctx: &mut miniquad::Context,
                  character: char,
                  keymods: miniquad::KeyMods,
                  repeat: bool)
    {
        for stage in &mut self.stages {
            stage.char_event(ctx, character, keymods, repeat);
        }
    }

    fn touch_event(&mut self,
                   ctx: &mut miniquad::Context,
                   phase: miniquad::TouchPhase,
                   id: u64, x: f32, y: f32)
    {
        for stage in &mut self.stages {
            stage.touch_event(ctx, phase, id, x, y);
        }
    }

    fn resize_event(&mut self, ctx: &mut miniquad::Context, width: f32, height: f32) {
        for stage in &mut self.stages {
            stage.resize_event(ctx, width, height);
        }
    }

    fn key_up_event(&mut self,
                    ctx: &mut miniquad::Context,
                    keycode: miniquad::KeyCode,
                    keymods: miniquad::KeyMods)
    {
        for stage in &mut self.stages {
            stage.key_up_event(ctx, keycode, keymods);
        }
    }

    fn key_down_event(&mut self,
                      ctx: &mut miniquad::Context,
                      keycode: miniquad::KeyCode,
                      keymods: miniquad::KeyMods,
                      repeat: bool)
    {
        for stage in &mut self.stages {
            stage.key_down_event(ctx, keycode, keymods, repeat);
        }
    }

    fn raw_mouse_motion(&mut self, ctx: &mut miniquad::Context, dx: f32, dy: f32) {
        for stage in &mut self.stages {
            stage.raw_mouse_motion(ctx, dx, dy);
        }
    }

    fn mouse_wheel_event(&mut self, ctx: &mut miniquad::Context, x: f32, y: f32) {
        for stage in &mut self.stages {
            stage.mouse_wheel_event(ctx, x, y);
        }
    }

    fn mouse_motion_event(&mut self, ctx: &mut miniquad::Context, x: f32, y: f32) {
        for stage in &mut self.stages {
            stage.mouse_motion_event(ctx, x, y);
        }
    }

    fn files_dropped_event(&mut self, ctx: &mut miniquad::Context) {
        for stage in &mut self.stages {
            stage.files_dropped_event(ctx);
        }
    }

    fn quit_requested_event(&mut self, ctx: &mut miniquad::Context) {
        for stage in &mut self.stages {
            stage.quit_requested_event(ctx);
        }
    }

    fn mouse_button_up_event(&mut self,
                             ctx: &mut miniquad::Context,
                             button: miniquad::MouseButton,
                             x: f32,
                             y: f32)
    {
        for stage in &mut self.stages {
            stage.mouse_button_up_event(ctx, button, x, y);
        }
    }

    fn window_restored_event(&mut self, ctx: &mut miniquad::Context) {
        for stage in &mut self.stages {
            stage.window_restored_event(ctx);
        }
    }

    fn window_minimized_event(&mut self, ctx: &mut miniquad::Context) {
        for stage in &mut self.stages {
            stage.window_minimized_event(ctx);
        }
    }

    fn mouse_button_down_event(&mut self,
                               ctx: &mut miniquad::Context,
                               button: miniquad::MouseButton,
                               x: f32, y: f32)
    {
        for stage in &mut self.stages {
            stage.mouse_button_down_event(ctx, button, x, y);
        }
    }
}
