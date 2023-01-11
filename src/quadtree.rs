use std::{error::Error, collections::VecDeque};

use crate::{types::Vec2, drawable::DebugDrawable, primitives::WireframeQuad};
use crate::hilbert;
use crate::hilbert::HilbertIndex;

/// A trait for objects with a position.
pub trait Spatial {
    fn xy(&self) -> &Vec2;
}

/// A quadtree node item, either an internal node, a leaf node, or empty (i.e. a sparse region
/// where we can stop traversal.)
#[derive(PartialEq)]
pub enum QuadtreeNode<T: Spatial> {
    Empty,
    Internal,
    Leaf(T)
}

impl<T: Spatial> QuadtreeNode<T> {
    fn is_empty(&self) -> bool {
        match self {
            QuadtreeNode::Empty => true,
            _ => false,
        }
    }

    fn is_internal(&self) -> bool {
        match self {
            QuadtreeNode::Internal => true,
            _ => false,
        }
    }

    /// Get the xy of the item in the node, only valid for leaf nodes.
    fn xy(&self) -> &Vec2 {
        match self {
            QuadtreeNode::Leaf(item) => item.xy(),
            _ => panic!("Attempted to get xy of leaf node in quadtree"),
        }
    }
}

impl<T: Spatial> core::fmt::Debug for QuadtreeNode<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "Empty"),
            Self::Internal => write!(f, "Internal"),
            Self::Leaf(item) => f.debug_tuple("Leaf").field(item.xy()).finish(),
        }
    }
}

/// A sparse quadtree which is represented by a flat list of spatially indexed nodes. The leaf
/// nodes own their contained items and the tree grows dynamically like a Vec.
pub struct Quadtree<T: Spatial> {
    /// The min of the bounds of the quadtree's root node, both values must be less than the ones
    /// in Quadtree::max.
    min: Vec2,

    /// The max of the bounds of the quadtree's root node, both values must be greater than the
    /// ones in Quadtree::min.
    max: Vec2,

    /// The quadtree.
    nodes: Vec<QuadtreeNode<T>>,

    /// A wireframe quad primitive for debug drawing.
    wireframe_quad: Option<WireframeQuad>,
}

