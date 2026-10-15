use std::collections::HashMap;
use std::{error::Error, collections::VecDeque};

use crate::types::Vec2d;
use crate::hilbert;
use crate::hilbert::HilbertIndex;

/// TODO: it might be good for the quadtree to own the list of T so that it can also maintain a map
/// of the current leaf node location of each item. That way, when updating items, we can automatically
/// check if they've moved outside of their current parent node bounds and move them appropriately.
///
/// TODO: I think it's also good if the tree itself is an actual tree data structure, and refers to
/// nodes only by this index. That way the tree structure itself can be sparse without using
/// potentially an insane amount of memory for deep trees (for example 16 levels deep should be
/// reasonable as that results in about a 1 parsec grid size on galactic scales). Currently a tree
/// this deep uses many gigabytes of memory, even with the block size above.
///
/// TODO: finally, it might also be a good idea that the leaf nodes contain a list of items rather
/// than a single item, and that we use a different heuristic for splitting, maybe number of nodes.
/// This keeps our tree structure a reasonable size, but may make the results a little less
/// accurate or the N-body algorithm a little less efficient.
///
/// TODO: we should probably handle fully removing nodes from the tree and remove them from the
/// flat list too at some point (e.g. if the particle goes outside of the qaudtree bounds we
/// probably need to do that unless we want to re-create it with new bounds.) For now these nodes
/// just keep existing in the flat list but are not in the tree structure, which is a space leak.

/// The type for node indexes into our flat list. The way our quadtree works is that we store all
/// items in a flat list that also works as a lookup table for the item's current location in the
/// tree, and this type indexes into that list.
pub type NodeIndex = usize;

/// A trait for objects with a position.
pub trait Spatial {
    fn xy(&self) -> &Vec2d;
}

/// A quadtree node item, either an internal node, a leaf node, or empty (i.e. a sparse region
/// where we can stop traversal).
#[derive(PartialEq)]
pub enum QuadtreeNode {
    Internal(NodeIndex),
    Leaf(NodeIndex)
}

impl QuadtreeNode {
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
}

impl core::fmt::Debug for QuadtreeNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Internal(index) => f.debug_tuple("Internal").field(index).finish(),
            Self::Leaf(index) => f.debug_tuple("Leaf").field(index).finish(),
        }
    }
}

/// A sparse quadtree which is represented by a flat list of spatially indexed nodes. The leaf
/// nodes own their contained items and the tree grows dynamically like a Vec. The type `T` is the
/// type to be stored in the quadtree, and one is present in each leaf node of the tree. The
/// optional type parameter `Internal` can be used to specify a type for internal nodes.
pub struct Quadtree<T: Spatial, Internal = ()> {
    /// The min of the bounds of the quadtree's root node, both values must be less than the ones
    /// in Quadtree::max.
    pub min: Vec2d,

    /// The max of the bounds of the quadtree's root node, both values must be greater than the
    /// ones in Quadtree::min.
    pub max: Vec2d,

    /// Items stored in the quadtree as a flat list, along with the node index they're in.
    pub items: Vec<T>,

    /// Internal node values in the quadtree.
    internal: Vec<Option<Internal>>,

    /// The quadtree nodes, as a flat list.
    nodes: HashMap<HilbertIndex, QuadtreeNode>,
}

