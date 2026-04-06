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

        // Build merge map: for each point i, find lowest-index j (j < i) within distance
        // If none found, map[i] = i (point maps to itself)
        let mut merge_map: Vec<usize> = (0..num_pts).collect();

        for i in 0..num_pts {
            let pi = geo.point_pos(PointHandle::from_index(i));
            for j in 0..i {
                let pj = geo.point_pos(PointHandle::from_index(j));
                let diff = pi - pj;
                if diff.dot(diff) <= dist_sq {
                    merge_map[i] = j;
                    break;
                }
            }
        }

        // Follow chains: if i→j and j→k, then i→k
        // Iterate until stable (max depth is num_pts, but typically short chains)
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
        // First follow merge_map to find representative, then look up new_index
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
