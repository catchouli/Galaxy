use std::error::Error;
use std::f64::consts::PI;

use miniquad::*;
use rand::Rng;
use crate::hilbert::HilbertIndex;
use crate::primitives::TexturedQuad;
use crate::types::Vec2d;
use crate::drawable::{Drawable, DebugDrawable};
use crate::quadtree::{Quadtree, Spatial, QuadtreeNode};

/// The texture width.
const TEX_WIDTH: usize = 1024;

/// The texture height.
const TEX_HEIGHT: usize = 1024;

/// The view bounds (min, max), in parsecs, about the galaxy's origin.
const VIEW_BOUNDS: (Vec2d, Vec2d) = (Vec2d::new(-25_000.0, -25_000.0),
                                     Vec2d::new(25_000.0, 25_000.0));

/// The number of stars.
const STAR_COUNT: usize = 5000;

/// The minimum mass of each star, in solar masses.
const STAR_MASS_MIN: f64 = 0.1;

/// The maximum mass of each star, in solar masses.
const STAR_MASS_MAX: f64 = 10.0;

/// The star's initial speed in parsecs/second.
const STAR_INITIAL_SPEED: f64 = 50.0;

/// The mass of a supermassive black hole at a galaxy's core, in solar masses.
const SUPERMASSIVE_BLACK_HOLE_MASS: f64 = 4e6;

/// The gravitational constant in `parsecs * * solar mass^-1 * (km/s)^2`.
/// https://en.wikipedia.org/wiki/Gravitational_constant
const GRAVITATIONAL_CONSTANT: f64 = 4.30091727063;

/// Diameter of the galaxy in parsecs.
const GALAXY_DIAMETER: f64 = 32408.0;

/// Radius of the galaxy in parsecs, calculated.
const GALAXY_RADIUS: f64 = GALAXY_DIAMETER / 2.0;

/// Time scale of the simulation.
const INITIAL_TIME_SCALE: f64 = 100.0;

/// Minimum distance^2 in gravity calculation, below which it is clamped to this value.
const MIN_GRAVITY_DISTANCE_SQUARED: f64 = 1e5;

/// Whether to draw the debug overlay for the quadtree.
const DEBUG_DRAW_QUADTREE: bool = false;

/// A single star in our galaxy.
pub struct Star {
    position: Vec2d,
    velocity: Vec2d,
    mass: f64,
}

impl Spatial for Star {
    fn xy(&self) -> &Vec2d {
        &self.position
    }
}

/// A particle in a leaf node in the quadtree, which points to a star by index.
pub struct Particle {
    index: usize,
    position: Vec2d,
}

impl Spatial for Particle {
    fn xy(&self) -> &Vec2d {
        &self.position
    }
}

/// A region in our galaxy, in the quadtree. We use this to accelerate n-body calculations.
pub struct Region {
    center_of_mass: Vec2d,
    mass: f64,
}

/// A structure representing the rendering of a Galaxy. For now this includes both the simulation
/// and rendering logic, but it would be nice to separate them.
pub struct Galaxy {
    textured_quad: TexturedQuad,
    texture_dirty: bool,
    pub time_scale: f64,

    pub stars: Vec<Star>,

    /// The galaxy's quadtree. We store the stars as leaf nodes in the octree, and have an
    /// additional type Region for the internal nodes, which we use to accelerate n-body lookups.
    /// It's wrapped in an Option so it can be initialised lazily.
    pub quadtree: Quadtree<Particle, Option<Region>>,
}