impl<T: Spatial, Internal> Quadtree<T, Internal> {
    /// Create a new quadtree with the given bounds.
    pub fn new(min: Vec2d, max: Vec2d) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            min,
            max,
            items: Vec::new(),
            internal: Vec::new(),
            nodes: HashMap::new(),
        })
    }

    pub fn get_item(&self, index: NodeIndex) -> Option<&T> {
        self.items.get(index)
    }

    pub fn get_internal(&self, index: NodeIndex) -> Option<&Internal> {
        self.internal.get(index)
            .map(Option::as_ref)
            .flatten()
    }

    pub fn set_internal(&mut self, index: NodeIndex, value: Option<Internal>) {
        if self.internal.len() <= index {
            panic!("Attempted to set the value of the nonexistent internal node {index:?}");
        }
        self.internal[index] = value;
    }

    pub fn get(&self, index: HilbertIndex) -> Option<&QuadtreeNode> {
        self.nodes.get(&index)
        //let index = index.array_index();
        //let block = index / BLOCK_SIZE;
        //let index_in_block = index - (block * BLOCK_SIZE);

        //match self.blocks.get(block) {
        //    Some(Some(block)) => block.get(index_in_block).unwrap_or(&QuadtreeNode::Empty),
        //    _ => &QuadtreeNode::Empty,
        //}
    }

    /// Safely insert a node at an index, resizing the internal vector if necessary.
    fn safe_insert(&mut self, index: HilbertIndex, node: QuadtreeNode) {
        self.nodes.insert(index, node);
        //let index = index.array_index();
        //let block = index / BLOCK_SIZE;
        //let index_in_block = index - (block * BLOCK_SIZE);

        //if self.blocks.len() <= block {
        //    self.blocks.resize_with(block + 1, Default::default);
        //}

        //let block = self.blocks[block].get_or_insert_with(|| {
        //    let mut block = Vec::new();
        //    block.resize_with(BLOCK_SIZE, || QuadtreeNode::Empty);
        //    block
        //});

        //block[index_in_block] = node;
    }

    /// Add a new item to the quadtree.
    pub fn add(&mut self, item: T) {
        // If item is outside the bounds of the quadtree, do nothing.
        let pos = item.xy();
        if pos.x < self.min.x || pos.x > self.max.x || pos.y < self.min.y || pos.y > self.max.y {
            // TODO: re-add this?
            //log::warn!("Item at position {pos:?} is outside of quadtree area, discarding");
            return;
        }

        // Find an insert position for the item by recursively walking the tree.
        let insert_pos = self.find_insert_pos(pos);

        // Add item to internal list.
        let index = self.items.len();
        self.items.push(item);

        // If it's empty, (e.g. in the case where this is the first item added to the tree), we can
        // just add this node directly to the specified index.
        if self.get(insert_pos).is_none() {
            log::trace!("Inserting first node into tree at index {insert_pos:?}");
            self.safe_insert(insert_pos, QuadtreeNode::Leaf(index));
            return;
        }
        // Otherwise, we have to split the current leaf node until the two items are in separate quadrants.
        else {
            self.split_and_insert(insert_pos, index);
        }
    }

    /// Find the insert position of an item. The position might already contain another item, in
    /// which case it will need to be split recursively until the items end up in different nodes.
    fn find_insert_pos(&self, pos: &Vec2d) -> HilbertIndex {
        // Start at the root and recursively search for an appropriate insert position (leaf node)
        // to insert the item.
        let mut cur_xy = (0, 0);
        let mut cur_index = HilbertIndex(0, 0);
        let mut cur_min = self.min;
        let mut cur_max = self.max;

        // If the current node is an internal node (as opposed to a leaf or an empty node), we have
        // to keep searching.
        loop {
            // If the current node is empty or a leaf node, we can insert here (splitting if necessary).
            let cur_node = self.get(cur_index);
            if cur_node.is_none() || cur_node.unwrap().is_leaf() {
                break;
            }

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
    fn split_and_insert(&mut self, leaf_pos: HilbertIndex, item: NodeIndex) {
        log::trace!("Splitting leaf node at {leaf_pos:?}");

        // Get the existing item and the positions of both items.
        let existing_item = match self.get(leaf_pos) {
            Some(&QuadtreeNode::Leaf(index)) => index,
            _ => panic!("Tried to split a non-leaf node"),
        };
        let a_xy = *self.items[existing_item].xy();
        let b_xy = *self.items[item].xy();

        // Calculate bounds of current node.
        let original_node_size = (self.max - self.min) / (1 << leaf_pos.depth()) as f64;

        let (mut x, mut y) = leaf_pos.to_xy();
        let mut node_min = self.min + Vec2d::new(original_node_size.x * x as f64,
                                                 original_node_size.y * y as f64);
        let mut node_max = node_min + original_node_size;

        // Descend until the two items are in different quadrants, working out where the new
        // internal nodes and the two leaves go. Nothing is changed until we know it'll fit.
        let mut insert_pos = leaf_pos;
        let mut new_internal_positions = Vec::new();
        let (index_a, index_b) = loop {
            let insert_depth = insert_pos.depth() + 1;

            // Leaves only hold one item, so two items too close together to separate within the
            // maximum depth (including identical positions) can't both be in the tree. Leave the
            // new one out: it's still in the item list so it's still simulated, it just isn't
            // in the tree until the two move apart.
            if insert_depth >= hilbert::MAX_DEPTH {
                log::debug!("Items at {a_xy:?} and {b_xy:?} are too close to split, leaving one out of the tree");
                return;
            }

            let node_center = node_max * 0.5 + node_min * 0.5;
            let quadrant_a = Self::quadrant(&node_center, &a_xy);
            let quadrant_b = Self::quadrant(&node_center, &b_xy);

            // If the two items are in different quadrants, we can insert them here.
            if quadrant_a != quadrant_b {
                let index_a = HilbertIndex::from_xy_depth((x*2 + quadrant_a.0, y*2 + quadrant_a.1),
                    insert_depth);
                let index_b = HilbertIndex::from_xy_depth((x*2 + quadrant_b.0, y*2 + quadrant_b.1),
                    insert_depth);
                break (index_a, index_b);
            }

            // Otherwise we need another internal node, so descend into the shared quadrant,
            // updating node position and bounds.
            (x, y) = (x * 2 + quadrant_a.0, y * 2 + quadrant_a.1);
            insert_pos = HilbertIndex::from_xy_depth((x, y), insert_depth);
            new_internal_positions.push(insert_pos);

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
        };

        // Replace the leaf with an internal node, add the internal nodes below it, and then the
        // two leaves. Each internal node needs its own slot in the internal value list.
        for pos in std::iter::once(leaf_pos).chain(new_internal_positions) {
            let internal_index = self.new_internal();
            self.safe_insert(pos, QuadtreeNode::Internal(internal_index));
        }
        self.safe_insert(index_a, QuadtreeNode::Leaf(existing_item));
        self.safe_insert(index_b, QuadtreeNode::Leaf(item));
    }

    /// Allocate a slot for a new internal node's value and return its index.
    /// TODO: reuse slots from deleted internal nodes.
    fn new_internal(&mut self) -> NodeIndex {
        self.internal.push(None);
        self.internal.len() - 1
    }

    /// Get the quadrant of a point with regards to the specified cell center.
    fn quadrant(center: &Vec2d, point: &Vec2d) -> (u32, u32) {
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

                    if child_node.is_some() {
                        stack.push_back(child_index);
                    }
                }
            }
        }
    }

    /// Walk the quadtree depth-first, calling the specified callback with the hilbert index and node.
    pub fn walk_nodes<F>(&self, mut f: F)
        where F: FnMut(HilbertIndex, &QuadtreeNode) -> ()
    {
        self.walk_indices(|index| {
            if let Some(node) = self.get(index) {
                f(index, node);
            }
        });
    }
}
