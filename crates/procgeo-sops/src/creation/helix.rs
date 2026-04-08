use std::f32::consts::TAU;

use glam::Vec3;
use serde::{Deserialize, Serialize};

use procgeo_core::Geometry;

use crate::{Sop, SopError};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HelixParams {
    pub radius: f32,
    pub height: f32,
    pub turns: f32,
    pub points: u32,
    pub center: Vec3,
}

impl Default for HelixParams {
    fn default() -> Self {
        Self {
            radius: 0.5,
            height: 1.0,
            turns: 3.0,
            points: 96,
            center: Vec3::ZERO,
        }
    }
}

pub struct HelixSop;

impl Sop for HelixSop {
    type Params = HelixParams;

    fn name(&self) -> &'static str {
        "helix"
    }

    fn input_count(&self) -> (usize, usize) {
        (0, 0)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;

        if params.points < 2 {
            return Err(SopError::InvalidParam(format!(
                "points must be >= 2, got {}",
                params.points
            )));
        }
        if params.turns <= 0.0 {
            return Err(SopError::InvalidParam(format!(
                "turns must be > 0, got {}",
                params.turns
            )));
        }
        if params.radius < 0.0 {
            return Err(SopError::InvalidParam(format!(
                "radius must be >= 0, got {}",
                params.radius
            )));
        }

        let point_count = params.points as usize;
        let mut geo = Geometry::with_capacity(point_count, 1);
        let mut handles = Vec::with_capacity(point_count);

        for i in 0..point_count {
            let t = i as f32 / (point_count - 1) as f32;
            let angle = t * params.turns * TAU;
            let y = (t - 0.5) * params.height;
            let (sin_a, cos_a) = angle.sin_cos();
            let pos = params.center + Vec3::new(cos_a * params.radius, y, sin_a * params.radius);
            handles.push(geo.add_point(pos));
        }

        geo.add_polyline(&handles);
        Ok(geo)
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;
    use crate::generate;

    #[test]
    fn helix_default() {
        let geo = generate(&HelixSop, &HelixParams::default()).unwrap();

        assert_eq!(geo.num_points(), 96);
        assert_eq!(geo.num_prims(), 1);

        let bb = geo.bounding_box();
        assert_relative_eq!(bb.min.y, -0.5, epsilon = 1e-5);
        assert_relative_eq!(bb.max.y, 0.5, epsilon = 1e-5);
    }

    #[test]
    fn helix_custom_center() {
        let geo = generate(
            &HelixSop,
            &HelixParams {
                radius: 2.0,
                height: 4.0,
                turns: 2.0,
                points: 9,
                center: Vec3::new(1.0, 2.0, -3.0),
            },
        )
        .unwrap();

        let first = geo.point_pos(procgeo_core::PointHandle::from_index(0));
        let last = geo.point_pos(procgeo_core::PointHandle::from_index(8));
        assert_relative_eq!(first.x, 3.0, epsilon = 1e-5);
        assert_relative_eq!(first.y, 0.0, epsilon = 1e-5);
        assert_relative_eq!(first.z, -3.0, epsilon = 1e-5);
        assert_relative_eq!(last.x, 3.0, epsilon = 1e-5);
        assert_relative_eq!(last.y, 4.0, epsilon = 1e-5);
        assert_relative_eq!(last.z, -3.0, epsilon = 1e-5);
    }

    #[test]
    fn helix_rejects_invalid_params() {
        assert!(
            generate(
                &HelixSop,
                &HelixParams {
                    points: 1,
                    ..Default::default()
                }
            )
            .is_err()
        );
        assert!(
            generate(
                &HelixSop,
                &HelixParams {
                    turns: -1.0,
                    ..Default::default()
                }
            )
            .is_err()
        );
        assert!(
            generate(
                &HelixSop,
                &HelixParams {
                    radius: -0.5,
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
}