impl Galaxy {
    /// Create a new galaxy that renders via the given miniquad context.
    pub fn new<R: Rng + ?Sized>(ctx: &mut Context, rng: &mut R) -> Result<Self, Box<dyn Error>> {
        // Create textured quad for drawing stars.
        let textured_quad = TexturedQuad::new(ctx, TEX_WIDTH, TEX_HEIGHT)?;

        // Create stars flat list.
        let mut stars = Vec::new();

        // Create quadtree.
        let mut quadtree = Quadtree::new(Vec2d::new(-GALAXY_RADIUS, -GALAXY_RADIUS),
                                         Vec2d::new(GALAXY_RADIUS, GALAXY_RADIUS))?;

        // Add supermassive black hole at center of galaxy.
        stars.push(Star {
            position: Vec2d::new(0.0, 0.0),
            velocity: Vec2d::new(0.0, 0.0),
            mass: SUPERMASSIVE_BLACK_HOLE_MASS,
        });
        quadtree.add(Particle { index: 0, position: stars[0].position });

        // Generate stars.
        for _ in 0..STAR_COUNT {
            // Generate star mass.
            let mass = rng.gen_range(STAR_MASS_MIN..STAR_MASS_MAX);

            // Generate position with angle/distance from center.
            //let angle = rng.gen_range(0.0..(PI*2.0));
            //let distance_from_center = rng.gen_range(0.0..GALAXY_RADIUS);
            //let position = Vec2d::new(f64::sin(angle) * distance_from_center,
            //                          f64::cos(angle) * distance_from_center);

            // Generate position in a rectangle.
            let position_bounds = (-GALAXY_RADIUS)..GALAXY_RADIUS;
            let position = Vec2d::new(rng.gen_range(position_bounds.clone()),
                                      rng.gen_range(position_bounds));

            // Figure out direction perpendicular to center.
            let angle = f64::atan2(position.x, position.y) + PI / 2.0;
            let direction = Vec2d::new(f64::sin(angle), f64::cos(angle));
            let velocity = direction * STAR_INITIAL_SPEED;
            let star_index = stars.len();

            // Add star to flat list and quadtree.
            stars.push(Star { position, velocity, mass });
            quadtree.add(Particle { index: star_index, position: stars[star_index].position });
        }

        Ok(Self {
            textured_quad,
            texture_dirty: true,
            time_scale: INITIAL_TIME_SCALE,
            quadtree,
            stars,
        })
    }

    pub fn update_mass_distribution(quadtree: &mut Quadtree<Particle, Option<Region>>,
                                    stars: &Vec<Star>) {
        // Update mass distributions recursively. We only need to do this if the root node is an
        // internal node. If it's a leaf node then nothing needs doing, if it's empty then nothing
        // needs doing.
        let root_index = HilbertIndex(0, 0);
        if quadtree.get(root_index).is_internal() {
            Self::update_mass_distribution_inner(quadtree, stars, root_index);
        }
    }

    fn update_mass_distribution_inner(quadtree: &mut Quadtree<Particle, Option<Region>>,
                                      stars: &Vec<Star>,
                                      index: HilbertIndex)
    {
        // Update all children recursively, and then sum up their masses and produce a weighted
        // center of mess.
        let mut mass = 0.0;
        let mut center_of_mass = Vec2d::new(0.0, 0.0);

        for child_index in index.children() {
            // If the child node is itself an internal node, we need to recurse deeper
            if quadtree.get(child_index).is_internal() {
                Self::update_mass_distribution_inner(quadtree, stars, child_index);
            }

            // Update our mass and weighted center of mass.
            match quadtree.get(child_index) {
                QuadtreeNode::Internal(region) => {
                    // All child regions should be initialised now due to recursion.
                    let region = region.as_ref().expect("Internal error: child region not initialised");
                    mass += region.mass;
                    center_of_mass.x += region.mass * region.center_of_mass.x;
                    center_of_mass.y += region.mass * region.center_of_mass.y;
                },
                QuadtreeNode::Leaf(particle) => {
                    let star = &stars[particle.index];
                    mass += star.mass;
                    center_of_mass.x += star.position.x;
                    center_of_mass.y += star.position.y;
                },
                _ => {},
            }
        }

        // Calculate our weighted center of mass and store it.
        if mass != 0.0 {
            center_of_mass.x /= mass;
            center_of_mass.y /= mass;
        }

        if let QuadtreeNode::Internal(region) = quadtree.get_mut(index).expect("Internal node does not exist") {
            *region = Some(Region { mass, center_of_mass });
        }
    }

    /// Calculate the forces on an object of a given mass at a given point. To save an unnecessary
    /// multiplication followed by an inevitable division when calculating the acceleration, we omit
    /// the mass of the body since it cancels out anyway:
    ///   Fgravity = (mass a * mass b * gravitation constant) / distance^2
    ///   acceleration = force / mass (from F = ma)
    pub fn acceleration_at_point(&self, point: Vec2d) -> Vec2d {
        self.acceleration_at_point_inner(point, HilbertIndex(0, 0))
    }

