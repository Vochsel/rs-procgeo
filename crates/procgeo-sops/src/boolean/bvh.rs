// AABB Bounding Volume Hierarchy for Boolean SOP broadphase

use glam::Vec3;

// ---------------------------------------------------------------------------
// Aabb
// ---------------------------------------------------------------------------

/// Axis-aligned bounding box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    /// Construct from three triangle vertices.
    pub fn from_triangle(a: Vec3, b: Vec3, c: Vec3) -> Self {
        Self {
            min: a.min(b).min(c),
            max: a.max(b).max(c),
        }
    }

    /// Union of two AABBs — the smallest AABB that contains both.
    pub fn union(&self, other: &Aabb) -> Aabb {
        Aabb {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    /// True iff the two AABBs overlap (touching counts as overlap).
    pub fn intersects(&self, other: &Aabb) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }

    /// Half the surface area (used for SAH-style splitting heuristics).
    pub fn surface_area(&self) -> f32 {
        let d = self.max - self.min;
        2.0 * (d.x * d.y + d.y * d.z + d.z * d.x)
    }

    /// Centre point.
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    /// Grow the box outward by `eps` on all sides.
    pub fn expanded(&self, eps: f32) -> Aabb {
        Aabb {
            min: self.min - Vec3::splat(eps),
            max: self.max + Vec3::splat(eps),
        }
    }
}

// ---------------------------------------------------------------------------
// Triangle
// ---------------------------------------------------------------------------

/// A triangle with a back-reference to its source primitive index.
#[derive(Debug, Clone, Copy)]
pub struct Triangle {
    pub v0: Vec3,
    pub v1: Vec3,
    pub v2: Vec3,
    /// Index of the source primitive in the owning geometry.
    pub index: usize,
}

impl Triangle {
    /// Tight AABB around this triangle.
    pub fn aabb(&self) -> Aabb {
        Aabb::from_triangle(self.v0, self.v1, self.v2)
    }

    /// Geometric centroid of the triangle.
    pub fn centroid(&self) -> Vec3 {
        (self.v0 + self.v1 + self.v2) / 3.0
    }
}

// ---------------------------------------------------------------------------
// BvhNode (internal)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct BvhNode {
    aabb: Aabb,
    /// Index into `TriangleBvh::nodes` for left child (`None` for leaf).
    left: Option<usize>,
    /// Index into `TriangleBvh::nodes` for right child (`None` for leaf).
    right: Option<usize>,
    /// Index into the triangle slice for leaf nodes.
    tri_idx: Option<usize>,
}

impl BvhNode {
    fn is_leaf(&self) -> bool {
        self.tri_idx.is_some()
    }
}

// ---------------------------------------------------------------------------
// TriangleBvh
// ---------------------------------------------------------------------------

/// Top-down BVH over a set of triangles.
pub struct TriangleBvh {
    nodes: Vec<BvhNode>,
    /// Triangles stored by the BVH (same order as input, referenced by tri_idx).
    triangles: Vec<Triangle>,
    root: usize,
}

impl TriangleBvh {
    /// Build a BVH from a slice of triangles using top-down median splitting.
    pub fn build(triangles: &[Triangle]) -> Self {
        if triangles.is_empty() {
            // Return a degenerate BVH with a single empty leaf.
            let dummy_aabb = Aabb {
                min: Vec3::ZERO,
                max: Vec3::ZERO,
            };
            return Self {
                nodes: vec![BvhNode {
                    aabb: dummy_aabb,
                    left: None,
                    right: None,
                    tri_idx: None,
                }],
                triangles: Vec::new(),
                root: 0,
            };
        }

        let mut bvh = Self {
            nodes: Vec::with_capacity(2 * triangles.len()),
            triangles: triangles.to_vec(),
            root: 0,
        };

        // Build over indices into bvh.triangles.
        let mut indices: Vec<usize> = (0..triangles.len()).collect();
        bvh.root = bvh.build_recursive(&mut indices);
        bvh
    }

    /// Recursively build nodes; returns the index of the created node.
    fn build_recursive(&mut self, indices: &mut [usize]) -> usize {
        // Compute union AABB for all triangles in this set.
        let aabb = indices.iter().map(|&i| self.triangles[i].aabb()).fold(
            Aabb {
                min: Vec3::splat(f32::MAX),
                max: Vec3::splat(f32::MIN),
            },
            |acc, b| acc.union(&b),
        );

        // Leaf case.
        if indices.len() == 1 {
            let node_idx = self.nodes.len();
            self.nodes.push(BvhNode {
                aabb,
                left: None,
                right: None,
                tri_idx: Some(indices[0]),
            });
            return node_idx;
        }

        // Find the longest axis of the bounding box to split along.
        let extent = aabb.max - aabb.min;
        let axis = if extent.x >= extent.y && extent.x >= extent.z {
            0
        } else if extent.y >= extent.z {
            1
        } else {
            2
        };

        // Sort by centroid along the chosen axis, then median split.
        indices.sort_unstable_by(|&a, &b| {
            let ca = centroid_on_axis(&self.triangles[a], axis);
            let cb = centroid_on_axis(&self.triangles[b], axis);
            ca.partial_cmp(&cb).unwrap_or(std::cmp::Ordering::Equal)
        });

        let mid = indices.len() / 2;
        let (left_indices, right_indices) = indices.split_at_mut(mid);

        // Allocate a placeholder node slot first so we know this node's index.
        let node_idx = self.nodes.len();
        self.nodes.push(BvhNode {
            aabb,
            left: None,
            right: None,
            tri_idx: None,
        });

        let left_idx = self.build_recursive(left_indices);
        let right_idx = self.build_recursive(right_indices);

        // Patch in child indices.
        self.nodes[node_idx].left = Some(left_idx);
        self.nodes[node_idx].right = Some(right_idx);

        node_idx
    }