impl<T: Spatial> Quadtree<T> {
    /// Create a new quadtree with the given bounds.
    pub fn new(min: Vec2, max: Vec2) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            min,
            max,
            nodes: Vec::new(),
            wireframe_quad: None,
        })
    }

    /// Add a new item to the quadtree.
    pub fn add(&mut self, item: T) {
        // If item is outside the bounds of the quadtree, do nothing.
        let pos = item.xy();
        if pos.x < self.min.x || pos.x > self.max.x || pos.y < self.min.y || pos.y > self.max.y {
            log::info!("Item is outside of quadtree area, discarding");
            return;
        }

        // Walk down the tree, finding the right place to insert the item, and splitting existing
        // nodes so that each leaf node has a single item in. Like below, there's probably a more
        // direct way to get the children of a node by hilbert index, but this was fast to implement
        // and intuitive.
        // 
        // First we start and descend to the first leaf or empty node where we could put this item.
        let mut current_index = HilbertIndex(0, 0);
        let (mut current_x, mut current_y) = (0, 0);

        let mut current_origin = self.min;
        let mut current_size = Vec2::new(self.max.x - self.min.x, self.max.y - self.min.y);
        let mut current_node = self.nodes.get(current_index.array_index()).unwrap_or(&QuadtreeNode::Empty);
        let mut current_center = Vec2::new(current_origin.x + 0.5 * current_size.x,
                                           current_origin.y + 0.5 * current_size.y);

        while current_node.is_internal() {
            // Figure out which quadrant the item is in.
            let quadrant_x = if pos.x < current_center.x { 0 } else { 1 };
            let quadrant_y = if pos.y < current_center.y { 0 } else { 1 };

            // Figure out the new xy of the child node, and new hilbert index etc.
            current_x = current_x * 2 + quadrant_x;
            current_y = current_y * 2 + quadrant_y;
            current_index = HilbertIndex::from_xy_depth((current_x, current_y), current_index.depth() + 1);
            current_node = self.nodes.get(current_index.array_index()).unwrap_or(&QuadtreeNode::Empty);
            current_size = Vec2::new(0.5 * current_size.x, 0.5 * current_size.y);
            current_origin = Vec2::new(current_origin.x + current_size.x * quadrant_x as f32,
                                       current_origin.y + current_size.y * quadrant_y as f32);
            current_center = Vec2::new(current_origin.x + 0.5 * current_size.x,
                                       current_origin.y + 0.5 * current_size.y);
        }

        // If it's empty, (e.g. in the case where this is the first item added to the tree), we can
        // just add this node directly to the specified index.
        if current_node.is_empty() {
            let index = current_index.array_index();
            log::debug!("Inserting first node into tree at index {index}");
            if self.nodes.len() < index + 1 {
                self.nodes.resize_with(index + 1, || QuadtreeNode::Empty);
            }
            self.nodes[index] = QuadtreeNode::Leaf(item);
            return;
        }

        // Otherwise, we have to split the current leaf node until the two items are in separate
        // leaf nodes.
        log::debug!("Splitting leaf node at {current_index:?}");

        let leaf_a = std::mem::replace(&mut self.nodes[current_index.array_index()], QuadtreeNode::Internal);
        let leaf_a_pos = leaf_a.xy();

        let leaf_b = QuadtreeNode::Leaf(item);
        let leaf_b_pos = leaf_b.xy();

        // Descend into tree inserting internal nodes until the two items are in different
        // quadrants.
        loop {
            let leaf_a_quadrant_x = if leaf_a_pos.x < current_center.x { 0 } else { 1 };
            let leaf_a_quadrant_y = if leaf_a_pos.y < current_center.y { 0 } else { 1 };

            let leaf_b_quadrant_x = if leaf_b_pos.x < current_center.x { 0 } else { 1 };
            let leaf_b_quadrant_y = if leaf_b_pos.y < current_center.y { 0 } else { 1 };

            // If they're in the same quadrant, we need to insert a new internal node in this
            // quadrant, and recurse deeper unil they're in separate quadrants.
            if leaf_a_quadrant_x == leaf_b_quadrant_x && leaf_a_quadrant_y == leaf_b_quadrant_y {
                current_x = current_x * 2 + leaf_a_quadrant_x;
                current_y = current_y * 2 + leaf_a_quadrant_y;
                current_index = HilbertIndex::from_xy_depth((current_x, current_y), current_index.depth() + 1);

                current_size = Vec2::new(0.5 * current_size.x, 0.5 * current_size.y);
                current_origin = Vec2::new(current_origin.x + current_size.x * leaf_a_quadrant_x as f32,
                                           current_origin.y + current_size.y * leaf_a_quadrant_y as f32);
                current_center = Vec2::new(current_origin.x + 0.5 * current_size.x,
                                           current_origin.y + 0.5 * current_size.y);

                let new_len = current_index.array_index() + 1;
                if self.nodes.len() < new_len {
                    self.nodes.resize_with(new_len, || QuadtreeNode::Empty);
                }
                self.nodes[current_index.array_index()] = QuadtreeNode::Internal;
                log::debug!("Items are in the same quadrant, descending to {current_index:?}");
            }
            else {
                log::debug!("Found unique quadrants for items, inserting leaf nodes");

                let leaf_a_x = current_x * 2 + leaf_a_quadrant_x;
                let leaf_a_y = current_x * 2 + leaf_a_quadrant_y;
                let leaf_a_index = HilbertIndex::from_xy_depth((leaf_a_x, leaf_a_y), current_index.depth() + 1);

                let leaf_b_x = current_x * 2 + leaf_b_quadrant_x;
                let leaf_b_y = current_x * 2 + leaf_b_quadrant_y;
                let leaf_b_index = HilbertIndex::from_xy_depth((leaf_b_x, leaf_b_y), current_index.depth() + 1);

                let new_len = usize::max(leaf_a_index.array_index(), leaf_b_index.array_index()) + 1;
                if self.nodes.len() < new_len {
                    self.nodes.resize_with(new_len, || QuadtreeNode::Empty);
                }

                self.nodes[leaf_a_index.array_index()] = leaf_a;
                self.nodes[leaf_b_index.array_index()] = leaf_b;

                break;
            }
        }
    }

    /// Walk the quadtree depth-first, calling the specified callback with the hilbert index.
    pub fn walk_indices<F>(&self, mut f: F)
        where F: FnMut(HilbertIndex) -> ()
    {
        // Recursively walk the tree in depth-first order, visiting every node and calling the
        // callback. I don't know if it's best to manually maintain a stack like this or use
        // recursion, but I thought I'd try this for a change. Adds the root node to start with.
        let mut stack = VecDeque::<HilbertIndex>::new();
        stack.push_back(HilbertIndex(0, 0));

        while let Some(hilbert_index) = stack.pop_back() {
            // Get (x, y) of cell and depth in tree.
            let (x, y) = hilbert_index.to_xy();
            let depth = hilbert_index.depth();

            // Call the callback
            f(hilbert_index);

            // Add children to stack. There's probably a more direct way to do this from the
            // hilbert index, but for now this is simple and intuitive.
            if depth + 1 < hilbert::MAX_DEPTH {
                for child_x in (x*2)..(x*2+2) {
                    for child_y in (y*2)..(y*2+2) {
                        let child_index = HilbertIndex::from_xy_depth((child_x, child_y), depth + 1);
                        let child_node = self.nodes.get(child_index.array_index())
                            .unwrap_or(&QuadtreeNode::Empty);

                        if !child_node.is_empty() {
                            stack.push_back(child_index);
                        }
                    }
                }
            }
        }
    }

    /// Walk the quadtree depth-first, calling the specified callback with the hilbert index and node.
    pub fn walk_nodes<F>(&self, mut f: F)
        where F: FnMut(HilbertIndex, &QuadtreeNode<T>) -> ()
    {
        self.walk_indices(|index| {
            let array_index = index.array_index();
            f(index, &self.nodes[array_index]);
        });
    }
}

impl<T: Spatial> DebugDrawable for Quadtree<T> {
    fn debug_draw(&mut self, ctx: &mut miniquad::Context) {
        self.wireframe_quad.get_or_insert_with(|| {
            WireframeQuad::new(ctx).unwrap()
        });
        let wireframe_quad = self.wireframe_quad.take().unwrap();

        let root_origin = self.min;
        let root_size = Vec2::new(self.max.x - self.min.x, self.max.y - self.min.y);

        self.walk_indices(|index| {
            let (x, y) = index.to_xy();
            let grid_size = 1 << index.depth();

            let cell_size = Vec2::new(root_size.x / grid_size as f32, root_size.y / grid_size as f32);

            let cell_min = Vec2::new(root_origin.x + cell_size.x * x as f32,
                                     root_origin.y + cell_size.y * y as f32);
            let cell_max = Vec2::new(cell_min.x + cell_size.x,
                                     cell_min.y + cell_size.y);

            wireframe_quad.draw(ctx, &cell_min, &cell_max);
        });
    }
}
