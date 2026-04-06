use glam::Vec3;
use serde::{Deserialize, Serialize};

use procgeo_core::Geometry;

use crate::{Sop, SopError};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LineParams {
    pub origin: Vec3,
    pub direction: Vec3,
    pub length: f32,
    pub points: u32,
}

impl Default for LineParams {
    fn default() -> Self {
        LineParams {
            origin: Vec3::ZERO,
            direction: Vec3::Y,
            length: 1.0,
            points: 2,
        }
    }
}

pub struct LineSop;

impl Sop for LineSop {
    type Params = LineParams;

    fn name(&self) -> &'static str {
        "line"
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

        let n = params.points as usize;
        let dir = params.direction.normalize_or_zero();
        let mut geo = Geometry::with_capacity(n, 1);

        let mut handles = Vec::with_capacity(n);
        for i in 0..n {
            let t = if n == 1 {
                0.0
            } else {
                i as f32 / (n - 1) as f32
            };
            let pos = params.origin + dir * (t * params.length);
            handles.push(geo.add_point(pos));
        }

        geo.add_polyline(&handles);

        Ok(geo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use crate::generate;

    #[test]
    fn line_default() {
        let sop = LineSop;
        let params = LineParams::default();
        let geo = generate(&sop, &params).unwrap();

        assert_eq!(geo.num_points(), 2);
        assert_eq!(geo.num_prims(), 1);

        let points: Vec<_> = geo.points().collect();
        // Start at origin
        assert_relative_eq!(points[0].x, 0.0, epsilon = 1e-5);
        assert_relative_eq!(points[0].y, 0.0, epsilon = 1e-5);
        assert_relative_eq!(points[0].z, 0.0, epsilon = 1e-5);
        // End at origin + Y * length
        assert_relative_eq!(points[1].x, 0.0, epsilon = 1e-5);
        assert_relative_eq!(points[1].y, 1.0, epsilon = 1e-5);
        assert_relative_eq!(points[1].z, 0.0, epsilon = 1e-5);
    }

    #[test]
    fn line_multiple_points() {
        let sop = LineSop;
        let params = LineParams {
            origin: Vec3::ZERO,
            direction: Vec3::X,
            length: 4.0,
            points: 5,
        };
        let geo = generate(&sop, &params).unwrap();

        assert_eq!(geo.num_points(), 5);

        let points: Vec<_> = geo.points().collect();
        // Midpoint (index 2) should be at x=2.0
        assert_relative_eq!(points[2].x, 2.0, epsilon = 1e-5);
        assert_relative_eq!(points[2].y, 0.0, epsilon = 1e-5);
    }
}
