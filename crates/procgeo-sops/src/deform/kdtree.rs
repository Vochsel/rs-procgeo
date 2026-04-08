//! Lightweight 3D KD-tree for spatial nearest-neighbor queries.
//!
//! Used by [`PointDeformSop`](super::point_deform_sop::PointDeformSop) for
//! capture-radius and k-nearest-neighbor searches on the rest lattice.

use glam::Vec3;

// ---------------------------------------------------------------------------
// Node layout
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct KdNode {
    /// Index into the original point array.
    point_idx: usize,
    /// Splitting axis (0 = x, 1 = y, 2 = z).
    axis: u8,
    left: Option<Box<KdNode>>,
    right: Option<Box<KdNode>>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// A static 3D KD-tree built over an immutable point set.
#[derive(Debug)]
pub struct KdTree {
    root: Option<Box<KdNode>>,
    points: Vec<Vec3>,
}

impl KdTree {
    /// Build a KD-tree from a slice of 3D positions.
    ///
    /// Complexity: O(n log n) expected.
    pub fn build(points: &[Vec3]) -> Self {
        let mut indices: Vec<usize> = (0..points.len()).collect();
        let root = Self::build_recursive(points, &mut indices, 0);
        KdTree {
            root,
            points: points.to_vec(),
        }
    }

    /// Return all points within `radius` of `query`.
    ///
    /// Each result is `(point_index, distance_squared)`.
    pub fn radius_search(&self, query: Vec3, radius: f32) -> Vec<(usize, f32)> {
        let mut results = Vec::new();
        let radius_sq = radius * radius;
        if let Some(ref root) = self.root {
            Self::radius_search_recursive(root, &self.points, query, radius_sq, &mut results);
        }
        results
    }

    /// Return the `k` nearest neighbors to `query`, sorted ascending by
    /// squared distance.
    ///
    /// Each result is `(point_index, distance_squared)`.
    pub fn k_nearest(&self, query: Vec3, k: usize) -> Vec<(usize, f32)> {
        if k == 0 || self.root.is_none() {
            return Vec::new();
        }
        // Max-heap by distance so we can cheaply pop the farthest candidate.
        let mut heap = BoundedMaxHeap::new(k);
        if let Some(ref root) = self.root {
            Self::knn_recursive(root, &self.points, query, &mut heap);
        }
        let mut results = heap.into_sorted_vec();
        results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        results
    }

    // -----------------------------------------------------------------------
    // Build helpers
    // -----------------------------------------------------------------------

    fn build_recursive(
        points: &[Vec3],
        indices: &mut [usize],
        depth: usize,
    ) -> Option<Box<KdNode>> {
        if indices.is_empty() {
            return None;
        }

        let axis = (depth % 3) as u8;

        // Partial sort to find the median element.
        let mid = indices.len() / 2;
        indices.select_nth_unstable_by(mid, |&a, &b| {
            let va = component(points[a], axis);
            let vb = component(points[b], axis);
            va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal)
        });

        let point_idx = indices[mid];

        let (left_indices, right_indices) = {
            let (left, rest) = indices.split_at_mut(mid);
            // rest[0] is the median — skip it for the right subtree.
            let right = &mut rest[1..];
            (left, right)
        };

        let left = Self::build_recursive(points, left_indices, depth + 1);
        let right = Self::build_recursive(points, right_indices, depth + 1);

        Some(Box::new(KdNode {
            point_idx,
            axis,
            left,
            right,
        }))
    }

    // -----------------------------------------------------------------------
    // Radius search
    // -----------------------------------------------------------------------

    fn radius_search_recursive(
        node: &KdNode,
        points: &[Vec3],
        query: Vec3,
        radius_sq: f32,
        results: &mut Vec<(usize, f32)>,
    ) {
        let pos = points[node.point_idx];
        let dist_sq = (pos - query).length_squared();
        if dist_sq <= radius_sq {
            results.push((node.point_idx, dist_sq));
        }

        let axis_val = component(pos, node.axis);
        let query_val = component(query, node.axis);
        let diff = query_val - axis_val;
        let diff_sq = diff * diff;

        // Determine near / far subtrees.
        let (near, far) = if diff <= 0.0 {
            (&node.left, &node.right)
        } else {
            (&node.right, &node.left)
        };

        if let Some(child) = near {
            Self::radius_search_recursive(child, points, query, radius_sq, results);
        }
        // Only descend into far subtree if the splitting plane is within radius.
        if diff_sq <= radius_sq {
            if let Some(child) = far {
                Self::radius_search_recursive(child, points, query, radius_sq, results);
            }
        }
    }

    // -----------------------------------------------------------------------
    // KNN search
    // -----------------------------------------------------------------------

    fn knn_recursive(node: &KdNode, points: &[Vec3], query: Vec3, heap: &mut BoundedMaxHeap) {
        let pos = points[node.point_idx];
        let dist_sq = (pos - query).length_squared();
        heap.push(node.point_idx, dist_sq);

        let axis_val = component(pos, node.axis);
        let query_val = component(query, node.axis);
        let diff = query_val - axis_val;
        let diff_sq = diff * diff;

        let (near, far) = if diff <= 0.0 {
            (&node.left, &node.right)
        } else {
            (&node.right, &node.left)
        };

        if let Some(child) = near {
            Self::knn_recursive(child, points, query, heap);
        }

        // Prune far subtree if splitting plane is farther than our worst candidate.
        if diff_sq < heap.worst_dist_sq() || !heap.is_full() {
            if let Some(child) = far {
                Self::knn_recursive(child, points, query, heap);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

#[inline]
fn component(v: Vec3, axis: u8) -> f32 {
    match axis {
        0 => v.x,
        1 => v.y,
        _ => v.z,
    }
}

// ---------------------------------------------------------------------------
// Bounded max-heap (for KNN)
// ---------------------------------------------------------------------------

/// A fixed-capacity max-heap that retains the `k` smallest distance entries.
#[derive(Debug)]
struct BoundedMaxHeap {
    k: usize,
    data: Vec<(usize, f32)>, // (index, dist_sq)
}

impl BoundedMaxHeap {
    fn new(k: usize) -> Self {
        BoundedMaxHeap {
            k,
            data: Vec::with_capacity(k + 1),
        }
    }

    fn is_full(&self) -> bool {
        self.data.len() >= self.k
    }

    fn worst_dist_sq(&self) -> f32 {
        if self.data.is_empty() {
            f32::INFINITY
        } else {
            self.data[0].1
        }
    }

    fn push(&mut self, idx: usize, dist_sq: f32) {
        if self.is_full() && dist_sq >= self.data[0].1 {
            return; // Farther than worst candidate, skip.
        }
        self.data.push((idx, dist_sq));
        self.sift_up(self.data.len() - 1);

        if self.data.len() > self.k {
            // Remove the root (largest).
            let last = self.data.len() - 1;
            self.data.swap(0, last);
            self.data.pop();
            if !self.data.is_empty() {
                self.sift_down(0);
            }
        }
    }

    fn into_sorted_vec(self) -> Vec<(usize, f32)> {
        self.data
    }

    // Max-heap sift-up (parent has larger dist_sq).
    fn sift_up(&mut self, mut idx: usize) {
        while idx > 0 {
            let parent = (idx - 1) / 2;
            if self.data[idx].1 > self.data[parent].1 {
                self.data.swap(idx, parent);
                idx = parent;
            } else {
                break;
            }
        }
    }

    // Max-heap sift-down.
    fn sift_down(&mut self, mut idx: usize) {
        let len = self.data.len();
        loop {
            let left = 2 * idx + 1;
            let right = 2 * idx + 2;
            let mut largest = idx;
            if left < len && self.data[left].1 > self.data[largest].1 {
                largest = left;
            }
            if right < len && self.data[right].1 > self.data[largest].1 {
                largest = right;
            }
            if largest != idx {
                self.data.swap(idx, largest);
                idx = largest;
            } else {
                break;
            }
        }
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn radius_search_basic() {
        let points = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(0.5, 0.5, 0.0),
            Vec3::new(10.0, 10.0, 10.0),
        ];
        let tree = KdTree::build(&points);

        // Radius 1.0 from origin should find point 0 (dist=0), point 1 (dist=1.0, on boundary),
        // and point 3 (dist=sqrt(0.5)≈0.707)
        let results = tree.radius_search(Vec3::ZERO, 1.0);
        let indices: Vec<usize> = results.iter().map(|r| r.0).collect();
        assert!(indices.contains(&0), "origin should be found");
        assert!(indices.contains(&3), "point (0.5,0.5,0) should be found");
        assert!(
            indices.contains(&1),
            "point at x=1 is on the boundary (dist_sq <= radius_sq) and should be included"
        );
        // Point 2 is at distance 2.0, should not be found.
        assert!(!indices.contains(&2), "point at x=2 should not be found");
        // Point 4 is far away.
        assert!(!indices.contains(&4), "far point should not be found");
    }

    #[test]
    fn knn_basic() {
        let points = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(3.0, 0.0, 0.0),
            Vec3::new(100.0, 0.0, 0.0),
        ];
        let tree = KdTree::build(&points);

        let results = tree.k_nearest(Vec3::new(0.5, 0.0, 0.0), 3);
        assert_eq!(results.len(), 3);
        // Closest should be point 0 (dist=0.5) and point 1 (dist=0.5), then point 2 (dist=1.5)
        let indices: Vec<usize> = results.iter().map(|r| r.0).collect();
        assert!(indices.contains(&0));
        assert!(indices.contains(&1));
        assert!(indices.contains(&2));

        // Verify sorted by distance
        for w in results.windows(2) {
            assert!(w[0].1 <= w[1].1, "results should be sorted by distance");
        }
    }

    #[test]
    fn empty_tree() {
        let tree = KdTree::build(&[]);
        let radius_results = tree.radius_search(Vec3::ZERO, 10.0);
        assert!(radius_results.is_empty());
        let knn_results = tree.k_nearest(Vec3::ZERO, 5);
        assert!(knn_results.is_empty());
    }

    #[test]
    fn knn_k_larger_than_points() {
        let points = vec![Vec3::new(1.0, 0.0, 0.0), Vec3::new(2.0, 0.0, 0.0)];
        let tree = KdTree::build(&points);
        let results = tree.k_nearest(Vec3::ZERO, 10);
        assert_eq!(results.len(), 2, "should return all points when k > n");
    }

    #[test]
    fn radius_search_exact_boundary() {
        // Point exactly at distance = radius (dist_sq == radius_sq) should be included.
        let points = vec![Vec3::new(1.0, 0.0, 0.0)];
        let tree = KdTree::build(&points);
        let results = tree.radius_search(Vec3::ZERO, 1.0);
        assert_eq!(results.len(), 1, "point on boundary should be included");
    }
}
