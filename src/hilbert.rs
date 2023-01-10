/// A hilbert index type that represents a 32-bit one-dimensional spatial index and an 8-bit tree depth.
/// For example, (0, 1) would be the root node of a quad tree, while (0..4, 1) would be its 4^1 child nodes,
/// and then (0..16, 2) would be the 4^2 nodes on the next level.
///
/// We keep track of the depth so that we can calculate contiguous hilbert indexes for trees of different
/// levels, for example without this information the root node of an octree would be hilbert index 0, but 
/// The top left node on the second level would also be index 0. Instead, we store an index and a depth,
/// and then can convert it to an array index just by adding an appropriate offset according to the depth
/// if needed.
pub struct HilbertIndex(u32, u8);

/// The exclusive maximum depth, as explained below.
pub const MAX_DEPTH: u8 = 16;

/// Offsets for each tree depth, each one is basically the offset of the previous level, plus the
/// number of nodes in the current level.
/// A 32-bit index lets us store 16 full levels of quadtree, or 1_431_655_765 nodes this way
/// (4^0 + 4^1 + ... + 4^15).
const DEPTH_OFFSETS: [u32; 16] = [0, 1, 5, 21, 85, 341, 1365, 5461, 21845, 87381, 349525, 1398101, 5592405,
    22369621, 89478485, 357913941];

impl HilbertIndex {
    /// Convert from an (x, y) coordinate at a given quadtree depth. The (x, y) coordinate
    /// represents a cell in a grid of size (depth * depth).
    pub fn from_xy(depth: u8, x: u32, y: u32) -> HilbertIndex {
        todo!();
    }

    /// Convert from a hilbert index with a given depth to an (x, y) coordinate. The (x, y)
    /// coordinate represents a cell in a grid of size (depth * depth).
    pub fn to_xy(&self) -> (u32, u32) {
        todo!();
    }

    /// Get the raw hilbert index.
    pub fn index(&self) -> u32 {
        match self {
            HilbertIndex(idx, _) => *idx
        }
    }

    /// Get the octree depth the hilbert index refers to.
    pub fn depth(&self) -> u8 {
        match self {
            HilbertIndex(_, depth) => *depth
        }
    }

    /// Calculate the linear array index of this hilbert index at this quadtree depth.
    pub fn array_index(&self) -> u32 {
        let depth = self.depth();
        if depth >= MAX_DEPTH {
            panic!("Hilbert Index depth of {} is greater than maximum depth of {}", depth, MAX_DEPTH);
        }

        DEPTH_OFFSETS[depth as usize] + self.index()
    }

    /// Rotate/flip a quadrant appropriately.
    /// https://en.wikipedia.org/wiki/Hilbert_curve#Applications_and_mapping_algorithms
    fn rot(&self, x: &mut u32, y: &mut u32, rx: u32, ry: u32) {
        if ry == 0 {
            if rx == 1 {
                let dimensions = 2 << (self.depth() as u32);
                *x = dimensions - 1 - *x;
                *y = dimensions - 1 - *y;
            }

            std::mem::swap(x, y);
        }
    }
}
