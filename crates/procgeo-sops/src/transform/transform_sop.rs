use glam::{EulerRot, Mat4, Vec3};
use serde::{Deserialize, Serialize};

use procgeo_core::{Geometry, PointHandle};

use crate::{Sop, SopError};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransformParams {
    pub translate: Vec3,
    /// Rotation in degrees (Euler XYZ)
    pub rotate: Vec3,
    pub scale: Vec3,
    pub pivot: Vec3,
}

impl Default for TransformParams {
    fn default() -> Self {
        TransformParams {
            translate: Vec3::ZERO,
            rotate: Vec3::ZERO,
            scale: Vec3::ONE,
            pivot: Vec3::ZERO,
        }
    }
}

pub struct TransformSop;

impl Sop for TransformSop {
    type Params = TransformParams;

    fn name(&self) -> &'static str {
        "transform"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 1)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;

        let mut geo = inputs[0].clone();

        // Build transform: translate * translate(pivot) * rotate(euler XYZ in radians) * scale * translate(-pivot)
        let rot_radians = params.rotate * std::f32::consts::PI / 180.0;

        let mat = Mat4::from_translation(params.translate)
            * Mat4::from_translation(params.pivot)
            * Mat4::from_euler(EulerRot::XYZ, rot_radians.x, rot_radians.y, rot_radians.z)
            * Mat4::from_scale(params.scale)
            * Mat4::from_translation(-params.pivot);

        let num_pts = geo.num_points();
        for i in 0..num_pts {
            let handle = PointHandle::from_index(i);
            let pos = geo.point_pos(handle);
            let new_pos = mat.transform_point3(pos);
            geo.set_point_pos(handle, new_pos);
        }

        Ok(geo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use crate::creation::box_sop::{BoxSop, BoxParams};
    use crate::{GeometryExt, generate};

    fn make_box() -> Geometry {
        generate(&BoxSop, &BoxParams::default()).unwrap()
    }

    #[test]
    fn translate() {
        let sop = TransformSop;
        let params = TransformParams {
            translate: Vec3::new(10.0, 0.0, 0.0),
            ..Default::default()
        };
        let box_geo = make_box();
        let result = box_geo.apply(&sop, &params).unwrap();

        let bb = result.bounding_box();
        // Box was centered at origin with ±0.5, now should be at x=10 ±0.5
        assert_relative_eq!(bb.min.x, 9.5, epsilon = 1e-4);
        assert_relative_eq!(bb.max.x, 10.5, epsilon = 1e-4);
    }

    #[test]
    fn scale() {
        let sop = TransformSop;
        let params = TransformParams {
            scale: Vec3::splat(2.0),
            ..Default::default()
        };
        let box_geo = make_box();
        let result = box_geo.apply(&sop, &params).unwrap();

        let bb = result.bounding_box();
        // Box was ±0.5, now should be ±1.0
        assert_relative_eq!(bb.min.x, -1.0, epsilon = 1e-4);
        assert_relative_eq!(bb.max.x,  1.0, epsilon = 1e-4);
    }

    #[test]
    fn identity() {
        let sop = TransformSop;
        let params = TransformParams::default();
        let box_geo = make_box();
        let original_bb = box_geo.bounding_box();
        let result = box_geo.apply(&sop, &params).unwrap();
        let result_bb = result.bounding_box();

        assert_relative_eq!(result_bb.min.x, original_bb.min.x, epsilon = 1e-5);
        assert_relative_eq!(result_bb.max.x, original_bb.max.x, epsilon = 1e-5);
        assert_relative_eq!(result_bb.min.y, original_bb.min.y, epsilon = 1e-5);
        assert_relative_eq!(result_bb.max.y, original_bb.max.y, epsilon = 1e-5);
    }

    #[test]
    fn requires_input() {
        let sop = TransformSop;
        let params = TransformParams::default();
        let result = sop.execute(&[], &params);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SopError::WrongInputCount { .. }));
    }
}
