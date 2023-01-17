mod shaders;
mod types;
mod galaxy;
mod perlin_map;
mod drawable;
mod quadtree;
mod hilbert;
mod combined_stage;
mod input;

use std::cell::RefCell;
use std::rc::Rc;
use std::{error::Error, iter::repeat, time::Instant};

use galaxy::Galaxy;
use miniquad::*;
use owning_ref::OwningRefMut;
use perlin_map::PerlinMap;
use rand::{rngs::StdRng, SeedableRng};

use crate::hilbert::HilbertIndex;
use crate::combined_stage::CombinedStage;
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
    imgui: Rc<RefCell<OwningRefMut<Box<imgui::Context>, imgui::Ui>>>,
    input_state: InputState,
}

impl Stage {
    pub fn new(ctx: &mut Context, imgui: Rc<RefCell<OwningRefMut<Box<imgui::Context>, imgui::Ui>>>) -> Result<Stage, Box<dyn Error>> {
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
            imgui,
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

impl<'a> EventHandler for Stage {
    fn update(&mut self, ctx: &mut Context) {
        let mut imgui = self.imgui.borrow_mut();

        // Update timer.
        let time_since_start = self.start_time.elapsed().as_secs_f64();

        if self.sim_time + FIXED_TIMESTEP < time_since_start {
            self.sim_time += FIXED_TIMESTEP;

            // Update drawables.
            self.perlin_map.update(ctx, imgui.as_mut(), &self.input_state, FIXED_TIMESTEP);
            self.galaxy.update(ctx, imgui.as_mut(), &self.input_state, FIXED_TIMESTEP);

            // Clear relative moevments from input state.
            self.input_state.mouse_diff = (0.0, 0.0);
            self.input_state.mouse_wheel_dy = 0.0;
        }
    }

    fn draw(&mut self, ctx: &mut Context) {
        ctx.begin_default_pass(Default::default());

        let mut imgui = self.imgui.borrow_mut();

        // Draw drawables.
        if DRAW_PERLIN_MAP {
            self.perlin_map.draw(ctx, imgui.as_mut());
        }
        self.galaxy.draw(ctx, imgui.as_mut());

        ctx.end_render_pass();
        ctx.commit_frame();
    }

    fn key_down_event(&mut self, ctx: &mut Context, keycode: KeyCode, _keymods: KeyMods, _repeat: bool) {
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

    fn mouse_wheel_event(&mut self, _ctx: &mut Context, _x: f32, y: f32) {
        self.input_state.mouse_wheel_dy += y;
    }

    fn mouse_motion_event(&mut self, _ctx: &mut Context, x: f32, y: f32) {
        let (old_x, old_y) = self.input_state.mouse_pos;
        let (cur_dx, cur_dy) = self.input_state.mouse_diff;

        self.input_state.mouse_pos = (x, y);
        self.input_state.mouse_diff = (cur_dx + (x - old_x), cur_dy + (y - old_y));
    }

    fn mouse_button_up_event(&mut self, _ctx: &mut Context, button: MouseButton, _x: f32, _y: f32) {
        let button_state = match button {
            MouseButton::Left => &mut self.input_state.left_mouse_button_down,
            MouseButton::Right => &mut self.input_state.right_mouse_button_down,
            _ => &mut self.input_state.middle_mouse_button_down,
        };
        *button_state = false;
    }

    fn mouse_button_down_event(&mut self, _ctx: &mut Context, button: MouseButton, _x: f32, _y: f32) {
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

    miniquad::start(config, |mut ctx: &mut GraphicsContext| {
        let mut imgui_renderer = drawable::ImguiRenderer::new(&mut ctx);

        Box::new(CombinedStage::new(vec![
            Box::new(Stage::new(&mut ctx, imgui_renderer.ui()).unwrap()),
            Box::new(imgui_renderer),
        ]))
    });
}