    /// Find all pairs of triangle indices `(a, b)` where the AABB of triangle
    /// `a` in `self` overlaps the AABB of triangle `b` in `other`.
    pub fn find_overlapping_pairs(&self, other: &TriangleBvh) -> Vec<(usize, usize)> {
        let mut pairs = Vec::new();
        if self.triangles.is_empty() || other.triangles.is_empty() {
            return pairs;
        }
        self.traverse_pair(self.root, other, other.root, &mut pairs);
        pairs
    }

    /// Recursive tree-vs-tree traversal.
    fn traverse_pair(
        &self,
        a_idx: usize,
        other: &TriangleBvh,
        b_idx: usize,
        pairs: &mut Vec<(usize, usize)>,
    ) {
        let a = &self.nodes[a_idx];
        let b = &other.nodes[b_idx];

        if !a.aabb.intersects(&b.aabb) {
            return;
        }

        match (a.is_leaf(), b.is_leaf()) {
            (true, true) => {
                // Both leaves — record the pair (using source primitive indices).
                if let (Some(ai), Some(bi)) = (a.tri_idx, b.tri_idx) {
                    pairs.push((self.triangles[ai].index, other.triangles[bi].index));
                }
            }
            (true, false) => {
                // Descend into b's children.
                if let Some(bl) = b.left {
                    self.traverse_pair(a_idx, other, bl, pairs);
                }
                if let Some(br) = b.right {
                    self.traverse_pair(a_idx, other, br, pairs);
                }
            }
            (false, true) => {
                // Descend into a's children.
                if let Some(al) = a.left {
                    self.traverse_pair(al, other, b_idx, pairs);
                }
                if let Some(ar) = a.right {
                    self.traverse_pair(ar, other, b_idx, pairs);
                }
            }
            (false, false) => {
                // Descend into the larger node first.
                let a_area = a.aabb.surface_area();
                let b_area = b.aabb.surface_area();
                if a_area > b_area {
                    if let (Some(al), Some(ar)) = (a.left, a.right) {
                        self.traverse_pair(al, other, b_idx, pairs);
                        self.traverse_pair(ar, other, b_idx, pairs);
                    }
                } else if let (Some(bl), Some(br)) = (b.left, b.right) {
                    self.traverse_pair(a_idx, other, bl, pairs);
                    self.traverse_pair(a_idx, other, br, pairs);
                }
            }
        }
    }
}

