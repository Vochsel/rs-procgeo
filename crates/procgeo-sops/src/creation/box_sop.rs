use glam::Vec3;
use serde::{Deserialize, Serialize};

use procgeo_core::Geometry;

use crate::{Sop, SopError};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BoxParams {
    pub size: Vec3,
    pub center: Vec3,
}

impl Default for BoxParams {
    fn default() -> Self {
        BoxParams {
            size: Vec3::ONE,
            center: Vec3::ZERO,
        }
    }
}

pub struct BoxSop;

impl Sop for BoxSop {
    type Params = BoxParams;

    fn name(&self) -> &'static str {
        "box"
    }

    fn input_count(&self) -> (usize, usize) {
        (0, 0)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;

        let c = params.center;
        let half = params.size * 0.5;

        let mut geo = Geometry::with_capacity(8, 6);

        // Bottom face points (y = -half.y), going around:
        // p0 = (-x, -y, -z)
        // p1 = (+x, -y, -z)
        // p2 = (+x, -y, +z)
        // p3 = (-x, -y, +z)
        let p0 = geo.add_point(c + Vec3::new(-half.x, -half.y, -half.z));
        let p1 = geo.add_point(c + Vec3::new(half.x, -half.y, -half.z));
        let p2 = geo.add_point(c + Vec3::new(half.x, -half.y, half.z));
        let p3 = geo.add_point(c + Vec3::new(-half.x, -half.y, half.z));

        // Top face points (y = +half.y)
        // p4 = (-x, +y, -z)
        // p5 = (+x, +y, -z)
        // p6 = (+x, +y, +z)
        // p7 = (-x, +y, +z)
        let p4 = geo.add_point(c + Vec3::new(-half.x, half.y, -half.z));
        let p5 = geo.add_point(c + Vec3::new(half.x, half.y, -half.z));
        let p6 = geo.add_point(c + Vec3::new(half.x, half.y, half.z));
        let p7 = geo.add_point(c + Vec3::new(-half.x, half.y, half.z));

        // Bottom face: normal pointing -Y (outward)
        geo.add_face(&[p0, p1, p2, p3]);
        // Top face: normal pointing +Y (outward)
        geo.add_face(&[p4, p7, p6, p5]);
        // Front face (-Z): normal pointing -Z
        geo.add_face(&[p0, p4, p5, p1]);
        // Back face (+Z): normal pointing +Z
        geo.add_face(&[p2, p6, p7, p3]);
        // Left face (-X): normal pointing -X
        geo.add_face(&[p0, p3, p7, p4]);
        // Right face (+X): normal pointing +X
        geo.add_face(&[p1, p5, p6, p2]);

        Ok(geo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate;
    use approx::assert_relative_eq;

    #[test]
    fn box_default() {
        let sop = BoxSop;
        let params = BoxParams::default();
        let geo = generate(&sop, &params).unwrap();

        assert_eq!(geo.num_points(), 8);
        assert_eq!(geo.num_prims(), 6);
        assert_eq!(geo.num_vertices(), 24);

        let bb = geo.bounding_box();
        assert_relative_eq!(bb.min.x, -0.5, epsilon = 1e-5);
        assert_relative_eq!(bb.min.y, -0.5, epsilon = 1e-5);
        assert_relative_eq!(bb.min.z, -0.5, epsilon = 1e-5);
        assert_relative_eq!(bb.max.x, 0.5, epsilon = 1e-5);
        assert_relative_eq!(bb.max.y, 0.5, epsilon = 1e-5);
        assert_relative_eq!(bb.max.z, 0.5, epsilon = 1e-5);
    }

    #[test]
    fn box_custom_size() {
        let sop = BoxSop;
        let params = BoxParams {
            size: Vec3::new(2.0, 4.0, 6.0),
            center: Vec3::ZERO,
        };
        let geo = generate(&sop, &params).unwrap();
        assert_eq!(geo.num_points(), 8);

        let bb = geo.bounding_box();
        assert_relative_eq!(bb.min.x, -1.0, epsilon = 1e-5);
        assert_relative_eq!(bb.max.x, 1.0, epsilon = 1e-5);
        assert_relative_eq!(bb.min.y, -2.0, epsilon = 1e-5);
        assert_relative_eq!(bb.max.y, 2.0, epsilon = 1e-5);
        assert_relative_eq!(bb.min.z, -3.0, epsilon = 1e-5);
        assert_relative_eq!(bb.max.z, 3.0, epsilon = 1e-5);
    }

    #[test]
    fn box_with_center() {
        let sop = BoxSop;
        let params = BoxParams {
            size: Vec3::ONE,
            center: Vec3::new(10.0, 5.0, -3.0),
        };
        let geo = generate(&sop, &params).unwrap();

        let bb = geo.bounding_box();
        assert_relative_eq!(bb.min.x, 9.5, epsilon = 1e-5);
        assert_relative_eq!(bb.max.x, 10.5, epsilon = 1e-5);
        assert_relative_eq!(bb.min.y, 4.5, epsilon = 1e-5);
        assert_relative_eq!(bb.max.y, 5.5, epsilon = 1e-5);
        assert_relative_eq!(bb.min.z, -3.5, epsilon = 1e-5);
        assert_relative_eq!(bb.max.z, -2.5, epsilon = 1e-5);
    }

    #[test]
    fn box_all_faces_are_quads() {
        let sop = BoxSop;
        let params = BoxParams::default();
        let geo = generate(&sop, &params).unwrap();

        for prim in geo.prims() {
            assert_eq!(prim.vertex_count(), 4, "Expected quad (4 verts)");
        }
    }

    #[test]
    fn box_rejects_inputs() {
        let sop = BoxSop;
        let params = BoxParams::default();
        let dummy = Geometry::new();
        let result = sop.execute(&[&dummy], &params);
        assert!(result.is_err());
        if let Err(SopError::WrongInputCount {
            expected_min,
            expected_max,
            got,
        }) = result
        {
            assert_eq!(expected_min, 0);
            assert_eq!(expected_max, 0);
            assert_eq!(got, 1);
        } else {
            panic!("Expected WrongInputCount error");
        }
    }
}
