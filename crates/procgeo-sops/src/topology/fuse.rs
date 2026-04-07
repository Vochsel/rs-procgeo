use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use procgeo_core::{Geometry, PointHandle, PrimHandle};

use crate::{Sop, SopError};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FuseParams {
    /// Maximum distance between two points to be considered coincident.
    pub distance: f32,
}

impl Default for FuseParams {
    fn default() -> Self {
        FuseParams { distance: 0.001 }
    }
}

pub struct FuseSop;

/// Spatial hash cell key for a 3D grid.
#[derive(Hash, Eq, PartialEq, Clone, Copy)]
struct CellKey(i32, i32, i32);

impl CellKey {
    fn from_pos(pos: glam::Vec3, inv_cell: f32) -> Self {
        Self(
            (pos.x * inv_cell).floor() as i32,
            (pos.y * inv_cell).floor() as i32,
            (pos.z * inv_cell).floor() as i32,
        )
    }
}

impl Sop for FuseSop {
    type Params = FuseParams;

    fn name(&self) -> &'static str {
        "fuse"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 1)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;
        let geo = inputs[0];

        let num_pts = geo.num_points();
        let dist_sq = params.distance * params.distance;
        let cell_size = params.distance.max(f32::EPSILON);
        let inv_cell = 1.0 / cell_size;

        // Build merge map using spatial hash grid for O(n) average lookup.
        // For each point, check only the 27 neighboring cells (3x3x3) instead
        // of all previous points.
        let mut merge_map: Vec<usize> = (0..num_pts).collect();
        let mut grid: HashMap<CellKey, Vec<usize>> =
            HashMap::with_capacity(num_pts.min(1 << 20));

        for (i, entry) in merge_map.iter_mut().enumerate() {
            let pi = geo.point_pos(PointHandle::from_index(i));
            let ci = CellKey::from_pos(pi, inv_cell);
            let mut merged = false;

            // Check 27 neighboring cells
            'outer: for dz in -1..=1i32 {
                for dy in -1..=1i32 {
                    for dx in -1..=1i32 {
                        let neighbor = CellKey(ci.0 + dx, ci.1 + dy, ci.2 + dz);
                        if let Some(bucket) = grid.get(&neighbor) {
                            for &j in bucket {
                                let pj = geo.point_pos(PointHandle::from_index(j));
                                let diff = pi - pj;
                                if diff.dot(diff) <= dist_sq {
                                    *entry = j;
                                    merged = true;
                                    break 'outer;
                                }
                            }
                        }
                    }
                }
            }

            // Only insert into grid if this point is a representative (not merged)
            if !merged {
                grid.entry(ci).or_default().push(i);
            }
        }

        // Follow chains: if i→j and j→k, then i→k (path compression)
        for i in 0..num_pts {
            let mut current = merge_map[i];
            while merge_map[current] != current {
                current = merge_map[current];
            }
            merge_map[i] = current;
        }

        // Track which points survive (those that map to themselves)
        let mut surviving: Vec<bool> = vec![false; num_pts];
        for i in 0..num_pts {
            if merge_map[i] == i {
                surviving[i] = true;
            }
        }

        // Build new geometry: add only surviving points
        let mut out = Geometry::new();
        let mut new_index: Vec<usize> = vec![0; num_pts];

        for i in 0..num_pts {
            if surviving[i] {
                let pos = geo.point_pos(PointHandle::from_index(i));
                let new_pt = out.add_point(pos);
                new_index[i] = new_pt.index();
            }
        }

        // Final remap: old_pt_idx -> new geometry index
        let final_remap: Vec<usize> = (0..num_pts)
            .map(|i| new_index[merge_map[i]])
            .collect();

        // Add prims with remapped point indices
        for prim_idx in 0..geo.num_prims() {
            let ph = PrimHandle::from_index(prim_idx);
            let old_pts = geo.prim_points(ph);
            let new_pts: Vec<PointHandle> = old_pts
                .iter()
                .map(|&h| PointHandle::from_index(final_remap[h.index()]))
                .collect();

            let prim = geo.prim(ph);
            match prim {
                procgeo_core::Primitive::Polygon(poly) => match poly.poly_type {
                    procgeo_core::PolyType::Closed => {
                        out.add_face(&new_pts);
                    }
                    procgeo_core::PolyType::Open => {
                        out.add_polyline(&new_pts);
                    }
                },
            }
        }

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creation::box_sop::{BoxSop, BoxParams};
    use crate::merge::{MergeSop, MergeParams};
    use crate::{GeometryExt, generate};
    use glam::Vec3;

    fn make_box() -> Geometry {
        generate(&BoxSop, &BoxParams::default()).unwrap()
    }

    #[test]
    fn fuse_coincident() {
        // Merge two boxes at the same position → should reduce from 16 to 8 unique points
        let box1 = make_box();
        let box2 = make_box();
        let merged = MergeSop.execute(&[&box1, &box2], &MergeParams).unwrap();
        assert_eq!(merged.num_points(), 16);

        let params = FuseParams { distance: 0.001 };
        let result = merged.apply(&FuseSop, &params).unwrap();

        assert_eq!(result.num_points(), 8, "expected 8 unique points after fuse");
        assert_eq!(result.num_prims(), 12, "expected 12 prims (6+6) after fuse");
    }

    #[test]
    fn fuse_tolerance() {
        // Two points at distance 0.01
        let mut geo = Geometry::new();
        geo.add_point(Vec3::new(0.0, 0.0, 0.0));
        geo.add_point(Vec3::new(0.01, 0.0, 0.0));

        // Fuse with tolerance 0.001 → not fused
        let params_small = FuseParams { distance: 0.001 };
        let result_small = geo.clone().apply(&FuseSop, &params_small).unwrap();
        assert_eq!(result_small.num_points(), 2, "should not fuse with small tolerance");

        // Fuse with tolerance 0.1 → fused
        let params_large = FuseParams { distance: 0.1 };
        let result_large = geo.apply(&FuseSop, &params_large).unwrap();
        assert_eq!(result_large.num_points(), 1, "should fuse with large tolerance");
    }
}
