use std::f32::consts::TAU;

use glam::Vec3;
use serde::{Deserialize, Serialize};

use procgeo_core::Geometry;

use super::grid::GridOrientation;
use crate::{Sop, SopError};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CircleParams {
    pub radius: f32,
    pub center: Vec3,
    pub divisions: u32,
    pub orientation: GridOrientation,
}

impl Default for CircleParams {
    fn default() -> Self {
        CircleParams {
            radius: 1.0,
            center: Vec3::ZERO,
            divisions: 40,
            orientation: GridOrientation::XZ,
        }
    }
}

pub struct CircleSop;

impl Sop for CircleSop {
    type Params = CircleParams;

    fn name(&self) -> &'static str {
        "circle"
    }

    fn input_count(&self) -> (usize, usize) {
        (0, 0)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;

        if params.divisions < 3 {
            return Err(SopError::InvalidParam(format!(
                "divisions must be >= 3, got {}",
                params.divisions
            )));
        }

        let n = params.divisions as usize;
        let mut geo = Geometry::with_capacity(n, 1);

        let mut handles = Vec::with_capacity(n);
        for i in 0..n {
            let angle = TAU * i as f32 / n as f32;
            let (s, c_) = angle.sin_cos();

            let pos = match params.orientation {
                GridOrientation::XZ => {
                    params.center + Vec3::new(c_ * params.radius, 0.0, s * params.radius)
                }
                GridOrientation::XY => {
                    params.center + Vec3::new(c_ * params.radius, s * params.radius, 0.0)
                }
                GridOrientation::YZ => {
                    params.center + Vec3::new(0.0, c_ * params.radius, s * params.radius)
                }
            };
            handles.push(geo.add_point(pos));
        }

        handles.push(handles[0]);
        geo.add_polyline(&handles);

        Ok(geo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate;
    use approx::assert_relative_eq;
    use procgeo_core::{PolyType, PrimHandle, Primitive};

    #[test]
    fn circle_default() {
        let sop = CircleSop;
        let params = CircleParams::default();
        let geo = generate(&sop, &params).unwrap();

        assert_eq!(geo.num_points(), 40);
        assert_eq!(geo.num_prims(), 1);
        assert_eq!(geo.num_vertices(), 41);
    }

    #[test]
    fn circle_is_open_loop() {
        let geo = generate(&CircleSop, &CircleParams::default()).unwrap();

        match geo.prim(PrimHandle::from_index(0)) {
            Primitive::Polygon(poly) => assert_eq!(poly.poly_type, PolyType::Open),
        }

        let pts = geo.prim_points(PrimHandle::from_index(0));
        assert_eq!(pts.first(), pts.last());
    }

    #[test]
    fn circle_radius() {
        let sop = CircleSop;
        let params = CircleParams {
            radius: 3.0,
            center: Vec3::ZERO,
            divisions: 16,
            orientation: GridOrientation::XZ,
        };
        let geo = generate(&sop, &params).unwrap();

        for pt in geo.points() {
            let dist = (pt.x * pt.x + pt.z * pt.z).sqrt();
            assert_relative_eq!(dist, 3.0, epsilon = 1e-5);
        }
    }
}
