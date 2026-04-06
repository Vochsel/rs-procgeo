use std::f32::consts::TAU;

use glam::Vec3;
use serde::{Deserialize, Serialize};

use procgeo_core::Geometry;

use crate::{Sop, SopError};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TorusParams {
    pub radius_outer: f32,
    pub radius_inner: f32,
    pub center: Vec3,
    pub rows: u32,
    pub cols: u32,
}

impl Default for TorusParams {
    fn default() -> Self {
        TorusParams {
            radius_outer: 1.0,
            radius_inner: 0.3,
            center: Vec3::ZERO,
            rows: 12,
            cols: 24,
        }
    }
}

pub struct TorusSop;

impl Sop for TorusSop {
    type Params = TorusParams;

    fn name(&self) -> &'static str {
        "torus"
    }

    fn input_count(&self) -> (usize, usize) {
        (0, 0)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;

        if params.rows < 3 {
            return Err(SopError::InvalidParam(format!(
                "rows must be >= 3, got {}",
                params.rows
            )));
        }
        if params.cols < 3 {
            return Err(SopError::InvalidParam(format!(
                "cols must be >= 3, got {}",
                params.cols
            )));
        }

        let rows = params.rows as usize;
        let cols = params.cols as usize;
        let ro = params.radius_outer;
        let ri = params.radius_inner;
        let c = params.center;

        let num_points = rows * cols;
        let mut geo = Geometry::with_capacity(num_points, num_points);

        // Generate points: rows = tubes around the ring, cols = ring cross-section
        let mut handles = vec![vec![]; rows];
        for row_idx in 0..rows {
            let theta = TAU * row_idx as f32 / rows as f32; // major angle
            let (sin_t, cos_t) = theta.sin_cos();
            let center_x = ro * cos_t;
            let center_z = ro * sin_t;

            for col_idx in 0..cols {
                let phi = TAU * col_idx as f32 / cols as f32; // minor angle
                let (sin_p, cos_p) = phi.sin_cos();

                let pos = c + Vec3::new(
                    (ro + ri * cos_p) * cos_t,
                    ri * sin_p,
                    (ro + ri * cos_p) * sin_t,
                );
                let _ = center_x; // suppress unused warning
                let _ = center_z;
                handles[row_idx].push(geo.add_point(pos));
            }
        }

        // Generate quads wrapping in both directions
        for row_idx in 0..rows {
            let next_row = (row_idx + 1) % rows;
            for col_idx in 0..cols {
                let next_col = (col_idx + 1) % cols;
                geo.add_face(&[
                    handles[row_idx][col_idx],
                    handles[next_row][col_idx],
                    handles[next_row][next_col],
                    handles[row_idx][next_col],
                ]);
            }
        }

        Ok(geo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use crate::generate;

    #[test]
    fn torus_default() {
        let sop = TorusSop;
        let params = TorusParams::default(); // rows=12, cols=24
        let geo = generate(&sop, &params).unwrap();

        // num_points = 12*24 = 288
        assert_eq!(geo.num_points(), 288);
        // num_prims = 12*24 = 288
        assert_eq!(geo.num_prims(), 288);
    }

    #[test]
    fn torus_minimal() {
        let sop = TorusSop;
        let params = TorusParams {
            rows: 3,
            cols: 3,
            ..Default::default()
        };
        let geo = generate(&sop, &params).unwrap();

        // num_points = 3*3 = 9
        assert_eq!(geo.num_points(), 9);
        // num_prims = 3*3 = 9
        assert_eq!(geo.num_prims(), 9);
    }

    #[test]
    fn torus_symmetry() {
        let sop = TorusSop;
        let params = TorusParams {
            center: Vec3::ZERO,
            ..Default::default()
        };
        let geo = generate(&sop, &params).unwrap();

        let bb = geo.bounding_box();
        // Should be roughly symmetric about origin
        let cx = (bb.min.x + bb.max.x) * 0.5;
        let cz = (bb.min.z + bb.max.z) * 0.5;
        let cy = (bb.min.y + bb.max.y) * 0.5;
        assert_relative_eq!(cx, 0.0, epsilon = 1e-4);
        assert_relative_eq!(cz, 0.0, epsilon = 1e-4);
        assert_relative_eq!(cy, 0.0, epsilon = 1e-4);
    }
}
