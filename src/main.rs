mod shaders;
mod types;
mod galaxy;
mod perlin_map;
mod drawable;
mod quadtree;
mod hilbert;
mod input;

use std::{error::Error, iter::repeat, time::Instant};

use galaxy::Galaxy;
use miniquad::*;
use perlin_map::PerlinMap;
use rand::{rngs::StdRng, SeedableRng};

use crate::hilbert::HilbertIndex;
use crate::drawable::Drawable;
use crate::input::InputState;

/// The window width.
const WINDOW_WIDTH: i32 = 1024;

/// The window height.
const WINDOW_HEIGHT: i32 = 1024;

/// The fixed timestep, each update will account for this many seconds of simulation.
const FIXED_TIMESTEP: f64 = 1.0 / 60.0;

/// Whether to draw the perlin noise map.
const DRAW_PERLIN_MAP: bool = false;

/// The oddly named 'Stage', which is actually just an event handler that renders our application
/// via miniquad.
pub struct Stage {
    perlin_map: PerlinMap,
    galaxy: Galaxy,
    seed: u64,
    start_time: Instant,
    sim_time: f64,
    egui_mq: egui_miniquad::EguiMq,
    input_state: InputState,
}

impl Stage {
    pub fn new(ctx: &mut Context) -> Result<Stage, Box<dyn Error>> {
        let start_time = Instant::now();

        // Create perlin map.
        let perlin_map = PerlinMap::new(ctx)?;

        // Create galaxy.
        let seed = 152;
        let galaxy = Self::generate_galaxy(ctx, seed)?;

        Ok(Stage {
            perlin_map,
            galaxy,
            seed,
            start_time,
            sim_time: start_time.elapsed().as_secs_f64(),
            egui_mq: egui_miniquad::EguiMq::new(ctx),
            input_state: Default::default(),
        })
    }

    fn generate_galaxy(ctx: &mut Context, seed: u64) -> Result<Galaxy, Box<dyn Error>> {
        log::info!("Generating galaxy with seed {seed}");

        let mut rng = StdRng::seed_from_u64(seed);
        let galaxy = Galaxy::new(ctx, &mut rng)?;

        // Print out quadtree for debugging.
        galaxy.quadtree.walk_nodes(|index@HilbertIndex(_, depth), node| {
            let indentation: String = repeat(' ').take(depth as usize * 2).collect();
            log::debug!("{indentation}{index:?} {node:?}");
        });

        Ok(galaxy)
    }
}

impl EventHandler for Stage {
    fn update(&mut self, ctx: &mut Context) {
        // Update timer.
        let time_since_start = self.start_time.elapsed().as_secs_f64();

        if self.sim_time + FIXED_TIMESTEP < time_since_start {
            self.sim_time += FIXED_TIMESTEP;

            // Update drawables.
            self.perlin_map.update(ctx, &self.input_state, FIXED_TIMESTEP);
            self.galaxy.update(ctx, &self.input_state, FIXED_TIMESTEP);

            // Clear relative movements from input state.
            self.input_state.mouse_diff = (0.0, 0.0);
            self.input_state.mouse_wheel_dy = 0.0;
        }
    }

    fn draw(&mut self, ctx: &mut Context) {
        // Build the UI for this frame.
        let galaxy = &mut self.galaxy;
        self.egui_mq.run(ctx, |_ctx, egui_ctx| {
            galaxy.ui(egui_ctx);
        });

        ctx.begin_default_pass(Default::default());

        // Draw drawables.
        if DRAW_PERLIN_MAP {
            self.perlin_map.draw(ctx);
        }
        self.galaxy.draw(ctx);

        ctx.end_render_pass();

        // Draw the UI on top.
        self.egui_mq.draw(ctx);

        ctx.commit_frame();
    }

    fn char_event(&mut self, _ctx: &mut Context, character: char, _keymods: KeyMods, _repeat: bool) {
        self.egui_mq.char_event(character);
    }

    fn key_up_event(&mut self, _ctx: &mut Context, keycode: KeyCode, keymods: KeyMods) {
        self.egui_mq.key_up_event(keycode, keymods);
    }

    fn key_down_event(&mut self, ctx: &mut Context, keycode: KeyCode, keymods: KeyMods, _repeat: bool) {
        self.egui_mq.key_down_event(ctx, keycode, keymods);

        // Leave keyboard input to egui while it's using it, e.g. for typing into a slider.
        if self.egui_mq.egui_ctx().wants_keyboard_input() {
            return;
        }

        if keycode == KeyCode::Escape {
            ctx.quit();
        }
        else if keycode == KeyCode::Space {
            log::info!("Key pressed, regenerating galaxy");
            self.seed += 1;
            self.galaxy = Self::generate_galaxy(ctx, self.seed).unwrap();
        }
        else if keycode == KeyCode::M {
            self.galaxy.time_scale *= 10.0;
        }
        else if keycode == KeyCode::A {
            self.galaxy.time_scale /= 10.0;
        }
    }

    fn mouse_wheel_event(&mut self, _ctx: &mut Context, x: f32, y: f32) {
        self.egui_mq.mouse_wheel_event(x, y);

        if !self.egui_mq.egui_ctx().is_pointer_over_area() {
            self.input_state.mouse_wheel_dy += y;
        }
    }

    fn mouse_motion_event(&mut self, _ctx: &mut Context, x: f32, y: f32) {
        self.egui_mq.mouse_motion_event(x, y);

        let (old_x, old_y) = self.input_state.mouse_pos;
        let (cur_dx, cur_dy) = self.input_state.mouse_diff;

        self.input_state.mouse_pos = (x, y);
        self.input_state.mouse_diff = (cur_dx + (x - old_x), cur_dy + (y - old_y));
    }

    fn mouse_button_up_event(&mut self, ctx: &mut Context, button: MouseButton, x: f32, y: f32) {
        self.egui_mq.mouse_button_up_event(ctx, button, x, y);

        let button_state = match button {
            MouseButton::Left => &mut self.input_state.left_mouse_button_down,
            MouseButton::Right => &mut self.input_state.right_mouse_button_down,
            _ => &mut self.input_state.middle_mouse_button_down,
        };
        *button_state = false;
    }

    fn mouse_button_down_event(&mut self, ctx: &mut Context, button: MouseButton, x: f32, y: f32) {
        self.egui_mq.mouse_button_down_event(ctx, button, x, y);

        // Clicks on the UI shouldn't also move the camera.
        if self.egui_mq.egui_ctx().is_pointer_over_area() {
            return;
        }

        let button_state = match button {
            MouseButton::Left => &mut self.input_state.left_mouse_button_down,
            MouseButton::Right => &mut self.input_state.right_mouse_button_down,
            _ => &mut self.input_state.middle_mouse_button_down,
        };
        *button_state = true;
    }
}

fn main() {
    // Initialize logging.
    env_logger::init();
    log::info!("Hello!");

    // Create window config.
    let config = conf::Conf {
        window_title: "Galaxy".to_owned(),
        window_width: WINDOW_WIDTH,
        window_height: WINDOW_HEIGHT,
        ..Default::default()
    };

    miniquad::start(config, |ctx: &mut GraphicsContext| {
        Box::new(Stage::new(ctx).unwrap())
    });
}
