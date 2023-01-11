use std::error::Error;

use miniquad::*;
use rand::Rng;
use crate::primitives::TexturedQuad;
use crate::types::Vec2;
use crate::drawable::{Drawable, DebugDrawable};
use crate::quadtree::{Quadtree, Spatial, QuadtreeNode};

/// The texture width.
const TEX_WIDTH: usize = 1024;

/// The texture height.
const TEX_HEIGHT: usize = 1024;

/// The number of stars.
const STAR_COUNT: usize = 1000;

/// A single star.
struct Star {
    position: Vec2,
}

impl Spatial for Star {
    fn xy(&self) -> &Vec2 {
        &self.position
    }
}

/// A structure representing the rendering of a Galaxy. For now this includes both the simulation
/// and rendering logic, but it would be nice to separate them.
pub struct Galaxy {
    textured_quad: TexturedQuad,
    texture_dirty: bool,
    quadtree: Quadtree<Star>,
}

impl Galaxy {
    /// Create a new galaxy that renders via the given miniquad context.
    pub fn new<R: Rng + ?Sized>(ctx: &mut Context, rng: &mut R) -> Result<Self, Box<dyn Error>> {
        // Create textured quad for drawing stars.
        let textured_quad = TexturedQuad::new(ctx, TEX_WIDTH, TEX_HEIGHT)?;

        // Create quadtree.
        let mut quadtree = Quadtree::new(Vec2::new(-1.0, -1.0), Vec2::new(1.0, 1.0))?;

        // Generate stars.
        for _ in 0..STAR_COUNT {
            let position = Vec2::new(rng.gen_range(-1.0..1.0), rng.gen_range(-1.0..1.0));
            quadtree.add(Star { position });
        }

        Ok(Self {
            textured_quad,
            texture_dirty: true,
            quadtree,
        })
    }

    /// Update the texture if the dirty flag is set.
    pub fn update_texture(&mut self, ctx: &mut Context) {
        if self.texture_dirty {
            log::info!("Updating star texture");

            self.texture_dirty = false;

            // Create new buffer.
            let mut bytes = vec![0; 4 * TEX_WIDTH * TEX_HEIGHT];

            // Fill buffer.
            self.quadtree.walk_nodes(|_, node| {
                match node {
                    QuadtreeNode::Leaf(star) => {
                        // Check that the star is within the texture.
                        let pos = star.position;
                        if pos.x > -1.0 && pos.x < 1.0 as f32 && pos.y > -1.0 && pos.y < 1.0 as f32 {
                            // Convert star position to x and y in texture.
                            let x = ((pos.x / 2.0 + 0.5) * TEX_WIDTH as f32) as usize;
                            let y = TEX_HEIGHT - ((pos.y / 2.0 + 0.5) * TEX_HEIGHT as f32) as usize;

                            // Get index and slice of pixel, *4 because the texture is 4 bytes per pixel.
                            let idx = 4 * (y * TEX_WIDTH + x);
                            let pixel = &mut bytes[idx..idx+4];

                            pixel[0] = 0xFF;
                            pixel[1] = 0xFF;
                            pixel[2] = 0xFF;
                            pixel[3] = 0xFF;
                        }
                    },
                    _ => {},
                }
            });

            // Update texture.
            self.textured_quad.texture.update(ctx, &bytes);
        }
    }
}

impl Drawable for Galaxy {
    /// Update the galaxy.
    fn update(&mut self, _ctx: &mut Context) {
    }

    /// Draw the galaxy.
    fn draw(&mut self, ctx: &mut Context) {
        self.update_texture(ctx);
        self.textured_quad.draw(ctx);
        self.quadtree.debug_draw(ctx);
    }
}
