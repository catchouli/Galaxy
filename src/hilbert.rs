/// A hilbert index type that represents a 32-bit one-dimensional spatial index and an 8-bit tree depth.
/// For example, (0, 0) would be the root node of a quad tree, while (0..4, 1) would be its 4^1 child nodes,
/// and then (0..16, 2) would be the 4^2 nodes on the next level.
///
/// We keep track of the depth so that we can calculate contiguous hilbert indexes for trees of different
/// levels, for example without this information the root node of an octree would be hilbert index 0, but 
/// The top left node on the second level would also be index 0. Instead, we store an index and a depth,
/// and then can convert it to an array index just by adding an appropriate offset according to the depth
/// if needed.
#[derive(Debug, PartialEq)]
pub struct HilbertIndex(pub u32, pub u8);

/// The exclusive maximum depth, as explained below.
pub const MAX_DEPTH: u8 = 16;

/// Offsets for each tree depth, each one is basically the offset of the previous level, plus the
/// number of nodes in the current level.
/// A 32-bit index lets us store 16 full levels of quadtree, or 1_431_655_765 nodes this way
/// (4^0 + 4^1 + ... + 4^15).
pub const DEPTH_OFFSETS: [usize; 16] = [0, 1, 5, 21, 85, 341, 1365, 5461, 21845, 87381, 349525, 1398101,
                                        5592405, 22369621, 89478485, 357913941];

impl HilbertIndex {
    /// Convert from an (x, y) coordinate at a given quadtree depth. The (x, y) coordinate
    /// represents a cell in a grid of size (depth * depth).
    pub fn from_xy_depth((mut x, mut y): (u32, u32), depth: u8) -> HilbertIndex {
        // This very non-rusty code is ported from the C code linked below.
        // https://en.wikipedia.org/wiki/Hilbert_curve#Applications_and_mapping_algorithms
        // n is the number of cells in each dimension, e.g. depth 0 = 1, depth 1 = 2, depth 2 = 4
        let n = 1 << depth;

        let mut rx;
        let mut ry;
        let mut d = 0;

        let mut s = n / 2;
        while s > 0 {
            rx = if x & s > 0 { 1 } else { 0 };
            ry = if y & s > 0 { 1 } else { 0 };
            d += s * s * ((3 * rx) ^ ry);
            Self::rot(n, &mut x, &mut y, rx, ry);
            s /= 2;
        }

        HilbertIndex(d, depth)
    }

