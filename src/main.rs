mod shaders;
mod primitives;
mod types;
mod galaxy;
mod perlin_map;
mod drawable;
mod quadtree;
mod hilbert;

use std::{error::Error, iter::repeat, time::Instant};

use galaxy::Galaxy;
use miniquad::*;
use perlin_map::PerlinMap;
use drawable::Drawable;
use rand::{rngs::StdRng, SeedableRng};

use crate::hilbert::HilbertIndex;

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
            self.perlin_map.update(ctx, FIXED_TIMESTEP);
            self.galaxy.update(ctx, FIXED_TIMESTEP);
        }
    }

    fn draw(&mut self, ctx: &mut Context) {
        ctx.begin_default_pass(Default::default());

        // Draw drawables.
        if DRAW_PERLIN_MAP {
            self.perlin_map.draw(ctx);
        }
        self.galaxy.draw(ctx);

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

    miniquad::start(config, |mut ctx| {
        Box::new(Stage::new(&mut ctx).unwrap())
    });
}
