use std::f32::consts::{PI, TAU};

use glam::Vec3;
use serde::{Deserialize, Serialize};

use procgeo_core::Geometry;

use crate::{Sop, SopError};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SphereParams {
    pub radius: Vec3,
    pub center: Vec3,
    pub rows: u32,
    pub cols: u32,
}

impl Default for SphereParams {
    fn default() -> Self {
        SphereParams {
            radius: Vec3::splat(0.5),
            center: Vec3::ZERO,
            rows: 12,
            cols: 24,
        }
    }
}

pub struct SphereSop;

impl Sop for SphereSop {
    type Params = SphereParams;

    fn name(&self) -> &'static str {
        "sphere"
    }

    fn input_count(&self) -> (usize, usize) {
        (0, 0)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;

        if params.rows < 2 {
            return Err(SopError::InvalidParam(format!(
                "rows must be >= 2, got {}",
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
        let r = params.radius;
        let c = params.center;

        // num_points = 2 + (rows-1)*cols  (top pole + rings + bottom pole)
        // num_prims  = cols (top caps) + (rows-2)*cols (middle quads) + cols (bottom caps)
        let num_points = 2 + (rows - 1) * cols;
        let num_prims = cols + (rows - 2) * cols + cols;
        let mut geo = Geometry::with_capacity(num_points, num_prims);

        // Top pole
        let top_pole = geo.add_point(c + Vec3::new(0.0, r.y, 0.0));

        // Rings: ring i (0-indexed) corresponds to latitude angle
        // We have rows-1 rings (not counting the poles)
        // ring 0 is just below top pole, ring rows-2 is just above bottom pole
        let mut rings: Vec<Vec<_>> = Vec::with_capacity(rows - 1);
        for ring_idx in 0..(rows - 1) {
            // lat goes from PI/(rows) to PI*(rows-1)/rows
            let lat = PI * (ring_idx + 1) as f32 / rows as f32;
            let sin_lat = lat.sin();
            let cos_lat = lat.cos();

            let mut ring_handles = Vec::with_capacity(cols);
            for col_idx in 0..cols {
                let lon = TAU * col_idx as f32 / cols as f32;
                let (sin_lon, cos_lon) = lon.sin_cos();
                let pos = c + Vec3::new(
                    r.x * sin_lat * cos_lon,
                    r.y * cos_lat,
                    r.z * sin_lat * sin_lon,
                );
                ring_handles.push(geo.add_point(pos));
            }
            rings.push(ring_handles);
        }

        // Bottom pole
        let bot_pole = geo.add_point(c + Vec3::new(0.0, -r.y, 0.0));

        // Top cap triangles (top pole → first ring)
        let first_ring = &rings[0];
        for col_idx in 0..cols {
            let next = (col_idx + 1) % cols;
            geo.add_face(&[top_pole, first_ring[next], first_ring[col_idx]]);
        }

        // Middle quad strips
        for ring_idx in 0..(rows - 2) {
            let cur_ring = &rings[ring_idx];
            let next_ring = &rings[ring_idx + 1];
            for col_idx in 0..cols {
                let next_col = (col_idx + 1) % cols;
                geo.add_face(&[
                    cur_ring[col_idx],
                    cur_ring[next_col],
                    next_ring[next_col],
                    next_ring[col_idx],
                ]);
            }
        }

        // Bottom cap triangles (last ring → bottom pole)
        let last_ring = &rings[rows - 2];
        for col_idx in 0..cols {
            let next = (col_idx + 1) % cols;
            geo.add_face(&[last_ring[col_idx], last_ring[next], bot_pole]);
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
    fn sphere_default() {
        let sop = SphereSop;
        let params = SphereParams::default(); // rows=12, cols=24
        let geo = generate(&sop, &params).unwrap();

        // num_points = 2 + (12-1)*24 = 2 + 264 = 266
        assert_eq!(geo.num_points(), 266);
        // num_prims = 24 + (12-2)*24 + 24 = 24 + 240 + 24 = 288
        assert_eq!(geo.num_prims(), 288);
    }

    #[test]
    fn sphere_bounding_box() {
        let sop = SphereSop;
        let params = SphereParams::default(); // radius splat(0.5), center ZERO
        let geo = generate(&sop, &params).unwrap();

        let bb = geo.bounding_box();
        assert_relative_eq!(bb.min.y, -0.5, epsilon = 1e-5);
        assert_relative_eq!(bb.max.y,  0.5, epsilon = 1e-5);
        // X and Z should be within [-0.5, 0.5] (may be slightly less due to discrete sampling)
        assert!(bb.min.x >= -0.5 - 1e-5);
        assert!(bb.max.x <=  0.5 + 1e-5);
        assert!(bb.min.z >= -0.5 - 1e-5);
        assert!(bb.max.z <=  0.5 + 1e-5);
    }

    #[test]
    fn sphere_minimal() {
        let sop = SphereSop;
        let params = SphereParams {
            rows: 2,
            cols: 3,
            ..Default::default()
        };
        let geo = generate(&sop, &params).unwrap();

        // num_points = 2 + (2-1)*3 = 2 + 3 = 5
        assert_eq!(geo.num_points(), 5);
        // num_prims = 3 (top) + (2-2)*3 (middle=0) + 3 (bottom) = 6
        assert_eq!(geo.num_prims(), 6);
    }
}
