use std::collections::{HashMap, HashSet};

use glam::Vec3;
use serde::{Deserialize, Serialize};

use procgeo_core::{Geometry, PointHandle, PrimHandle};

use crate::{Sop, SopError};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SmoothParams {
    /// Number of smoothing iterations.
    pub iterations: u32,
    /// Blend strength: 0 = no change, 1 = full average.
    pub strength: f32,
}

impl Default for SmoothParams {
    fn default() -> Self {
        SmoothParams {
            iterations: 1,
            strength: 0.5,
        }
    }
}

pub struct SmoothSop;

/// Build a neighbor map: for each point index, the set of neighboring point indices
/// connected via shared polygon edges.
fn build_adjacency(geo: &Geometry) -> HashMap<usize, HashSet<usize>> {
    let mut adjacency: HashMap<usize, HashSet<usize>> = HashMap::new();

    // Initialize all points
    for i in 0..geo.num_points() {
        adjacency.entry(i).or_default();
    }

    for prim_idx in 0..geo.num_prims() {
        let ph = PrimHandle::from_index(prim_idx);
        let pts = geo.prim_points(ph);
        let n = pts.len();
        if n < 2 {
            continue;
        }
        // For each consecutive pair (and wrap around for closed polygons)
        for i in 0..n {
            let a = pts[i].index();
            let b = pts[(i + 1) % n].index();
            adjacency.entry(a).or_default().insert(b);
            adjacency.entry(b).or_default().insert(a);
        }
    }

    adjacency
}

impl Sop for SmoothSop {
    type Params = SmoothParams;

    fn name(&self) -> &'static str {
        "smooth"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 1)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;
        let mut out = inputs[0].clone();

        if params.iterations == 0 || params.strength == 0.0 {
            return Ok(out);
        }

        let adjacency = build_adjacency(&out);
        let num_pts = out.num_points();

        for _ in 0..params.iterations {
            // Compute all new positions simultaneously
            let new_positions: Vec<Vec3> = (0..num_pts)
                .map(|i| {
                    let current = out.point_pos(PointHandle::from_index(i));
                    let neighbors = &adjacency[&i];
                    if neighbors.is_empty() {
                        return current;
                    }
                    let avg: Vec3 = neighbors
                        .iter()
                        .map(|&j| out.point_pos(PointHandle::from_index(j)))
                        .sum::<Vec3>()
                        / neighbors.len() as f32;
                    current.lerp(avg, params.strength)
                })
                .collect();

            // Apply all new positions
            for (i, pos) in new_positions.into_iter().enumerate() {
                out.set_point_pos(PointHandle::from_index(i), pos);
            }
        }

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use crate::creation::box_sop::{BoxSop, BoxParams};
    use crate::reshape::subdivide::{SubdivideSop, SubdivideParams, SubdivideMode};
    use crate::{GeometryExt, generate};
    use glam::Vec3;

    fn make_subdivided_box() -> Geometry {
        let box_geo = generate(&BoxSop, &BoxParams::default()).unwrap();
        box_geo
            .apply(&SubdivideSop, &SubdivideParams { depth: 1, mode: SubdivideMode::Linear })
            .unwrap()
    }

    #[test]
    fn smooth_reduces_bbox() {
        let geo = make_subdivided_box();
        let bb_before = geo.bounding_box();

        let params = SmoothParams {
            iterations: 5,
            strength: 0.5,
        };
        let result = geo.apply(&SmoothSop, &params).unwrap();
        let bb_after = result.bounding_box();

        // After smoothing, the bbox should shrink (corners pulled inward)
        assert!(
            bb_after.max.x < bb_before.max.x,
            "max.x should shrink: {} < {}",
            bb_after.max.x,
            bb_before.max.x
        );
        assert!(
            bb_after.min.x > bb_before.min.x,
            "min.x should grow: {} > {}",
            bb_after.min.x,
            bb_before.min.x
        );
    }

    #[test]
    fn smooth_zero_strength() {
        let geo = make_subdivided_box();
        let num_pts = geo.num_points();

        // Collect original positions
        let orig_positions: Vec<Vec3> = (0..num_pts)
            .map(|i| geo.point_pos(PointHandle::from_index(i)))
            .collect();

        let params = SmoothParams {
            iterations: 10,
            strength: 0.0,
        };
        let result = geo.apply(&SmoothSop, &params).unwrap();

        // Positions should be unchanged
        for i in 0..num_pts {
            let pos = result.point_pos(PointHandle::from_index(i));
            assert_relative_eq!(pos.x, orig_positions[i].x, epsilon = 1e-5);
            assert_relative_eq!(pos.y, orig_positions[i].y, epsilon = 1e-5);
            assert_relative_eq!(pos.z, orig_positions[i].z, epsilon = 1e-5);
        }
    }
}
