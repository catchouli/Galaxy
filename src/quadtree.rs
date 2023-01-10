use std::error::Error;

use crate::{types::Vec2, drawable::DebugDrawable, primitives::WireframeQuad};

/// A sparse quadtree which is represented by a flat list of spatially indexed nodes. The leaf
/// nodes own their contained items and the tree grows dynamically like a Vec.
pub struct Quadtree<T> {
    /// The min of the bounds of the quadtree's root node, both values must be less than the ones
    /// in Quadtree::max.
    min: Vec2,

    /// The max of the bounds of the quadtree's root node, both values must be greater than the
    /// ones in Quadtree::min.
    max: Vec2,

    /// The quadtree.
    nodes: Vec<Option<QuadtreeNode<T>>>,

    /// A wireframe quad primitive for debug drawing.
    wireframe_quad: Option<WireframeQuad>,
}

/// A single quadtree node, which represents a given region in the quadtree, depending on the
/// (min, max) dimensions of the root node of the tree, and its index in the node list.
struct QuadtreeNode<T> {
    items: Vec<T>,
}

impl<T> Quadtree<T> {
    /// Create a new quadtree with the given bounds.
    pub fn new(min: Vec2, max: Vec2) -> Result<Self, Box<dyn Error>> {
        // Create initial node list, which should have the root node in.
        let nodes = vec![Some(QuadtreeNode { items: Vec::new() })];

        Ok(Self {
            min,
            max,
            nodes,
            wireframe_quad: None,
        })
    }
}

impl<T> DebugDrawable for Quadtree<T> {
    fn debug_draw(&mut self, ctx: &mut miniquad::Context) {
        let wireframe_quad = self.wireframe_quad.get_or_insert_with(|| {
            WireframeQuad::new(ctx).unwrap()
        });

        let mut size = Vec2::new(self.max.x - self.min.x, self.max.y - self.min.y);
        for depth in 1..5 {
            // For each node at this depth
            for y in 0..depth {
                for x in 0..depth {
                    let min = Vec2::new(self.min.x + x as f32 * size.x, self.min.y + y as f32 * size.y);
                    let max = Vec2::new(min.x + size.x, min.y + size.y);
                    wireframe_quad.draw(ctx, &min, &max);
                }
            }
            size.x /= 2.0;
            size.y /= 2.0;
        }
    }
}