/// Helper: return the centroid coordinate of a triangle along a given axis.
#[inline]
fn centroid_on_axis(tri: &Triangle, axis: usize) -> f32 {
    let c = tri.centroid();
    match axis {
        0 => c.x,
        1 => c.y,
        _ => c.z,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn tri(v0: Vec3, v1: Vec3, v2: Vec3, index: usize) -> Triangle {
        Triangle { v0, v1, v2, index }
    }

    // ------------------------------------------------------------------
    // AABB tests
    // ------------------------------------------------------------------

    #[test]
    fn aabb_intersection() {
        let a = Aabb {
            min: Vec3::ZERO,
            max: Vec3::ONE,
        };
        let b = Aabb {
            min: Vec3::splat(0.5),
            max: Vec3::splat(1.5),
        };
        assert!(a.intersects(&b));
        assert!(b.intersects(&a));
    }

    #[test]
    fn aabb_no_intersection() {
        let a = Aabb {
            min: Vec3::ZERO,
            max: Vec3::ONE,
        };
        let b = Aabb {
            min: Vec3::splat(2.0),
            max: Vec3::splat(3.0),
        };
        assert!(!a.intersects(&b));
        assert!(!b.intersects(&a));
    }

    #[test]
    fn aabb_from_triangle_basic() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(1.0, 0.0, 0.0);
        let c = Vec3::new(0.5, 1.0, 0.0);
        let aabb = Aabb::from_triangle(a, b, c);
        assert_eq!(aabb.min, Vec3::new(0.0, 0.0, 0.0));
        assert_eq!(aabb.max, Vec3::new(1.0, 1.0, 0.0));
    }

    #[test]
    fn aabb_expanded() {
        let a = Aabb {
            min: Vec3::ZERO,
            max: Vec3::ONE,
        };
        let exp = a.expanded(0.1);
        assert!((exp.min.x - (-0.1)).abs() < 1e-5);
        assert!((exp.max.x - 1.1).abs() < 1e-5);
    }

    #[test]
    fn aabb_surface_area_unit_cube() {
        let a = Aabb {
            min: Vec3::ZERO,
            max: Vec3::ONE,
        };
        // 6 faces × 1.0 each = 6
        assert!((a.surface_area() - 6.0).abs() < 1e-5);
    }

    // ------------------------------------------------------------------
    // BVH tests
    // ------------------------------------------------------------------

    #[test]
    fn bvh_find_overlapping_pairs() {
        // Two triangles that clearly overlap in space.
        let t0 = tri(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.5, 1.0, 0.0),
            0,
        );
        let t1 = tri(
            Vec3::new(0.2, 0.2, 0.0),
            Vec3::new(1.2, 0.2, 0.0),
            Vec3::new(0.7, 1.2, 0.0),
            1,
        );

        let bvh_a = TriangleBvh::build(&[t0]);
        let bvh_b = TriangleBvh::build(&[t1]);

        let pairs = bvh_a.find_overlapping_pairs(&bvh_b);
        assert_eq!(pairs.len(), 1, "expected one overlapping pair");
        assert_eq!(pairs[0], (0, 1));
    }

    #[test]
    fn bvh_no_overlap() {
        // Two triangles far apart.
        let t0 = tri(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.5, 1.0, 0.0),
            42,
        );
        let t1 = tri(
            Vec3::new(10.0, 10.0, 10.0),
            Vec3::new(11.0, 10.0, 10.0),
            Vec3::new(10.5, 11.0, 10.0),
            99,
        );

        let bvh_a = TriangleBvh::build(&[t0]);
        let bvh_b = TriangleBvh::build(&[t1]);

        let pairs = bvh_a.find_overlapping_pairs(&bvh_b);
        assert!(pairs.is_empty(), "expected no overlapping pairs");
    }

    #[test]
    fn bvh_many_triangles() {
        // Build two grids of 10×10 triangles that interpenetrate in 3D.
        // Each triangle has z-extent spanning [-0.5, 0.5] so the AABBs
        // of the two grids fully overlap in all three axes.
        //
        // Grid A: vertex z values centred at 0.
        // Grid B: vertex z values centred at 0 but offset in XY by (0.1, 0.1)
        //         so they are "interleaved" — still overlapping AABBs.
        //
        // The "far" grid is placed at z=1000 so its AABB doesn't touch A at all.

        let mut grid_a: Vec<Triangle> = Vec::new();
        let mut grid_b: Vec<Triangle> = Vec::new();
        let mut far: Vec<Triangle> = Vec::new();

        let n = 10usize;
        for row in 0..n {
            for col in 0..n {
                let x = col as f32;
                let y = row as f32;
                // Grid A — each triangle has three distinct z values so the AABB
                // gets z extent [-0.5, 0.5].
                grid_a.push(tri(
                    Vec3::new(x, y, -0.5),
                    Vec3::new(x + 1.0, y, 0.5),
                    Vec3::new(x + 0.5, y + 1.0, 0.0),
                    row * n + col,
                ));
                // Grid B — same XY cell, z range also [-0.5, 0.5] → overlaps A.
                grid_b.push(tri(
                    Vec3::new(x + 0.1, y + 0.1, -0.5),
                    Vec3::new(x + 1.1, y + 0.1, 0.5),
                    Vec3::new(x + 0.6, y + 1.1, 0.0),
                    row * n + col,
                ));
                // Far grid at z = 1000 — no overlaps expected with A.
                far.push(tri(
                    Vec3::new(x, y, 999.5),
                    Vec3::new(x + 1.0, y, 1000.5),
                    Vec3::new(x + 0.5, y + 1.0, 1000.0),
                    row * n + col,
                ));
            }
        }

        let bvh_a = TriangleBvh::build(&grid_a);
        let bvh_b = TriangleBvh::build(&grid_b);
        let bvh_far = TriangleBvh::build(&far);

        let pairs_ab = bvh_a.find_overlapping_pairs(&bvh_b);
        let pairs_af = bvh_a.find_overlapping_pairs(&bvh_far);

        // At minimum, every triangle in A should pair with the same-cell triangle
        // in B (diagonal overlap) — there must be at least n*n pairs.
        assert!(
            pairs_ab.len() >= n * n,
            "expected at least {} overlapping A×B pairs, got {}",
            n * n,
            pairs_ab.len()
        );

        // None of the far triangles should overlap A.
        assert!(
            pairs_af.is_empty(),
            "expected no A×far pairs, got {}",
            pairs_af.len()
        );
    }
}
