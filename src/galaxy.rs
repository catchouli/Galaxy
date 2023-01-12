use std::error::Error;

use miniquad::*;
use rand::Rng;
use crate::hilbert::HilbertIndex;
use crate::primitives::TexturedQuad;
use crate::types::Vec2;
use crate::drawable::{Drawable, DebugDrawable};
use crate::quadtree::{Quadtree, Spatial, QuadtreeNode};

/// The texture width.
const TEX_WIDTH: usize = 256;

/// The texture height.
const TEX_HEIGHT: usize = 256;

/// The number of stars.
const STAR_COUNT: usize = 10;

/// The mass of each star.
const STAR_MASS: f32 = 1.0;

/// A single star in our galaxy.
pub struct Star {
    position: Vec2,
    mass: f32,
}

impl Spatial for Star {
    fn xy(&self) -> &Vec2 {
        &self.position
    }
}

/// A region in our galaxy, in the quadtree. We use this to accelerate n-body calculations.
pub struct Region {
    center_of_mass: Vec2,
    mass: f32,
}

/// A structure representing the rendering of a Galaxy. For now this includes both the simulation
/// and rendering logic, but it would be nice to separate them.
pub struct Galaxy {
    textured_quad: TexturedQuad,
    texture_dirty: bool,

    /// The galaxy's quadtree. We store the stars as leaf nodes in the octree, and have an
    /// additional type Region for the internal nodes, which we use to accelerate n-body lookups.
    /// It's wrapped in an Option so it can be initialised lazily.
    pub quadtree: Quadtree<Star, Option<Region>>,
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
            log::debug!("Adding star at position: {position:?}");
            quadtree.add(Star { position, mass: STAR_MASS });
        }

        // Update mass distribution.
        Self::update_mass_distribution(&mut quadtree);

        Ok(Self {
            textured_quad,
            texture_dirty: true,
            quadtree,
        })
    }

    pub fn update_mass_distribution(quadtree: &mut Quadtree<Star, Option<Region>>) {
        // Update mass distributions recursively. We only need to do this if the root node is an
        // internal node. If it's a leaf node then nothing needs doing, if it's empty then nothing
        // needs doing.
        let root_index = HilbertIndex(0, 0);
        if quadtree.get(root_index).unwrap_or(&QuadtreeNode::Empty).is_internal() {
            Self::update_mass_distribution_inner(quadtree, root_index);
        }
    }

    fn update_mass_distribution_inner(quadtree: &mut Quadtree<Star, Option<Region>>, index: HilbertIndex) {
        // Update all children recursively, and then sum up their masses and produce a weighted
        // center of mess.
        let depth = index.depth();
        let (x, y) = index.to_xy();

        let mut mass = 0.0;
        let mut center_of_mass = Vec2::new(0.0, 0.0);

        for child_y in (y*2)..(y*2+2) {
            for child_x in (x*2)..(x*2+2) {
                let child_index = HilbertIndex::from_xy_depth((child_x, child_y), depth + 1);

                // If the child node is itself an internal node, we need to recurse deeper
                if quadtree.get(child_index).unwrap_or(&QuadtreeNode::Empty).is_internal() {
                    Self::update_mass_distribution_inner(quadtree, child_index);
                }

                // Update our mass and weighted center of mass.
                match quadtree.get(child_index) {
                    Some(QuadtreeNode::Internal(region)) => {
                        // All child regions should be initialised now due to recursion.
                        let region = region.as_ref().expect("Internal error: child region not initialised");
                        mass += region.mass;
                        center_of_mass.x += region.mass * region.center_of_mass.x;
                        center_of_mass.y += region.mass * region.center_of_mass.y;
                    },
                    Some(QuadtreeNode::Leaf(star)) => {
                        mass += star.mass;
                        center_of_mass.x += star.position.x;
                        center_of_mass.y += star.position.y;
                    },
                    _ => {},
                }
            }
        }

        // Calculate our weighted center of mass and store it.
        if mass != 0.0 {
            center_of_mass.x /= mass;
            center_of_mass.y /= mass;
        }

        log::info!("Setting mass ({mass}) and center of mass {center_of_mass:?} for node {index:?}");
        if let QuadtreeNode::Internal(region) = quadtree.get_mut(index).expect("Internal node does not exist") {
            *region = Some(Region { mass, center_of_mass });
        }
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
                            let y = ((pos.y / 2.0 + 0.5) * TEX_HEIGHT as f32) as usize;

                            // Get index and slice of pixel, *4 because the texture is 4 bytes per pixel.
                            let idx = 4 * (y * TEX_WIDTH + x);
                            let pixel = &mut bytes[idx..idx+4];

                            pixel[0] = 0xFF;
                            pixel[1] = 0x00;
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
