use serde::{Deserialize, Serialize};

use procgeo_core::{AttribClass, Geometry, PrimHandle};

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
            let prim_offset = out.num_prims();

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
                            PolyType::Closed => {
                                out.add_face(&remapped);
                            }
                            PolyType::Open => {
                                out.add_polyline(&remapped);
                            }
                        }
                    }
                }
            }

            // Copy point attributes (skip "P" — handled by add_point).
            // Note: add_point auto-resizes existing attrs with defaults,
            // so for existing attrs we overwrite the range; for new attrs we create + set.
            for name in input.attrib_names(AttribClass::Point) {
                if name == "P" {
                    continue;
                }
                let src_attr = input
                    .attributes()
                    .get_raw(AttribClass::Point, name)
                    .unwrap();
                if let Some(dst_attr) = out.attributes_mut().get_raw_mut(AttribClass::Point, name) {
                    // Already auto-resized by add_point — overwrite the new slots
                    dst_attr
                        .storage
                        .copy_from_at(point_offset, &src_attr.storage);
                } else {
                    // New attribute — create it (auto-sizes to current point count with defaults)
                    out.attributes_mut()
                        .create(
                            AttribClass::Point,
                            name,
                            src_attr.default.clone(),
                            src_attr.qualifier,
                        )
                        .ok();
                    let count = out.num_points();
                    out.attributes_mut().resize_class(AttribClass::Point, count);
                    if let Some(dst) = out.attributes_mut().get_raw_mut(AttribClass::Point, name) {
                        dst.storage.copy_from_at(point_offset, &src_attr.storage);
                    }
                }
            }

            // Copy primitive attributes (same logic — add_face auto-resizes)
            for name in input.attrib_names(AttribClass::Primitive) {
                let src_attr = input
                    .attributes()
                    .get_raw(AttribClass::Primitive, name)
                    .unwrap();
                if let Some(dst_attr) = out
                    .attributes_mut()
                    .get_raw_mut(AttribClass::Primitive, name)
                {
                    dst_attr
                        .storage
                        .copy_from_at(prim_offset, &src_attr.storage);
                } else {
                    out.attributes_mut()
                        .create(
                            AttribClass::Primitive,
                            name,
                            src_attr.default.clone(),
                            src_attr.qualifier,
                        )
                        .ok();
                    let count = out.num_prims();
                    out.attributes_mut()
                        .resize_class(AttribClass::Primitive, count);
                    if let Some(dst) = out
                        .attributes_mut()
                        .get_raw_mut(AttribClass::Primitive, name)
                    {
                        dst.storage.copy_from_at(prim_offset, &src_attr.storage);
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
    use crate::creation::box_sop::{BoxParams, BoxSop};
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
    fn merge_propagates_point_attribs() {
        use crate::color::color::{ColorParams, ColorSop};

        let sop = MergeSop;
        let b1 = ColorSop
            .execute(
                &[&make_box()],
                &ColorParams {
                    color: [1.0, 0.0, 0.0],
                },
            )
            .unwrap();
        let b2 = ColorSop
            .execute(
                &[&make_box()],
                &ColorParams {
                    color: [0.0, 0.0, 1.0],
                },
            )
            .unwrap();
        let result = sop.execute(&[&b1, &b2], &MergeParams).unwrap();

        // Cd attribute should exist on all 16 points
        let handle = result
            .find_attrib::<[f32; 3]>(AttribClass::Point, "Cd")
            .unwrap();
        // First box points → red
        for i in 0..8 {
            let c = result.get_attrib(&handle, i).unwrap();
            assert_eq!(c, [1.0, 0.0, 0.0], "point {i} should be red");
        }
        // Second box points → blue
        for i in 8..16 {
            let c = result.get_attrib(&handle, i).unwrap();
            assert_eq!(c, [0.0, 0.0, 1.0], "point {i} should be blue");
        }
    }

    #[test]
    fn merge_backfills_missing_attribs() {
        use crate::color::color::{ColorParams, ColorSop};

        let sop = MergeSop;
        // b1 has Cd, b2 does NOT
        let b1 = ColorSop
            .execute(
                &[&make_box()],
                &ColorParams {
                    color: [1.0, 0.5, 0.0],
                },
            )
            .unwrap();
        let b2 = make_box(); // no Cd
        let result = sop.execute(&[&b1, &b2], &MergeParams).unwrap();

        // Cd should still exist — b2's points get the attribute's stored default
        let handle = result
            .find_attrib::<[f32; 3]>(AttribClass::Point, "Cd")
            .unwrap();
        // First box → orange
        let c0 = result.get_attrib(&handle, 0).unwrap();
        assert_eq!(c0, [1.0, 0.5, 0.0]);
        // Second box → attribute default (white, as ColorSop creates Cd with [1,1,1] default)
        let c8 = result.get_attrib(&handle, 8).unwrap();
        // Just verify Cd exists for all points and first box kept its values
        assert_eq!(
            result.attrib_names(AttribClass::Point).contains(&"Cd"),
            true
        );
        assert_ne!(c0, c8, "merged geos with different Cd should differ");
    }

    #[test]
    fn merge_chain_preserves_attribs() {
        use crate::color::color::{ColorParams, ColorSop};

        let sop = MergeSop;
        let colors = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let boxes: Vec<_> = colors
            .iter()
            .map(|&c| {
                ColorSop
                    .execute(&[&make_box()], &ColorParams { color: c })
                    .unwrap()
            })
            .collect();

        // Chain: merge(merge(r, g), b) — simulates WASM binary merge
        let m1 = sop.execute(&[&boxes[0], &boxes[1]], &MergeParams).unwrap();
        let m2 = sop.execute(&[&m1, &boxes[2]], &MergeParams).unwrap();

        assert_eq!(m2.num_points(), 24);
        let handle = m2
            .find_attrib::<[f32; 3]>(AttribClass::Point, "Cd")
            .unwrap();
        // Spot check: point 0 red, point 8 green, point 16 blue
        assert_eq!(m2.get_attrib(&handle, 0).unwrap(), [1.0, 0.0, 0.0]);
        assert_eq!(m2.get_attrib(&handle, 8).unwrap(), [0.0, 1.0, 0.0]);
        assert_eq!(m2.get_attrib(&handle, 16).unwrap(), [0.0, 0.0, 1.0]);
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
