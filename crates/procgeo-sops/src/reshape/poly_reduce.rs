use std::collections::HashMap;

use glam::Vec3;
use serde::{Deserialize, Serialize};

use procgeo_core::{Geometry, PointHandle, PrimHandle};

use crate::{Sop, SopError};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PolyReduceParams {
    /// Percentage of polygons to keep, 0.0 to 1.0.
    pub target_percent: f32,
    /// Whether to preserve unshared boundary edges from collapsing.
    pub preserve_boundaries: bool,
}

impl Default for PolyReduceParams {
    fn default() -> Self {
        PolyReduceParams {
            target_percent: 0.5,
            preserve_boundaries: true,
        }
    }
}

pub struct PolyReduceSop;

/// Follow the remap chain to find the canonical representative for a vertex.
fn resolve(remap: &[usize], mut v: usize) -> usize {
    while remap[v] != v {
        v = remap[v];
    }
    v
}

/// Check if a slice has duplicate values.
fn has_duplicates(vals: &[usize]) -> bool {
    for i in 0..vals.len() {
        for j in (i + 1)..vals.len() {
            if vals[i] == vals[j] {
                return true;
            }
        }
    }
    false
}

impl Sop for PolyReduceSop {
    type Params = PolyReduceParams;

    fn name(&self) -> &'static str {
        "poly_reduce"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 1)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;
        let geo = inputs[0];

        let num_points = geo.num_points();
        let num_prims = geo.num_prims();

        if num_prims == 0 || num_points == 0 {
            return Ok(geo.clone());
        }

        let percent = params.target_percent.clamp(0.0, 1.0);

        // Clone point positions into a mutable buffer
        let mut points: Vec<Vec3> = (0..num_points)
            .map(|i| geo.point_pos(PointHandle::from_index(i)))
            .collect();

        // Clone face topology: each face is a list of point indices
        let faces: Vec<Vec<usize>> = (0..num_prims)
            .map(|i| {
                let ph = PrimHandle::from_index(i);
                geo.prim_points(ph).iter().map(|h| h.index()).collect()
            })
            .collect();

        let mut alive_face = vec![true; faces.len()];
        let mut remap: Vec<usize> = (0..points.len()).collect();

        let target_faces = (faces.len() as f32 * percent).ceil().max(1.0) as usize;

        // Build edge list with costs (edge length).
        // Use canonical key (min, max) to deduplicate edges.
        let mut edge_set: HashMap<(usize, usize), f32> = HashMap::new();
        for face in &faces {
            let n = face.len();
            for i in 0..n {
                let a = face[i];
                let b = face[(i + 1) % n];
                let key = (a.min(b), a.max(b));
                edge_set
                    .entry(key)
                    .or_insert_with(|| (points[a] - points[b]).length());
            }
        }

        // Count how many faces use each edge (for boundary detection)
        let mut edge_face_count: HashMap<(usize, usize), usize> = HashMap::new();
        for face in &faces {
            let n = face.len();
            for i in 0..n {
                let a = face[i];
                let b = face[(i + 1) % n];
                let key = (a.min(b), a.max(b));
                *edge_face_count.entry(key).or_insert(0) += 1;
            }
        }

        // Build edges with costs; mark boundary edges with infinite cost if needed
        let mut edges: Vec<(usize, usize, f32)> = edge_set
            .into_iter()
            .map(|((a, b), cost)| {
                let mut c = cost;
                if params.preserve_boundaries {
                    if let Some(&count) = edge_face_count.get(&(a, b)) {
                        if count == 1 {
                            c = f32::MAX;
                        }
                    }
                }
                (a, b, c)
            })
            .collect();

        // Sort by cost ascending (cheapest collapses first)
        edges.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));

        // Collapse edges
        let mut alive_count = faces.len();
        for &(v1, v2, _cost) in &edges {
            if alive_count <= target_faces {
                break;
            }

            // Resolve through remap
            let a = resolve(&remap, v1);
            let b = resolve(&remap, v2);
            if a == b {
                continue; // already collapsed
            }

            // Place new vertex at midpoint
            points[a] = (points[a] + points[b]) * 0.5;

            // Remap b -> a
            remap[b] = a;

            // Kill degenerate faces
            for (i, face) in faces.iter().enumerate() {
                if !alive_face[i] {
                    continue;
                }
                let remapped: Vec<usize> = face.iter().map(|&v| resolve(&remap, v)).collect();
                if has_duplicates(&remapped) {
                    alive_face[i] = false;
                    alive_count -= 1;
                }
            }
        }

        // Rebuild geometry from surviving faces
        let mut out = Geometry::new();

        // Collect unique point indices used by surviving faces
        let mut used_points: Vec<bool> = vec![false; points.len()];
        let mut surviving_faces: Vec<Vec<usize>> = Vec::new();

        for (i, face) in faces.iter().enumerate() {
            if !alive_face[i] {
                continue;
            }
            let remapped: Vec<usize> = face.iter().map(|&v| resolve(&remap, v)).collect();
            // Skip degenerate faces (should already be killed, but be safe)
            if has_duplicates(&remapped) {
                continue;
            }
            for &idx in &remapped {
                used_points[idx] = true;
            }
            surviving_faces.push(remapped);
        }

        // Create point index mapping: old index -> new index
        let mut point_remap: Vec<Option<usize>> = vec![None; points.len()];
        for (old_idx, &used) in used_points.iter().enumerate() {
            if used {
                let new_idx = out.num_points();
                point_remap[old_idx] = Some(new_idx);
                out.add_point(points[old_idx]);
            }
        }

        // Add surviving faces with remapped indices
        for face in &surviving_faces {
            let new_pts: Vec<PointHandle> = face
                .iter()
                .filter_map(|&idx| point_remap[idx].map(PointHandle::from_index))
                .collect();
            if new_pts.len() >= 3 {
                out.add_face(&new_pts);
            }
        }

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creation::grid::{GridParams, GridSop};
    use crate::reshape::subdivide::{SubdivideMode, SubdivideParams, SubdivideSop};
    use crate::{GeometryExt, generate};

    fn make_subdivided_grid() -> Geometry {
        let grid = generate(
            &GridSop,
            &GridParams {
                rows: 5,
                cols: 5,
                size: [2.0, 2.0],
                ..Default::default()
            },
        )
        .unwrap();
        grid.apply(
            &SubdivideSop,
            &SubdivideParams {
                depth: 1,
                mode: SubdivideMode::Linear,
            },
        )
        .unwrap()
    }

    #[test]
    fn reduce_half() {
        let geo = make_subdivided_grid();
        let orig_prims = geo.num_prims();
        assert!(orig_prims > 4, "need enough prims to reduce");

        let params = PolyReduceParams {
            target_percent: 0.5,
            preserve_boundaries: false,
        };
        let result = geo.apply(&PolyReduceSop, &params).unwrap();

        assert!(
            result.num_prims() < orig_prims,
            "should have fewer prims: {} < {}",
            result.num_prims(),
            orig_prims
        );
        assert!(result.num_prims() > 0, "should still have some prims");
    }

    #[test]
    fn reduce_preserves_bounds() {
        let geo = make_subdivided_grid();
        let bb_before = geo.bounding_box();

        let params = PolyReduceParams {
            target_percent: 0.5,
            preserve_boundaries: true,
        };
        let result = geo.apply(&PolyReduceSop, &params).unwrap();
        let bb_after = result.bounding_box();

        // Bounding box should not grow significantly (midpoint placement can
        // only shrink the extent, never grow it)
        assert!(
            bb_after.max.x <= bb_before.max.x + 0.01,
            "max.x should not grow: {} <= {}",
            bb_after.max.x,
            bb_before.max.x
        );
        assert!(
            bb_after.min.x >= bb_before.min.x - 0.01,
            "min.x should not shrink: {} >= {}",
            bb_after.min.x,
            bb_before.min.x
        );
    }

    #[test]
    fn reduce_zero_keeps_all() {
        let geo = make_subdivided_grid();
        let orig_prims = geo.num_prims();

        let params = PolyReduceParams {
            target_percent: 1.0,
            preserve_boundaries: true,
        };
        let result = geo.apply(&PolyReduceSop, &params).unwrap();

        assert_eq!(
            result.num_prims(),
            orig_prims,
            "target_percent=1.0 should keep all prims"
        );
    }

    #[test]
    fn reduce_minimum() {
        let geo = make_subdivided_grid();

        let params = PolyReduceParams {
            target_percent: 0.01,
            preserve_boundaries: false,
        };
        let result = geo.apply(&PolyReduceSop, &params).unwrap();

        // Should still produce valid geometry
        // If any prims survive, they should have at least 3 points each
        if result.num_prims() > 0 {
            assert!(
                result.num_points() >= 3,
                "surviving geometry should have at least 3 points"
            );
        }
    }
}
