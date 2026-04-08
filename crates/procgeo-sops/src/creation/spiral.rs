use std::f32::consts::TAU;

use glam::Vec3;
use serde::{Deserialize, Serialize};

use procgeo_core::Geometry;

use crate::{Sop, SopError};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpiralParams {
    pub start_radius: f32,
    pub end_radius: f32,
    pub height: f32,
    pub turns: f32,
    pub points: u32,
    pub center: Vec3,
}

impl Default for SpiralParams {
    fn default() -> Self {
        Self {
            start_radius: 0.0,
            end_radius: 1.0,
            height: 0.0,
            turns: 3.0,
            points: 96,
            center: Vec3::ZERO,
        }
    }
}

pub struct SpiralSop;

impl Sop for SpiralSop {
    type Params = SpiralParams;

    fn name(&self) -> &'static str {
        "spiral"
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
        if params.start_radius < 0.0 || params.end_radius < 0.0 {
            return Err(SopError::InvalidParam("radii must be >= 0".to_string()));
        }

        let point_count = params.points as usize;
        let mut geo = Geometry::with_capacity(point_count, 1);
        let mut handles = Vec::with_capacity(point_count);

        for i in 0..point_count {
            let t = i as f32 / (point_count - 1) as f32;
            let angle = t * params.turns * TAU;
            let radius = params.start_radius + (params.end_radius - params.start_radius) * t;
            let y = (t - 0.5) * params.height;
            let (sin_a, cos_a) = angle.sin_cos();
            let pos = params.center + Vec3::new(cos_a * radius, y, sin_a * radius);
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
    use procgeo_core::{PolyType, PrimHandle, Primitive};

    #[test]
    fn spiral_default() {
        let geo = generate(&SpiralSop, &SpiralParams::default()).unwrap();

        assert_eq!(geo.num_points(), 96);
        assert_eq!(geo.num_prims(), 1);

        let first = geo.point_pos(procgeo_core::PointHandle::from_index(0));
        let last = geo.point_pos(procgeo_core::PointHandle::from_index(95));
        assert_relative_eq!(first.length(), 0.0, epsilon = 1e-6);
        assert_relative_eq!(last.x, 1.0, epsilon = 1e-4);
        assert_relative_eq!(last.y, 0.0, epsilon = 1e-6);
        assert_relative_eq!(last.z, 0.0, epsilon = 1e-4);
    }

    #[test]
    fn spiral_is_open_polyline() {
        let geo = generate(&SpiralSop, &SpiralParams::default()).unwrap();

        match geo.prim(PrimHandle::from_index(0)) {
            Primitive::Polygon(poly) => assert_eq!(poly.poly_type, PolyType::Open),
        }

        let pts = geo.prim_points(PrimHandle::from_index(0));
        assert_ne!(pts.first(), pts.last());
    }

    #[test]
    fn spiral_custom_radii_and_height() {
        let geo = generate(
            &SpiralSop,
            &SpiralParams {
                start_radius: 0.5,
                end_radius: 2.0,
                height: 3.0,
                turns: 1.0,
                points: 5,
                center: Vec3::new(1.0, 2.0, -1.0),
            },
        )
        .unwrap();

        let first = geo.point_pos(procgeo_core::PointHandle::from_index(0));
        let last = geo.point_pos(procgeo_core::PointHandle::from_index(4));
        assert_relative_eq!(first.x, 1.5, epsilon = 1e-5);
        assert_relative_eq!(first.y, 0.5, epsilon = 1e-5);
        assert_relative_eq!(first.z, -1.0, epsilon = 1e-5);
        assert_relative_eq!(last.x, 3.0, epsilon = 1e-5);
        assert_relative_eq!(last.y, 3.5, epsilon = 1e-5);
        assert_relative_eq!(last.z, -1.0, epsilon = 1e-5);
    }

    #[test]
    fn spiral_rejects_invalid_params() {
        assert!(
            generate(
                &SpiralSop,
                &SpiralParams {
                    points: 1,
                    ..Default::default()
                }
            )
            .is_err()
        );
        assert!(
            generate(
                &SpiralSop,
                &SpiralParams {
                    turns: 0.0,
                    ..Default::default()
                }
            )
            .is_err()
        );
        assert!(
            generate(
                &SpiralSop,
                &SpiralParams {
                    end_radius: -1.0,
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
}
