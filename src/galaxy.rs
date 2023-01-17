use std::error::Error;
use std::f64::consts::PI;
use std::time::Instant;

use imgui::TreeNodeFlags;
use miniquad::*;
use rand::Rng;
use crate::hilbert::HilbertIndex;
use crate::drawable::*;
use crate::input::InputState;
use crate::types::Vec2d;
use crate::quadtree::{Quadtree, Spatial, QuadtreeNode};

/// The texture width.
const TEX_WIDTH: usize = 512;

/// The texture height.
const TEX_HEIGHT: usize = 512;

/// The view bounds (min, max), in parsecs, about the galaxy's origin.
const VIEW_BOUNDS: (Vec2d, Vec2d) = (Vec2d::new(-25_000.0, -25_000.0),
                                     Vec2d::new(25_000.0, 25_000.0));

/// The number of stars.
const STAR_COUNT: usize = 5;

/// The minimum mass of each star, in solar masses.
const STAR_MASS_MIN: f64 = 0.1;

/// The maximum mass of each star, in solar masses.
const STAR_MASS_MAX: f64 = 10.0;

/// The mass of a supermassive black hole at a galaxy's core, in solar masses.
const SUPERMASSIVE_BLACK_HOLE_MASS: f64 = 4e6;

/// The gravitational constant in `km^2 pc Msun^-1 s^-2`.
/// https://lweb.cfa.harvard.edu/~dfabricant/huchra/ay145/constants.html
const GRAVITATIONAL_CONSTANT: f64 = 4.3e-3;

/// Diameter of the galaxy in parsecs.
const GALAXY_DIAMETER: f64 = 32408.0;

/// Radius of the galaxy in parsecs, calculated.
const GALAXY_RADIUS: f64 = GALAXY_DIAMETER / 2.0;

/// Time scale of the simulation.
const INITIAL_TIME_SCALE: f64 = 1000.0;

/// Minimum distance^2 in gravity calculation, below which it is clamped to this value.
const MIN_GRAVITY_DISTANCE_SQUARED: f64 = 0.0;

/// Whether to draw the debug overlay for the quadtree.
const DEBUG_DRAW_QUADTREE: bool = false;

/// How many stars to highlight in red for debugging purposes.
const HIGHLIGHT_RED_STAR_COUNT: usize = 0;

/// How fast the camera zooms (per mouse wheel click, which probably isn't consistent between
/// mousewheels but oh well.)
const CAMERA_ZOOM_SPEED: f64 = 1.0 / 200.0;

/// A simple "camera" (just a position, default viewport width and height, and zoom level).
struct Camera {
    position: Vec2d,
    viewport_dimensions: Vec2d,
    zoom_level: f64,
    locked_star: Option<usize>,
    highlighted_star: usize,
    right_mouse_down_prev: bool,
}

impl Camera {
    fn new() -> Self {
        Self {
            position: VIEW_BOUNDS.0 * 0.5 + VIEW_BOUNDS.1 * 0.5,
            viewport_dimensions: VIEW_BOUNDS.1 - VIEW_BOUNDS.0,
            zoom_level: 0.0,
            locked_star: None,
            highlighted_star: 0,
            right_mouse_down_prev: false,
        }
    }
}

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

    /// The galaxy's quadtree. We store the stars as leaf nodes in the octree, and have an
    /// additional type Region for the internal nodes, which we use to accelerate n-body lookups.
    /// It's wrapped in an Option so it can be initialised lazily.
    pub quadtree: Quadtree<Star, Region>,

    /// The simple "camera" containing the parameters to render the galaxy (such as viewport
    /// position).
    camera: Camera,
}

