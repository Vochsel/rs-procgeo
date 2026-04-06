use serde::{Deserialize, Serialize};

use procgeo_core::{Geometry, PrimHandle};

use crate::{Sop, SopError};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MergeParams;

pub struct MergeSop;

impl Sop for MergeSop {
    type Params = MergeParams;

    fn name(&self) -> &'static str {
        "merge"
    }

    fn input_count(&self) -> (usize, usize) {
        (0, usize::MAX)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;
        let _ = params;

        let mut out = Geometry::new();

        for &input in inputs {
            let point_offset = out.num_points();

            // Copy all points from this input
            for pt in input.points() {
                out.add_point(pt);
            }

            // Copy all primitives with remapped point indices
            for prim_idx in 0..input.num_prims() {
                let prim_handle = PrimHandle::from_index(prim_idx);
                let pt_handles = input.prim_points(prim_handle);

                // Remap point handles by adding the offset
                let remapped: Vec<_> = pt_handles
                    .iter()
                    .map(|ph| {
                        use procgeo_core::PointHandle;
                        PointHandle::from_index(ph.index() + point_offset)
                    })
                    .collect();

                // Determine if it's open or closed by checking the primitive type
                let prim = input.prim(prim_handle);
                match prim {
                    procgeo_core::Primitive::Polygon(poly) => {
                        use procgeo_core::PolyType;
                        match poly.poly_type {
                            PolyType::Closed => { out.add_face(&remapped); }
                            PolyType::Open => { out.add_polyline(&remapped); }
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
    use crate::generate;

    fn make_box() -> Geometry {
        generate(&BoxSop, &BoxParams::default()).unwrap()
    }

    #[test]
    fn merge_two_boxes() {
        let sop = MergeSop;
        let b1 = make_box();
        let b2 = make_box();
        let result = sop.execute(&[&b1, &b2], &MergeParams).unwrap();

        assert_eq!(result.num_points(), 16);
        assert_eq!(result.num_prims(), 12);
    }

    #[test]
    fn merge_empty() {
        let sop = MergeSop;
        let result = sop.execute(&[], &MergeParams).unwrap();

        assert_eq!(result.num_points(), 0);
        assert_eq!(result.num_prims(), 0);
    }

    #[test]
    fn merge_single() {
        let sop = MergeSop;
        let b = make_box();
        let result = sop.execute(&[&b], &MergeParams).unwrap();

        assert_eq!(result.num_points(), 8);
        assert_eq!(result.num_prims(), 6);
    }

    #[test]
    fn merge_preserves_topology() {
        let sop = MergeSop;
        let b1 = make_box();
        let b2 = make_box();
        let result = sop.execute(&[&b1, &b2], &MergeParams).unwrap();

        // All primitives from second box should reference points 8..16
        for prim_idx in 6..12 {
            let prim_handle = PrimHandle::from_index(prim_idx);
            let pt_handles = result.prim_points(prim_handle);
            for ph in pt_handles {
                assert!(
                    ph.index() >= 8,
                    "Second box prim has point index {}, expected >= 8",
                    ph.index()
                );
            }
        }
    }
}
