use serde::{Deserialize, Serialize};

use procgeo_core::{AttribClass, AttribDefault, Geometry, PointHandle, PrimHandle, TypeQualifier};

use crate::{Sop, SopError};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CopyToPointsParams;

pub struct CopyToPointsSop;

impl Sop for CopyToPointsSop {
    type Params = CopyToPointsParams;

    fn name(&self) -> &'static str {
        "copy_to_points"
    }

    fn input_count(&self) -> (usize, usize) {
        (2, 2)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;
        let _ = params;

        let source = inputs[0];
        let target = inputs[1];

        let src_npts = source.num_points();
        let src_nprims = source.num_prims();
        let tgt_npts = target.num_points();

        let total_points = src_npts * tgt_npts;
        let total_prims = src_nprims * tgt_npts;

        let mut out = Geometry::with_capacity(total_points, total_prims);

        // Add "copynum" int attribute on points
        out.add_attrib(
            AttribClass::Point,
            "copynum",
            AttribDefault::Int(0),
            TypeQualifier::None,
        )?;

        let copynum_handle = out.find_attrib::<i32>(AttribClass::Point, "copynum")?;

        for copy_idx in 0..tgt_npts {
            let target_pt = PointHandle::from_index(copy_idx);
            let target_pos = target.point_pos(target_pt);
            let point_offset = out.num_points();

            // Add all source points, offset by target position
            for src_pt_idx in 0..src_npts {
                let src_pt = PointHandle::from_index(src_pt_idx);
                let src_pos = source.point_pos(src_pt);
                let new_pt = out.add_point(src_pos + target_pos);
                out.set_attrib(&copynum_handle, new_pt.index(), copy_idx as i32)?;
            }

            // Add all source prims with remapped point indices
            for prim_idx in 0..src_nprims {
                let prim_handle = PrimHandle::from_index(prim_idx);
                let old_pts = source.prim_points(prim_handle);
                let new_pts: Vec<PointHandle> = old_pts
                    .iter()
                    .map(|ph| PointHandle::from_index(ph.index() + point_offset))
                    .collect();

                let prim = source.prim(prim_handle);
                match prim {
                    procgeo_core::Primitive::Polygon(poly) => {
                        use procgeo_core::PolyType;
                        match poly.poly_type {
                            PolyType::Closed => { out.add_face(&new_pts); }
                            PolyType::Open => { out.add_polyline(&new_pts); }
                        }
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
    use crate::creation::line::{LineSop, LineParams};
    use crate::generate;
    use glam::Vec3;

    fn make_box() -> Geometry {
        generate(&BoxSop, &BoxParams::default()).unwrap()
    }

    fn make_line(pts: u32) -> Geometry {
        generate(
            &LineSop,
            &LineParams {
                points: pts,
                origin: Vec3::ZERO,
                direction: Vec3::X,
                length: 4.0,
            },
        )
        .unwrap()
    }

    #[test]
    fn copy_box_to_line() {
        // Box: 8 pts, 6 prims. Line with 5 pts, 1 prim.
        // Copy box to line points → 8*5=40 pts, 6*5=30 prims
        let sop = CopyToPointsSop;
        let bx = make_box();
        let ln = make_line(5);
        let result = sop.execute(&[&bx, &ln], &CopyToPointsParams).unwrap();

        assert_eq!(result.num_points(), 40, "expected 40 points (8*5)");
        assert_eq!(result.num_prims(), 30, "expected 30 prims (6*5)");
    }

    #[test]
    fn copy_preserves_topology() {
        let sop = CopyToPointsSop;
        let bx = make_box();
        let ln = make_line(3);
        let result = sop.execute(&[&bx, &ln], &CopyToPointsParams).unwrap();

        // Each face should have 4 vertices (box faces are quads)
        for prim_idx in 0..result.num_prims() {
            let ph = PrimHandle::from_index(prim_idx);
            assert_eq!(result.prim_vertices(ph).len(), 4, "expected quad at prim {prim_idx}");
        }
    }

    #[test]
    fn copy_has_copynum() {
        let sop = CopyToPointsSop;
        let bx = make_box();
        let ln = make_line(3);
        let result = sop.execute(&[&bx, &ln], &CopyToPointsParams).unwrap();

        let handle = result.find_attrib::<i32>(AttribClass::Point, "copynum").unwrap();

        // First 8 points should have copynum=0, next 8 → 1, next 8 → 2
        for copy_idx in 0..3_usize {
            for local_pt in 0..8_usize {
                let global_idx = copy_idx * 8 + local_pt;
                let val = result.get_attrib(&handle, global_idx).unwrap();
                assert_eq!(val, copy_idx as i32, "point {global_idx} should have copynum={copy_idx}");
            }
        }
    }
}