impl Galaxy {
    /// Create a new galaxy that renders via the given miniquad context.
    pub fn new<R: Rng + ?Sized>(ctx: &mut Context, rng: &mut R) -> Result<Self, Box<dyn Error>> {
        // Create textured quad for drawing stars.
        let textured_quad = TexturedQuad::new(ctx, TEX_WIDTH, TEX_HEIGHT)?;

        // Create quadtree.
        let mut quadtree = Quadtree::new(Vec2d::new(-GALAXY_RADIUS*2.0, -GALAXY_RADIUS*2.0),
                                         Vec2d::new(GALAXY_RADIUS*2.0, GALAXY_RADIUS*2.0))?;

        // Add supermassive black hole at center of galaxy.
        quadtree.add(Star {
            position: Vec2d::new(0.0, 0.0),
            velocity: Vec2d::new(0.0, 0.0),
            mass: SUPERMASSIVE_BLACK_HOLE_MASS,
        });

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
            let distance_from_center = f64::sqrt(position.x * position.x + position.y * position.y);

            // Calculate speed for orbit at this radius.
            // https://www.nagwa.com/en/explainers/142168516704/
            let speed = f64::sqrt(GRAVITATIONAL_CONSTANT * SUPERMASSIVE_BLACK_HOLE_MASS / distance_from_center);
            //let speed = f64::sqrt(GRAVITATIONAL_CONSTANT * 10000.0 / distance_from_center);
            //let speed = 0.0;
            //let speed = rng.gen_range(0.0..0.1);

            // Figure out direction perpendicular to center.
            let angle = f64::atan2(position.x, position.y) + PI / 2.0;
            let direction = Vec2d::new(f64::sin(angle), f64::cos(angle));
            let velocity = direction * speed;

            // Add star to flat list and quadtree.
            quadtree.add(Star { position, velocity, mass });
        }