    /// Calculate the forces on an object from a particular tree node, recursively.
    fn acceleration_at_point_inner(&self, point: Vec2d, index: HilbertIndex) -> Vec2d {
        let mut force = Vec2d::new(0.0, 0.0);

        match self.quadtree.get(index) {
            QuadtreeNode::Leaf(particle) => {
                let star = &self.stars[particle.index];

                // If the star is at the same position as the point, we should ignore it as it's
                // probably the object itself, and otherwise we'll end up dividing by zero anyway.
                let diff = star.position - point;
                let d_squared = f64::max(MIN_GRAVITY_DISTANCE_SQUARED,
                                         diff.x * diff.x + diff.y * diff.y);

                if d_squared > 0.0 {
                    let dir = diff / f64::sqrt(d_squared);
                    let force_of_star_gravity = star.mass * GRAVITATIONAL_CONSTANT / d_squared;

                    force = force + dir * force_of_star_gravity;
                }
            },
            QuadtreeNode::Internal(region) => {
                let region = region.as_ref()
                    .expect(&format!("Region {index:?} uninitialised when calculating forces"));

                let diff = region.center_of_mass - point;
                let dist_squared = diff.x * diff.x + diff.y * diff.y;
                let dist = f64::sqrt(dist_squared);
                let node_size = GALAXY_DIAMETER / (1 << index.depth()) as f64;
                let dir = diff / dist;

                if dist > 0.0 && (node_size/dist) < 1.0 {
                    let force_of_gravity = region.mass * GRAVITATIONAL_CONSTANT / dist_squared;
                    force = force + dir * force_of_gravity;
                }
                else {
                    for child_index in index.children() {
                        force = force + self.acceleration_at_point_inner(point, child_index);
                    }
                }
            },
            _ => {},
        }

        force
    }

    /// Integrate stars.
    fn integrate(&mut self, time_delta: f64) {
        // Integrate all star velocities and positions.
        for i in 0..self.stars.len() {
            let star_position = self.stars[i].position;
            let acceleration = self.acceleration_at_point(star_position);

            let star = &mut self.stars[i];
            let velocity = star.velocity + acceleration * self.time_scale * time_delta;
            let position = star.position + velocity * self.time_scale * time_delta;

            star.position = position;
            star.velocity = velocity;
        }
    }

    /// Update the texture if the dirty flag is set.
    pub fn update_texture(&mut self, ctx: &mut Context) {
        if self.texture_dirty {
            log::info!("Updating star texture");

            self.texture_dirty = false;

            // Create new buffer.
            let mut bytes = vec![0; 4 * TEX_WIDTH * TEX_HEIGHT];

            // Draw all stars in buffer.
            let mut star_count = 0;
            let view_offset = VIEW_BOUNDS.0;
            let view_size = VIEW_BOUNDS.1 - VIEW_BOUNDS.0;
            self.quadtree.walk_nodes(|_, node| {
                match node {
                    QuadtreeNode::Leaf(particle) => {
                        let star = &self.stars[particle.index];

                        // Normalize position to texture coordinates.
                        let mut pos = star.position - view_offset;
                        pos.x /= view_size.x;
                        pos.y /= view_size.y;

                        // Convert to pixel coordinates in our texture.
                        let x = (pos.x * TEX_WIDTH as f64) as usize;
                        let y = (pos.y * TEX_HEIGHT as f64) as usize;

                        if star.mass < 1000000.0 {
                            if x < TEX_WIDTH && y < TEX_HEIGHT {
                                // Get index and slice of pixel, *4 because the texture is 4 bytes per pixel.
                                let idx = 4 * (y * TEX_WIDTH + x);
                                let pixel = &mut bytes[idx..idx+4];

                                let brightness = f64::min(star.mass / (STAR_MASS_MAX - STAR_MASS_MIN) * 255.0,
                                                          255.0) as u8;

                                if star_count > 25 {
                                    pixel[0] = brightness;
                                    pixel[1] = brightness;
                                    pixel[2] = brightness;
                                    pixel[3] = 0xFF;
                                }
                                else {
                                    pixel[0] = brightness;
                                    pixel[1] = 0x0;
                                    pixel[2] = 0x0;
                                    pixel[3] = 0xFF;
                                }
                            }
                        }

                        star_count += 1;
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
    fn update(&mut self, _ctx: &mut Context, time_delta: f64) {
        // Lets just make a new quadtree every time...
        self.quadtree = Quadtree::new(Vec2d::new(-GALAXY_RADIUS, -GALAXY_RADIUS),
                                      Vec2d::new(GALAXY_RADIUS, GALAXY_RADIUS)).unwrap();

        for i in 0..self.stars.len() {
            self.quadtree.add(Particle { index: i, position: self.stars[i].position });
        }

        Self::update_mass_distribution(&mut self.quadtree, &self.stars);
        self.integrate(time_delta);
        self.texture_dirty = true;
    }

    /// Draw the galaxy.
    fn draw(&mut self, ctx: &mut Context) {
        self.update_texture(ctx);
        self.textured_quad.draw(ctx);
        if DEBUG_DRAW_QUADTREE {
            self.quadtree.debug_draw(ctx);
        }
    }
}
