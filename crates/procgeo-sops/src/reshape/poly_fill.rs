use std::collections::{HashMap, HashSet};

use glam::Vec3;
use serde::{Deserialize, Serialize};

use procgeo_core::{Geometry, PointHandle, PrimHandle};

use crate::{Sop, SopError};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub enum PolyFillMode {
    /// One N-gon per hole.
    #[default]
    SinglePolygon,
    /// Center point + triangle fan per hole.
    TriangleFan,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PolyFillParams {
    /// Fill strategy.
    pub mode: PolyFillMode,
    /// Smoothing factor for fill surface, 0.0 to 1.0.
    pub smooth: f32,
}

impl Default for PolyFillParams {
    fn default() -> Self {
        PolyFillParams {
            mode: PolyFillMode::SinglePolygon,
            smooth: 0.0,
        }
    }
}

pub struct PolyFillSop;

/// Find boundary edge loops in the geometry.
/// A boundary edge is one used by exactly one face.
/// Returns a list of loops, each being an ordered list of point indices.
fn find_boundary_loops(geo: &Geometry) -> Vec<Vec<usize>> {
    let num_prims = geo.num_prims();

    // Build edge use counts.
    // We store directed edges per face, then count the canonical (min,max) key.
    let mut edge_count: HashMap<(usize, usize), usize> = HashMap::new();
    // Also store adjacency for boundary edges: for each point, which other
    // boundary-edge endpoints connect to it.
    let mut all_edges: Vec<(usize, usize)> = Vec::new();

    for prim_idx in 0..num_prims {
        let ph = PrimHandle::from_index(prim_idx);
        let pts = geo.prim_points(ph);
        let n = pts.len();
        for i in 0..n {
            let a = pts[i].index();
            let b = pts[(i + 1) % n].index();
            let key = (a.min(b), a.max(b));
            *edge_count.entry(key).or_insert(0) += 1;
            all_edges.push(key);
        }
    }

    // Collect boundary edges (count == 1)
    let boundary_edges: HashSet<(usize, usize)> = edge_count
        .iter()
        .filter(|&(_, &count)| count == 1)
        .map(|(&key, _)| key)
        .collect();

    if boundary_edges.is_empty() {
        return Vec::new();
    }

    // Build adjacency map for boundary edges: point -> set of connected points via boundary edges
    let mut boundary_adj: HashMap<usize, Vec<usize>> = HashMap::new();
    for &(a, b) in &boundary_edges {
        boundary_adj.entry(a).or_default().push(b);
        boundary_adj.entry(b).or_default().push(a);
    }

    // Chain boundary edges into loops
    let mut used: HashSet<(usize, usize)> = HashSet::new();
    let mut loops: Vec<Vec<usize>> = Vec::new();

    for &(ea, eb) in &boundary_edges {
        let key = (ea.min(eb), ea.max(eb));
        if used.contains(&key) {
            continue;
        }

        // Start a new loop from this edge
        let mut loop_pts = vec![ea, eb];
        used.insert(key);

        loop {
            let last = *loop_pts.last().unwrap();

            // Find the next boundary edge connected to `last` that isn't used
            let mut found_next = false;
            if let Some(neighbors) = boundary_adj.get(&last) {
                for &neighbor in neighbors {
                    let edge_key = (last.min(neighbor), last.max(neighbor));
                    if used.contains(&edge_key) {
                        continue;
                    }
                    used.insert(edge_key);

                    if neighbor == loop_pts[0] {
                        // Loop closed
                        found_next = true;
                        break;
                    }

                    loop_pts.push(neighbor);
                    found_next = true;
                    break;
                }
            }

            if !found_next {
                break; // Open boundary or loop closed
            }

            // Check if we just closed the loop (the neighbor == loop_pts[0] case
            // already broke above, but check if last added == first)
            if *loop_pts.last().unwrap() == loop_pts[0] {
                loop_pts.pop(); // Remove duplicate closing point
                break;
            }

            // Also check if we found the closing edge
            let last_pt = *loop_pts.last().unwrap();
            if let Some(neighbors) = boundary_adj.get(&last_pt) {
                let close_key = (last_pt.min(loop_pts[0]), last_pt.max(loop_pts[0]));
                if boundary_edges.contains(&close_key) && !used.contains(&close_key) {
                    // The next step would close the loop; check if there's nothing else
                    // We don't close here — let the next iteration handle it
                }
                let _ = neighbors; // suppress unused warning
            }
        }

        if loop_pts.len() >= 3 {
            loops.push(loop_pts);
        }
    }

    loops
}

impl Sop for PolyFillSop {
    type Params = PolyFillParams;

    fn name(&self) -> &'static str {
        "poly_fill"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 1)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;
        let geo = inputs[0];

        // Clone the input geometry: copy all points and faces
        let mut out = Geometry::new();

        let num_points = geo.num_points();
        let mut orig_handles: Vec<PointHandle> = Vec::with_capacity(num_points);
        for i in 0..num_points {
            let ph = PointHandle::from_index(i);
            orig_handles.push(out.add_point(geo.point_pos(ph)));
        }

        for prim_idx in 0..geo.num_prims() {
            let ph = PrimHandle::from_index(prim_idx);
            let pts = geo.prim_points(ph);
            let new_pts: Vec<PointHandle> = pts
                .iter()
                .map(|&p| orig_handles[p.index()])
                .collect();
            out.add_face(&new_pts);
        }

        // Find boundary loops on the input geometry
        let loops = find_boundary_loops(geo);

        if loops.is_empty() {
            return Ok(out);
        }

        // Fill each loop
        for loop_pts in &loops {
            match params.mode {
                PolyFillMode::SinglePolygon => {
                    let handles: Vec<PointHandle> = loop_pts
                        .iter()
                        .map(|&i| orig_handles[i])
                        .collect();
                    out.add_face(&handles);
                }
                PolyFillMode::TriangleFan => {
                    // Compute centroid of loop points
                    let center: Vec3 = loop_pts
                        .iter()
                        .map(|&i| geo.point_pos(PointHandle::from_index(i)))
                        .sum::<Vec3>()
                        / loop_pts.len() as f32;

                    // Optionally smooth the center toward the interior
                    let center = if params.smooth > 0.0 {
                        // Simple smoothing: blend centroid toward average of adjacent face centroids
                        // For now, just use the centroid as-is (smooth=0 has no effect on placement)
                        center
                    } else {
                        center
                    };

                    let center_handle = out.add_point(center);

                    for i in 0..loop_pts.len() {
                        let next = (i + 1) % loop_pts.len();
                        let a = orig_handles[loop_pts[i]];
                        let b = orig_handles[loop_pts[next]];
                        out.add_face(&[a, b, center_handle]);
                    }
                }
            }
        }

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creation::box_sop::{BoxSop, BoxParams};
    use crate::{GeometryExt, generate};

    /// Create a box with one face removed (creating a hole).
    fn box_with_hole() -> Geometry {
        let bx = generate(&BoxSop, &BoxParams::default()).unwrap();
        let keep: Vec<bool> = (0..bx.num_prims()).map(|i| i != 0).collect();
        bx.rebuild_keeping_prims(&keep)
    }

    #[test]
    fn fill_single_polygon() {
        let geo = box_with_hole();
        assert_eq!(geo.num_prims(), 5, "box with one face removed should have 5 prims");

        let params = PolyFillParams {
            mode: PolyFillMode::SinglePolygon,
            ..Default::default()
        };
        let result = geo.apply(&PolyFillSop, &params).unwrap();

        // 5 original faces + 1 fill polygon = 6
        assert_eq!(
            result.num_prims(),
            6,
            "expected 5 + 1 fill = 6 prims, got {}",
            result.num_prims()
        );
    }

    #[test]
    fn fill_triangle_fan() {
        let geo = box_with_hole();
        assert_eq!(geo.num_prims(), 5);

        let params = PolyFillParams {
            mode: PolyFillMode::TriangleFan,
            ..Default::default()
        };
        let result = geo.apply(&PolyFillSop, &params).unwrap();

        // The hole from removing one quad face has 4 boundary edges forming a loop of 4 points.
        // TriangleFan mode creates 4 triangles from that loop.
        // Total: 5 original + 4 fan triangles = 9
        assert_eq!(
            result.num_prims(),
            9,
            "expected 5 + 4 fan triangles = 9 prims, got {}",
            result.num_prims()
        );
    }

    #[test]
    fn fill_no_holes() {
        // A fully closed box has no boundary edges
        let geo = generate(&BoxSop, &BoxParams::default()).unwrap();
        let orig_prims = geo.num_prims();
        let orig_points = geo.num_points();

        let params = PolyFillParams::default();
        let result = geo.apply(&PolyFillSop, &params).unwrap();

        assert_eq!(
            result.num_prims(),
            orig_prims,
            "closed mesh should pass through unchanged"
        );
        assert_eq!(
            result.num_points(),
            orig_points,
            "closed mesh point count should be unchanged"
        );
    }

    #[test]
    fn fill_preserves_existing() {
        let geo = box_with_hole();
        let orig_prims = geo.num_prims();
        let orig_points = geo.num_points();

        let params = PolyFillParams {
            mode: PolyFillMode::SinglePolygon,
            ..Default::default()
        };
        let result = geo.apply(&PolyFillSop, &params).unwrap();

        // The result should have at least the original prim count
        assert!(
            result.num_prims() >= orig_prims,
            "should preserve all original faces: {} >= {}",
            result.num_prims(),
            orig_prims
        );

        // Verify point count: original points should all be present
        // SinglePolygon mode doesn't add new points
        assert_eq!(
            result.num_points(),
            orig_points,
            "SinglePolygon fill should not add new points"
        );
    }
}
