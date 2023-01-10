mod shaders;
mod primitives;
mod types;
mod galaxy;
mod perlin_map;
mod drawable;
mod quadtree;
mod hilbert;

use std::error::Error;

use galaxy::Galaxy;
use miniquad::*;
use perlin_map::PerlinMap;
use drawable::Drawable;

/// The window width.
const WINDOW_WIDTH: i32 = 800;

/// The window height.
const WINDOW_HEIGHT: i32 = 800;

/// Whether to draw the perlin noise map.
const DRAW_PERLIN_MAP: bool = false;

/// The oddly named 'Stage', which is actually just an event handler that renders our application
/// via miniquad.
pub struct Stage {
    perlin_map: PerlinMap,
    galaxy: Galaxy,
}

impl Stage {
    pub fn new(ctx: &mut Context) -> Result<Stage, Box<dyn Error>> {
        // Create perlin map.
        let perlin_map = PerlinMap::new(ctx)?;

        // Create galaxy.
        let galaxy = Galaxy::new(ctx, &mut rand::thread_rng())?;

        Ok(Stage {
            perlin_map,
            galaxy,
        })
    }
}

impl EventHandler for Stage {
    fn update(&mut self, ctx: &mut Context) {
        // Update drawables.
        self.perlin_map.update(ctx);
        self.galaxy.update(ctx);
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
}

fn main() {
    // Initialize logging.
    env_logger::init();
    log::info!("Hello!");

    let mut offset: usize = 0;

    //print!("= [");
    //for i in 0..17 {
    //    let nodes: usize = 4_usize.pow(i);
    //    //println!("{}, {}", nodes, offset);
    //    print!("{}, ", offset);
    //    offset += nodes;
    //}

    //return;

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