    /// Convert from a hilbert index with a given depth to an (x, y) coordinate. The (x, y)
    /// coordinate represents a cell in a grid of size (depth * depth).
    pub fn to_xy(&self) -> (u32, u32) {
        // This very non-rusty code is again ported from the C code linked below.
        // https://en.wikipedia.org/wiki/Hilbert_curve#Applications_and_mapping_algorithms
        // n is the number of cells in each dimension, e.g. depth 0 = 1, depth 1 = 2, depth 2 = 4
        let n = 1 << self.depth();

        // t is just the pure hilbert index.
        let mut t = self.index();

        let mut x = 0;
        let mut y = 0;
        let mut rx;
        let mut ry;

        let mut s = 1;
        while s < n {
            rx = 1 & (t/2);
            ry = 1 & (t ^ rx);

            Self::rot(s, &mut x, &mut y, rx, ry);

            x += s * rx;
            y += s * ry;

            t /= 4;
            s *= 2;
        }

        (x, y)
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
    pub fn array_index(&self) -> usize {
        let depth = self.depth();
        if depth >= MAX_DEPTH {
            panic!("Hilbert Index depth of {} is greater than maximum depth of {}", depth, MAX_DEPTH);
        }

        DEPTH_OFFSETS[depth as usize] + self.index() as usize
    }

    /// Rotate/flip a quadrant appropriately.
    /// https://en.wikipedia.org/wiki/Hilbert_curve#Applications_and_mapping_algorithms
    fn rot(n: u32, x: &mut u32, y: &mut u32, rx: u32, ry: u32) {
        if ry == 0 {
            if rx == 1 {
                *x = n - 1 - *x;
                *y = n - 1 - *y;
            }

            std::mem::swap(x, y);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::*;

    /// A data structure for generating valid (x, y, depth) values for hilbert indexes for
    /// quickcheck.
    #[derive(Debug, Copy, Clone)]
    struct ValidHilbertXYDepth(u32, u32, u8);

    impl quickcheck::Arbitrary for ValidHilbertXYDepth {
        fn arbitrary(g: &mut Gen) -> Self {
            // Generate depth values from 0..15, and xy values from 0..(2^depth - 1)
            let depth = u8::arbitrary(g) % 16;
            let size = 1 << depth;
            let x = u32::arbitrary(g) % size;
            let y = u32::arbitrary(g) % size;
            Self(x, y, depth)
        }
    }

    #[test]
    fn hilbert_from_xy_depth() {
        // Manually written tests for first three depths.
        assert_eq!(HilbertIndex::from_xy_depth((0, 0), 0), HilbertIndex(0, 0));

        assert_eq!(HilbertIndex::from_xy_depth((0, 0), 1), HilbertIndex(0, 1));
        assert_eq!(HilbertIndex::from_xy_depth((0, 1), 1), HilbertIndex(1, 1));
        assert_eq!(HilbertIndex::from_xy_depth((1, 1), 1), HilbertIndex(2, 1));
        assert_eq!(HilbertIndex::from_xy_depth((1, 0), 1), HilbertIndex(3, 1));

        assert_eq!(HilbertIndex::from_xy_depth((0, 0), 2), HilbertIndex(0,  2));
        assert_eq!(HilbertIndex::from_xy_depth((1, 0), 2), HilbertIndex(1,  2));
        assert_eq!(HilbertIndex::from_xy_depth((1, 1), 2), HilbertIndex(2,  2));
        assert_eq!(HilbertIndex::from_xy_depth((0, 1), 2), HilbertIndex(3,  2));
        assert_eq!(HilbertIndex::from_xy_depth((0, 2), 2), HilbertIndex(4,  2));
        assert_eq!(HilbertIndex::from_xy_depth((0, 3), 2), HilbertIndex(5,  2));
        assert_eq!(HilbertIndex::from_xy_depth((1, 3), 2), HilbertIndex(6,  2));
        assert_eq!(HilbertIndex::from_xy_depth((1, 2), 2), HilbertIndex(7,  2));
        assert_eq!(HilbertIndex::from_xy_depth((2, 2), 2), HilbertIndex(8,  2));
        assert_eq!(HilbertIndex::from_xy_depth((2, 3), 2), HilbertIndex(9,  2));
        assert_eq!(HilbertIndex::from_xy_depth((3, 3), 2), HilbertIndex(10, 2));
        assert_eq!(HilbertIndex::from_xy_depth((3, 2), 2), HilbertIndex(11, 2));
        assert_eq!(HilbertIndex::from_xy_depth((3, 1), 2), HilbertIndex(12, 2));
        assert_eq!(HilbertIndex::from_xy_depth((2, 1), 2), HilbertIndex(13, 2));
        assert_eq!(HilbertIndex::from_xy_depth((2, 0), 2), HilbertIndex(14, 2));
        assert_eq!(HilbertIndex::from_xy_depth((3, 0), 2), HilbertIndex(15, 2));

        assert_eq!(HilbertIndex::from_xy_depth((0, 0), 3), HilbertIndex(0, 3));
        assert_eq!(HilbertIndex::from_xy_depth((0, 0), 4), HilbertIndex(0, 4));
        assert_eq!(HilbertIndex::from_xy_depth((0, 0), 5), HilbertIndex(0, 5));
    }

    #[test]
    fn hilbert_to_xy() {
        // Manually written tests for hilbert indexes back to xy values.
        assert_eq!(HilbertIndex::from_xy_depth((0, 0), 0).to_xy(), (0, 0));

        assert_eq!(HilbertIndex::from_xy_depth((0, 0), 1).to_xy(), (0, 0));
        assert_eq!(HilbertIndex::from_xy_depth((0, 1), 1).to_xy(), (0, 1));
        assert_eq!(HilbertIndex::from_xy_depth((1, 1), 1).to_xy(), (1, 1));
        assert_eq!(HilbertIndex::from_xy_depth((1, 0), 1).to_xy(), (1, 0));

        assert_eq!(HilbertIndex::from_xy_depth((0, 0), 2).to_xy(), (0, 0));
        assert_eq!(HilbertIndex::from_xy_depth((1, 0), 2).to_xy(), (1, 0));
        assert_eq!(HilbertIndex::from_xy_depth((1, 1), 2).to_xy(), (1, 1));
        assert_eq!(HilbertIndex::from_xy_depth((0, 1), 2).to_xy(), (0, 1));
        assert_eq!(HilbertIndex::from_xy_depth((0, 2), 2).to_xy(), (0, 2));
        assert_eq!(HilbertIndex::from_xy_depth((0, 3), 2).to_xy(), (0, 3));
        assert_eq!(HilbertIndex::from_xy_depth((1, 3), 2).to_xy(), (1, 3));
        assert_eq!(HilbertIndex::from_xy_depth((1, 2), 2).to_xy(), (1, 2));
        assert_eq!(HilbertIndex::from_xy_depth((2, 2), 2).to_xy(), (2, 2));
        assert_eq!(HilbertIndex::from_xy_depth((2, 3), 2).to_xy(), (2, 3));
        assert_eq!(HilbertIndex::from_xy_depth((3, 3), 2).to_xy(), (3, 3));
        assert_eq!(HilbertIndex::from_xy_depth((3, 2), 2).to_xy(), (3, 2));
        assert_eq!(HilbertIndex::from_xy_depth((3, 1), 2).to_xy(), (3, 1));
        assert_eq!(HilbertIndex::from_xy_depth((2, 1), 2).to_xy(), (2, 1));
        assert_eq!(HilbertIndex::from_xy_depth((2, 0), 2).to_xy(), (2, 0));
        assert_eq!(HilbertIndex::from_xy_depth((3, 0), 2).to_xy(), (3, 0));

        assert_eq!(HilbertIndex::from_xy_depth((0, 0), 3).to_xy(), (0, 0));
        assert_eq!(HilbertIndex::from_xy_depth((0, 0), 4).to_xy(), (0, 0));
        assert_eq!(HilbertIndex::from_xy_depth((0, 0), 5).to_xy(), (0, 0));
    }

    #[test]
    fn hilbert_array_indexes() {
        // Manually written tests for array indexes.
        assert_eq!(HilbertIndex(0, 0).array_index(), 0);

        assert_eq!(HilbertIndex(0, 1).array_index(), 1);
        assert_eq!(HilbertIndex(1, 1).array_index(), 2);
        assert_eq!(HilbertIndex(2, 1).array_index(), 3);
        assert_eq!(HilbertIndex(3, 1).array_index(), 4);

        assert_eq!(HilbertIndex(0,  2).array_index(), 5);
        assert_eq!(HilbertIndex(1,  2).array_index(), 6);
        assert_eq!(HilbertIndex(2,  2).array_index(), 7);
        assert_eq!(HilbertIndex(3,  2).array_index(), 8);
        assert_eq!(HilbertIndex(4,  2).array_index(), 9);
        assert_eq!(HilbertIndex(5,  2).array_index(), 10);
        assert_eq!(HilbertIndex(6,  2).array_index(), 11);
        assert_eq!(HilbertIndex(7,  2).array_index(), 12);
        assert_eq!(HilbertIndex(8,  2).array_index(), 13);
        assert_eq!(HilbertIndex(9,  2).array_index(), 14);
        assert_eq!(HilbertIndex(10, 2).array_index(), 15);
        assert_eq!(HilbertIndex(11, 2).array_index(), 16);
        assert_eq!(HilbertIndex(12, 2).array_index(), 17);
        assert_eq!(HilbertIndex(13, 2).array_index(), 18);
        assert_eq!(HilbertIndex(14, 2).array_index(), 19);
        assert_eq!(HilbertIndex(15, 2).array_index(), 20);

        assert_eq!(HilbertIndex(0, 3).array_index(), DEPTH_OFFSETS[3]);
        assert_eq!(HilbertIndex(0, 4).array_index(), DEPTH_OFFSETS[4]);
        assert_eq!(HilbertIndex(0, 5).array_index(), DEPTH_OFFSETS[5]);
    }

    quickcheck! {
        fn hilbert_from_xy_to_xy_reversible(input: ValidHilbertXYDepth) -> bool {
            match input {
                ValidHilbertXYDepth(x, y, depth) => {
                    HilbertIndex::from_xy_depth((x, y), depth).to_xy() == (x, y)
                }
            }
        }
    }
}