        Ok(Self {
            textured_quad,
            texture_dirty: true,
            time_scale: INITIAL_TIME_SCALE,
            quadtree,
            camera: Camera::new(),
        })
    }

    pub fn update_mass_distribution(quadtree: &mut Quadtree<Star, Region>) {
        // Update mass distributions recursively. We only need to do this if the root node is an
        // internal node. If it's a leaf node then nothing needs doing, if it's empty then nothing
        // needs doing.
        let root_index = HilbertIndex(0, 0);
        if let Some(root_node) = quadtree.get(root_index) {
            if root_node.is_internal() {
                Self::update_mass_distribution_inner(quadtree, root_index);
            }
        }
    }

    fn update_mass_distribution_inner(quadtree: &mut Quadtree<Star, Region>,
                                      index: HilbertIndex)
    {
        // Update all children recursively, and then sum up their masses and produce a weighted
        // center of mess.
        let mut mass = 0.0;
        let mut center_of_mass = Vec2d::new(0.0, 0.0);

        for child_index in index.children() {
            let child_node = quadtree.get(child_index);
            if child_node.is_none() {
                continue;
            }
            let child_node = child_node.unwrap();

            // Update our mass and weighted center of mass.
            match child_node {
                &QuadtreeNode::Internal(region_index) => {
                    // If the child node is itself an internal node, we need to recurse deeper and update
                    // the children first.
                    Self::update_mass_distribution_inner(quadtree, child_index);

                    // All child regions should be initialised now due to recursion.
                    let region = quadtree.get_internal(region_index)
                        .expect(&format!("Internal error: child region {region_index:?} not initialised"));
                    mass += region.mass;
                    center_of_mass.x += region.mass * region.center_of_mass.x;
                    center_of_mass.y += region.mass * region.center_of_mass.y;
                },
                &QuadtreeNode::Leaf(item_index) => {
                    let star = quadtree.get_item(item_index)
                        .expect("Internal error: failed to get star from leaf node");
                    mass += star.mass;
                    center_of_mass.x += star.position.x;
                    center_of_mass.y += star.position.y;
                }
            }
        }

        // Calculate our weighted center of mass and store it.
        if mass != 0.0 {
            center_of_mass.x /= mass;
            center_of_mass.y /= mass;
        }

        // Update region data for this internal node.
        match quadtree.get(index) {
            Some(&QuadtreeNode::Internal(region_index)) => {
                let region = Region { mass, center_of_mass };
                quadtree.set_internal(region_index, Some(region));
            },
            _ => panic!("Found non-internal node when updating mass distribution")
        }
    }

    /// Calculate the forces on an object of a given mass at a given point. To save an unnecessary
    /// multiplication followed by an inevitable division when calculating the acceleration, we omit
    /// the mass of the body since it cancels out anyway:
    ///   Fgravity = (mass a * mass b * gravitation constant) / distance^2
    ///   acceleration = force / mass (from F = ma)
    pub fn acceleration_at_point(quadtree: &Quadtree<Star, Region>, point: Vec2d) -> Vec2d {
        Self::acceleration_at_point_inner(quadtree, point, HilbertIndex(0, 0))
    }

    /// Calculate the forces on an object from a particular tree node, recursively.
    fn acceleration_at_point_inner(quadtree: &Quadtree<Star, Region>, point: Vec2d, index: HilbertIndex) -> Vec2d {
        let mut force = Vec2d::new(0.0, 0.0);

        match quadtree.get(index) {
            Some(&QuadtreeNode::Leaf(item_index)) => {
                let star = quadtree.get_item(item_index)
                    .expect("Failed to get star");

                // If the star is at the same position as the point, we should ignore it as it's
                // probably the object itself, and otherwise we'll end up dividing by zero anyway.
                let diff = star.position - point;
                let d_squared = f64::max(MIN_GRAVITY_DISTANCE_SQUARED,
                                         diff.x * diff.x + diff.y * diff.y);

                if d_squared > 0.0 {
                    let dist = f64::sqrt(d_squared);
                    let dir = diff / dist;
                    let force_of_star_gravity = star.mass * GRAVITATIONAL_CONSTANT / d_squared;

                    force = force + dir * force_of_star_gravity;
                }
            },
            Some(&QuadtreeNode::Internal(region_index)) => {
                let region = quadtree.get_internal(region_index)
                    .expect(&format!("Region {index:?} uninitialised when calculating forces"));

                let diff = region.center_of_mass - point;
                let dist_squared = diff.x * diff.x + diff.y * diff.y;
                let dist = f64::sqrt(dist_squared);
                let node_size = GALAXY_DIAMETER / (1 << index.depth()) as f64;
                let dir = diff / dist;

                if dist != 0.0 && node_size / dist > 1.0 {
                    let force_of_gravity = region.mass * GRAVITATIONAL_CONSTANT / dist_squared;
                    force = force + dir * force_of_gravity;
                }
                else {
                    for child_index in index.children() {
                        force = force + Self::acceleration_at_point_inner(quadtree, point, child_index);
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
        // TODO: integrating the black hole breaks it and makes it disappear, it's not really
        // necessary but it would be nice to work out why :)
        for i in 1..self.quadtree.items.len() {
            // Calculate forces for star.
            let star = &self.quadtree.items[i];
            let acceleration = Self::acceleration_at_point(&self.quadtree, star.position);

            // Reborrow as mutable now that we're done calculating the forces and update it.
            let star = &mut self.quadtree.items[i];
            star.velocity = star.velocity + acceleration * self.time_scale * time_delta;
            star.position = star.position + star.velocity * self.time_scale * time_delta;
        }
    }

    /// Update the texture if the dirty flag is set.
    pub fn update_texture(&mut self, ctx: &mut Context) {
        if self.texture_dirty {
            log::debug!("Updating star texture");

            self.texture_dirty = false;

            // Create new buffer.
            let mut bytes = vec![0; 4 * TEX_WIDTH * TEX_HEIGHT];

            // Draw all stars in buffer.
            let mut star_count = 0;
            let zoom_scale = Self::linear_scale_to_exponential(self.camera.zoom_level);
            let view_size = self.camera.viewport_dimensions / zoom_scale;
            let view_offset = self.camera.position - view_size * 0.5;
            for (i, star) in self.quadtree.items.iter().enumerate() {
                // Normalize position to texture coordinates.
                let mut pos = star.position - view_offset;
                pos.x /= view_size.x;
                pos.y /= view_size.y;

                // Convert to pixel coordinates in our texture.
                let x = (pos.x * TEX_WIDTH as f64) as usize;
                let y = (pos.y * TEX_HEIGHT as f64) as usize;

                if true || star.mass < SUPERMASSIVE_BLACK_HOLE_MASS * 2.0 {
                    if x < TEX_WIDTH && y < TEX_HEIGHT {
                        // Get index and slice of pixel, *4 because the texture is 4 bytes per pixel.
                        let idx = 4 * (y * TEX_WIDTH + x);
                        let pixel = &mut bytes[idx..idx+4];

                        let brightness = f64::min(star.mass / (STAR_MASS_MAX - STAR_MASS_MIN) * 255.0,
                        255.0) as u8;

                        // TODO: refactor this a bit.
                        if i == self.camera.highlighted_star {
                            pixel[0] = 0x0;
                            pixel[1] = 0xFF;
                            pixel[2] = 0x0;
                            pixel[3] = 0xFF;
                        }
                        else if star_count > HIGHLIGHT_RED_STAR_COUNT {
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
            }

            // Update texture.
            self.textured_quad.texture.update(ctx, &bytes);
        }
    }

    fn update_camera(&mut self, input_state: &InputState) {
        // Just defined here since this module doesn't know the window parameters right now and
        // it's constant.
        const WINDOW_WIDTH: f64 = 1024.0;

        // Update camera zoom using scrollwheel.
        self.camera.zoom_level = f64::max(0.0,
            self.camera.zoom_level + input_state.mouse_wheel_dy as f64 * CAMERA_ZOOM_SPEED);

        let cur_scale = Self::linear_scale_to_exponential(self.camera.zoom_level);
        if input_state.left_mouse_button_down {
            // Translate pixel movement to movement at the current scale.
            // TODO: only works for a square viewport currently.
            let movement_scale = self.camera.viewport_dimensions.x / WINDOW_WIDTH
                / cur_scale;

            // Calculate movement.
            let (mouse_dx, mouse_dy) = input_state.mouse_diff;
            let movement = Vec2d::new(-mouse_dx as f64, mouse_dy as f64) * movement_scale;
            self.camera.position = self.camera.position + movement;
        }

        // Update highlighted star.
        if self.camera.locked_star.is_none() {
            let mouse_pos_window = Vec2d::new(input_state.mouse_pos.0 as f64, input_state.mouse_pos.1 as f64);
            let mouse_pos_world = self.window_to_world(mouse_pos_window);
            self.camera.highlighted_star = self.find_nearest_star(mouse_pos_world, HilbertIndex(0, 0));
        }

        // Update camera position to locked star position.
        if input_state.right_mouse_button_down && !self.camera.right_mouse_down_prev {
            if self.camera.locked_star.is_some() {
                self.camera.locked_star = None;
            }
            else {
                self.camera.locked_star = Some(self.camera.highlighted_star);
            }
        }
        self.camera.right_mouse_down_prev = input_state.right_mouse_button_down;

        if let Some(locked_star) = self.camera.locked_star {
            self.camera.position = self.quadtree.items[locked_star].position;
        }
    }

    fn linear_scale_to_exponential(linear: f64) -> f64 {
        f64::exp(linear)
    }

    // Project window to world coordinates.
    fn window_to_world(&self, window: Vec2d) -> Vec2d {
        // Just defined here since this module doesn't know the window parameters right now and
        // it's constant.
        const WINDOW_WIDTH: f64 = 1024.0;
        const WINDOW_HEIGHT: f64 = 1024.0;

        let zoom_scale = Self::linear_scale_to_exponential(self.camera.zoom_level);
        let view_size = self.camera.viewport_dimensions / zoom_scale;
        let view_offset = self.camera.position - view_size * 0.5;

        let pos_vp = Vec2d::new(window.x / WINDOW_WIDTH, 1.0 - window.y / WINDOW_HEIGHT);
        Vec2d::new(pos_vp.x * view_size.x, pos_vp.y * view_size.y) + view_offset
    }

    fn find_nearest_star(&self, point: Vec2d, index: HilbertIndex) -> usize {
        match self.quadtree.get(index) {
            Some(&QuadtreeNode::Internal(_)) => {
                let (x, y) = index.to_xy();
                let depth = index.depth();

                // Traverse into children until we find a leaf node.
                let (node_min, node_max) = index.bounds(self.quadtree.min, self.quadtree.max);
                let node_center = node_min * 0.5 + node_max * 0.5;

                let quadrant_x = if point.x < node_center.x { 0 } else { 1 };
                let quadrant_y = if point.y < node_center.y { 0 } else { 1 };

                let child_index = HilbertIndex::from_xy_depth((x*2 + quadrant_x, y*2 + quadrant_y), depth + 1);
                
                self.find_nearest_star(point, child_index)
            },
            Some(&QuadtreeNode::Leaf(star_index)) => star_index,
            _ => 0,
        }
    }
}

impl Drawable for Galaxy {
    /// Update the galaxy.
    fn update(&mut self, _ctx: &mut Context, ui: &mut imgui::Ui, input_state: &InputState, time_delta: f64) {
        // Update camera.
        self.update_camera(input_state);

        // Imgui windows.
        ui.window("Galaxy")
            .size([350.0, 300.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.collapsing_header("Simulation", TreeNodeFlags::all())
                    .then(|| {
                        ui.slider("Time scale", 0.0, 50_000.0, &mut self.time_scale);
                    });

                ui.collapsing_header("Camera", TreeNodeFlags::all())
                    .then(|| {
                        ui.label_text("Cam pos", format!("{:.2}, {:.2}",
                                                         self.camera.position.x,
                                                         self.camera.position.y));
                        ui.label_text("Zoom level", self.camera.zoom_level.to_string());
                    });

                ui.collapsing_header("Highlighted star", TreeNodeFlags::all())
                    .then(|| {
                        let star = &self.quadtree.items[self.camera.highlighted_star];
                        ui.label_text("Pos", format!("{:.2}, {:.2}", star.position.x, star.position.y));
                        ui.label_text("Velocity", format!("{:.2}, {:.2}", star.velocity.x, star.velocity.y));
                        ui.label_text("Mass", star.mass.to_string());
                    });
            });

        // Lets just make a new quadtree every time...
        let quadtree_build_start = Instant::now();
        let stars = std::mem::replace(&mut self.quadtree.items, Vec::new());

        self.quadtree = Quadtree::new(Vec2d::new(-GALAXY_RADIUS*2.0, -GALAXY_RADIUS*2.0),
                                      Vec2d::new(GALAXY_RADIUS*2.0, GALAXY_RADIUS*2.0)).unwrap();

        for star in stars {
            self.quadtree.add(star);
        }

        let quadtree_build_time = quadtree_build_start.elapsed().as_millis();

        // Update cached mass distribution and integrate.
        let mass_distribution_start = Instant::now();
        Self::update_mass_distribution(&mut self.quadtree);
        let mass_distribution_time = mass_distribution_start.elapsed().as_millis();

        let integrate_start = Instant::now();
        self.integrate(time_delta);
        let integrate_time = integrate_start.elapsed().as_millis();

        log::debug!("Update timings: quadtree {quadtree_build_time}ms, mass distribution {mass_distribution_time}ms, integrate {integrate_time}ms");

        self.texture_dirty = true;
    }

    /// Draw the galaxy.
    fn draw(&mut self, ctx: &mut Context, _ui: &mut imgui::Ui) {
        self.update_texture(ctx);
        self.textured_quad.draw(ctx);
        if DEBUG_DRAW_QUADTREE {
            self.quadtree.debug_draw(ctx);
        }
    }
}
