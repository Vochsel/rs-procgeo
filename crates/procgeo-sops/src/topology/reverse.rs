use serde::{Deserialize, Serialize};

use procgeo_core::{Geometry, PrimHandle, Primitive};

use crate::{Sop, SopError};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ReverseParams;

pub struct ReverseSop;

impl Sop for ReverseSop {
    type Params = ReverseParams;

    fn name(&self) -> &'static str {
        "reverse"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 1)
    }

    fn execute(&self, inputs: &[&Geometry], _params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;
        let mut out = inputs[0].clone();

        let num_prims = out.num_prims();
        for i in 0..num_prims {
            let ph = PrimHandle::from_index(i);
            let prim = out.prim_mut(ph);
            match prim {
                Primitive::Polygon(poly) => {
                    poly.vertices.reverse();
                }
            }
        }

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creation::grid::{GridOrientation, GridParams, GridSop};
    use crate::normals::normal::{NormalParams, NormalSop};
    use crate::{GeometryExt, generate};
    use approx::assert_relative_eq;
    use glam::Vec3;
    use procgeo_core::{AttribClass, Geometry};

    #[test]
    fn reverse_changes_winding() {
        // Create a triangle with known vertex order: p0, p1, p2
        let mut geo = Geometry::new();
        let p0 = geo.add_point(Vec3::new(0.0, 0.0, 0.0));
        let p1 = geo.add_point(Vec3::new(1.0, 0.0, 0.0));
        let p2 = geo.add_point(Vec3::new(0.0, 1.0, 0.0));
        geo.add_face(&[p0, p1, p2]);

        // Before reverse: points are [p0, p1, p2]
        let ph = PrimHandle::from_index(0);
        let before = geo.prim_points(ph);
        assert_eq!(before, vec![p0, p1, p2]);

        // After reverse: points should be [p2, p1, p0]
        let result = geo.apply(&ReverseSop, &ReverseParams).unwrap();
        let after = result.prim_points(ph);
        assert_eq!(after, vec![p2, p1, p0]);
    }

    #[test]
    fn reverse_inverts_normals() {
        // Build an XZ grid, compute normals, reverse, compute normals again
        // The normals after reverse should be opposite in sign
        let grid = generate(
            &GridSop,
            &GridParams {
                size: [2.0, 2.0],
                rows: 3,
                cols: 3,
                center: Vec3::ZERO,
                orientation: GridOrientation::XZ,
            },
        )
        .unwrap();

        // Normal before reverse
        let with_normals = grid
            .clone()
            .apply(&NormalSop, &NormalParams::default())
            .unwrap();
        let n_handle = with_normals
            .find_attrib::<[f32; 3]>(AttribClass::Point, "N")
            .unwrap();
        let normal_before: Vec3 = Vec3::from(with_normals.get_attrib(&n_handle, 4).unwrap());

        // Reverse and recompute normals
        let reversed = grid.apply(&ReverseSop, &ReverseParams).unwrap();
        let with_normals_after = reversed
            .apply(&NormalSop, &NormalParams::default())
            .unwrap();
        let n_handle2 = with_normals_after
            .find_attrib::<[f32; 3]>(AttribClass::Point, "N")
            .unwrap();
        let normal_after: Vec3 = Vec3::from(with_normals_after.get_attrib(&n_handle2, 4).unwrap());

        // Normals should be approximately opposite
        assert_relative_eq!(normal_before.x, -normal_after.x, epsilon = 1e-4);
        assert_relative_eq!(normal_before.y, -normal_after.y, epsilon = 1e-4);
        assert_relative_eq!(normal_before.z, -normal_after.z, epsilon = 1e-4);
    }
}
