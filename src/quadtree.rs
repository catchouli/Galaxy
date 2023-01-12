use std::{error::Error, collections::VecDeque};

use crate::{types::Vec2, drawable::DebugDrawable, primitives::WireframeQuad};
use crate::hilbert;
use crate::hilbert::HilbertIndex;

const BLOCK_SIZE: usize = 10000;

/// A trait for objects with a position.
pub trait Spatial {
    fn xy(&self) -> &Vec2;
}

/// A quadtree node item, either an internal node, a leaf node, or empty (i.e. a sparse region
/// where we can stop traversal).
#[derive(PartialEq)]
pub enum QuadtreeNode<T: Spatial, Internal: Default> {
    Empty,
    Internal(Internal),
    Leaf(T)
}

impl<T: Spatial, Internal: Default> QuadtreeNode<T, Internal> {
    pub fn is_empty(&self) -> bool {
        match self {
            QuadtreeNode::Empty => true,
            _ => false,
        }
    }

    pub fn is_leaf(&self) -> bool {
        match self {
            QuadtreeNode::Leaf(_) => true,
            _ => false,
        }
    }

    pub fn is_internal(&self) -> bool {
        match self {
            QuadtreeNode::Internal(_) => true,
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

impl<T: Spatial, Internal: Default> core::fmt::Debug for QuadtreeNode<T, Internal> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "Empty"),
            Self::Internal(_) => write!(f, "Internal"),
            Self::Leaf(item) => f.debug_tuple("Leaf").field(item.xy()).finish(),
        }
    }
}

/// A sparse quadtree which is represented by a flat list of spatially indexed nodes. The leaf
/// nodes own their contained items and the tree grows dynamically like a Vec. The type `T` is the
/// type to be stored in the quadtree, and one is present in each leaf node of the tree. The
/// optional type parameter `Internal` can be used to specify a type for internal nodes.
pub struct Quadtree<T: Spatial, Internal: Default = ()> {
    /// The min of the bounds of the quadtree's root node, both values must be less than the ones
    /// in Quadtree::max.
    min: Vec2,

    /// The max of the bounds of the quadtree's root node, both values must be greater than the
    /// ones in Quadtree::min.
    max: Vec2,

    /// The quadtree nodes, as a flat list.
    blocks: Vec<Option<Vec<QuadtreeNode<T, Internal>>>>,
    //nodes: Vec<QuadtreeNode<T, Internal>>,

    /// A wireframe quad primitive for debug drawing.
    wireframe_quad: Option<WireframeQuad>,
}

impl<T: Spatial, Internal: Default> Quadtree<T, Internal> {
    /// Create a new quadtree with the given bounds.
    pub fn new(min: Vec2, max: Vec2) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            min,
            max,
            blocks: Vec::new(),
            wireframe_quad: None,
        })
    }

    pub fn get(&self, index: HilbertIndex) -> &QuadtreeNode<T, Internal> {
        let index = index.array_index();
        let block = index / BLOCK_SIZE;
        let index_in_block = index - (block * BLOCK_SIZE);

        match self.blocks.get(block) {
            Some(Some(block)) => block.get(index_in_block).unwrap_or(&QuadtreeNode::Empty),
            _ => &QuadtreeNode::Empty,
        }
    }

    pub fn get_mut(&mut self, index: HilbertIndex) -> Option<&mut QuadtreeNode<T, Internal>> {
        let index = index.array_index();
        let block = index / BLOCK_SIZE;
        let index_in_block = index - (block * BLOCK_SIZE);

        match self.blocks.get_mut(block) {
            Some(Some(block)) => block.get_mut(index_in_block),
            _ => None,
        }
    }

    /// Safely insert a node at an index, resizing the internal vector if necessary.
    fn safe_insert(&mut self, index: HilbertIndex, node: QuadtreeNode<T, Internal>) {
        let index = index.array_index();
        let block = index / BLOCK_SIZE;
        let index_in_block = index - (block * BLOCK_SIZE);

        if self.blocks.len() <= block {
            self.blocks.resize_with(block + 1, Default::default);
        }

        let block = self.blocks[block].get_or_insert_with(|| {
            let mut block = Vec::new();
            block.resize_with(BLOCK_SIZE, || QuadtreeNode::Empty);
            block
        });

        block[index_in_block] = node;

        //let array_index = index.array_index();
        //if array_index + 1 > self.nodes.len() {
        //    self.nodes.resize_with(array_index + 1, || QuadtreeNode::Empty);
        //}
        //self.nodes[array_index] = node;
    }

    /// Add a new item to the quadtree.
    pub fn add(&mut self, item: T) {
        // If item is outside the bounds of the quadtree, do nothing.
        let pos = item.xy();
        if pos.x < self.min.x || pos.x > self.max.x || pos.y < self.min.y || pos.y > self.max.y {
            log::info!("Item is outside of quadtree area, discarding");
            return;
        }

        // Find an insert position for the item by recursively walking the tree.
        let insert_pos = self.find_insert_pos(pos);

        // If it's empty, (e.g. in the case where this is the first item added to the tree), we can
        // just add this node directly to the specified index.
        if self.get(insert_pos).is_empty() {
            log::debug!("Inserting first node into tree at index {insert_pos:?}");
            self.safe_insert(insert_pos, QuadtreeNode::Leaf(item));
            return;
        }
        // Otherwise, we have to split the current leaf node until the two items are in separate quadrants.
        else {
            self.split_and_insert(insert_pos, item);
        }
    }

    /// Find the insert position of an item. The position might already contain another item, in
    /// which case it will need to be split recursively until the items end up in different nodes.
    fn find_insert_pos(&self, pos: &Vec2) -> HilbertIndex {
        // Start at the root and recursively search for an appropriate insert position (leaf node)
        // to insert the item.
        let mut cur_xy = (0, 0);
        let mut cur_index = HilbertIndex(0, 0);
        let mut cur_min = self.min;
        let mut cur_max = self.max;

        // If the current node is an internal node (as opposed to a leaf or an empty node), we have
        // to keep searching.
        while self.get(cur_index).is_internal() {
            // Find out which quadrant the item is and descend into the tree.
            let cur_center = cur_max * 0.5 + cur_min * 0.5;
            let (quadrant_x, quadrant_y) = Self::quadrant(&cur_center, pos);

            // Descend into child.
            cur_xy = (cur_xy.0 * 2 + quadrant_x, cur_xy.1 * 2 + quadrant_y);
            cur_index = HilbertIndex::from_xy_depth(cur_xy, cur_index.depth() + 1);

            // Update bounds.
            if quadrant_x == 0 {
                cur_max.x = cur_center.x;
            }
            else {
                cur_min.x = cur_center.x;
            }

            if quadrant_y == 0 {
                cur_max.y = cur_center.y;
            }
            else {
                cur_min.y = cur_center.y;
            }
        }

        cur_index
    }

    /// Split the specified leaf node and insert the new item. In order to do this, we need to
    /// descend until the item in the existing leaf node and the new item are in different
    /// quadrants, if necessary.
    fn split_and_insert(&mut self, mut insert_pos: HilbertIndex, item: T) {
        // Otherwise, we have to split the current leaf node until the two items are in separate
        // leaf nodes.
        log::debug!("Splitting leaf node at {insert_pos:?}");

        // Replace leaf node in tree with internal node, and prepare to insert our two nodes
        // further down the tree.
        let a = std::mem::replace(self.get_mut(insert_pos).expect("Nonexistent leaf node"),
            QuadtreeNode::Internal(Default::default()));
        let b = QuadtreeNode::Leaf(item);

        // If the items match exactly, it's better just to discard some so that we don't end up
        // recursing infinitely.
        if a.xy() == b.xy() {
            log::info!("Tried to insert two identical items, discarding one.");
            return;
        }

        // Calculate bounds of current node.
        let original_node_size = (self.max - self.min) / (1 << insert_pos.depth()) as f32;

        let (mut x, mut y) = insert_pos.to_xy();
        let mut node_min = self.min + Vec2::new(original_node_size.x * x as f32,
                                                original_node_size.y * y as f32);
        let mut node_max = node_min + original_node_size;

        loop {
            let insert_depth = insert_pos.depth() + 1;
            let node_center = node_max * 0.5 + node_min * 0.5;
            let quadrant_a = Self::quadrant(&node_center, a.xy());
            let quadrant_b = Self::quadrant(&node_center, b.xy());

            // If the two nodes are in different quadrants, we can just insert them.
            if quadrant_a.0 != quadrant_b.0 || quadrant_a.1 != quadrant_b.1 {
                let insert_depth = insert_pos.depth() + 1;

                let index_a = HilbertIndex::from_xy_depth((x*2 + quadrant_a.0, y*2 + quadrant_a.1),
                    insert_depth);

                let index_b = HilbertIndex::from_xy_depth((x*2 + quadrant_b.0, y*2 + quadrant_b.1),
                    insert_depth);

                self.safe_insert(index_a, a);
                self.safe_insert(index_b, b);
                break;
            }
            // Otherwise, we have to insert a new internal node, and descend down the tree until we
            // find an appropriate place for them.
            else {
                // Descend into quadrant, updating node position and bounds.
                (x, y) = (x * 2 + quadrant_a.0, y * 2 + quadrant_a.1);
                insert_pos = HilbertIndex::from_xy_depth((x, y), insert_depth);

                if quadrant_a.0 == 0 {
                    node_max.x = node_center.x;
                }
                else {
                    node_min.x = node_center.x;
                }

                if quadrant_a.1 == 0 {
                    node_max.y = node_center.y;
                }
                else {
                    node_min.y = node_center.y;
                }

                // Insert internal node here, and repeat.
                self.safe_insert(insert_pos, QuadtreeNode::Internal(Default::default()));
            }
        }
    }

    /// Get the quadrant of a point with regards to the specified cell center.
    fn quadrant(center: &Vec2, point: &Vec2) -> (u32, u32) {
        (if point.x < center.x { 0 } else { 1 },
         if point.y < center.y { 0 } else { 1 })
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
            let depth = hilbert_index.depth();

            // Call the callback
            f(hilbert_index);

            // Add children to stack.
            if depth + 1 < hilbert::MAX_DEPTH {
                for i in 0..4 {
                    let child_index = HilbertIndex(hilbert_index.index() * 4 + i, depth + 1);
                    let child_node = self.get(child_index);

                    if !child_node.is_empty() {
                        stack.push_back(child_index);
                    }
                }
            }
        }
    }

    /// Walk the quadtree depth-first, calling the specified callback with the hilbert index and node.
    pub fn walk_nodes<F>(&self, mut f: F)
        where F: FnMut(HilbertIndex, &QuadtreeNode<T, Internal>) -> ()
    {
        self.walk_indices(|index| {
            f(index, self.get(index));
        });
    }
}

impl<T: Spatial, Internal: Default> DebugDrawable for Quadtree<T, Internal> {
    fn debug_draw(&mut self, ctx: &mut miniquad::Context) {
        self.wireframe_quad.get_or_insert_with(|| {
            WireframeQuad::new(ctx).unwrap()
        });
        let wireframe_quad = self.wireframe_quad.take().unwrap();

        let root_origin = self.min;
        let root_size = Vec2::new(self.max.x - self.min.x, self.max.y - self.min.y);

        self.walk_nodes(|index, node| {
            if node.is_internal() || node.is_leaf() {
                let (x, y) = index.to_xy();
                let grid_size = 1 << index.depth();

                let cell_size = Vec2::new(root_size.x / grid_size as f32, root_size.y / grid_size as f32);

                let cell_min = Vec2::new(root_origin.x + cell_size.x * x as f32,
                                         root_origin.y + cell_size.y * y as f32);
                let cell_max = Vec2::new(cell_min.x + cell_size.x,
                                         cell_min.y + cell_size.y);

                wireframe_quad.draw(ctx, &cell_min, &cell_max);
            }
        });
    }
}
