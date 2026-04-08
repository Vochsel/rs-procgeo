use std::f32::consts::TAU;

use glam::Vec3;
use serde::{Deserialize, Serialize};

use procgeo_core::Geometry;

use crate::{Sop, SopError};

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub enum TubeCap {
    #[default]
    None,
    Top,
    Bottom,
    Both,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TubeParams {
    pub radius_bottom: f32,
    pub radius_top: f32,
    pub height: f32,
    pub center: Vec3,
    pub cols: u32,
    pub rows: u32,
    pub caps: TubeCap,
}

impl Default for TubeParams {
    fn default() -> Self {
        TubeParams {
            radius_bottom: 0.5,
            radius_top: 0.5,
            height: 1.0,
            center: Vec3::ZERO,
            cols: 24,
            rows: 2,
            caps: TubeCap::None,
        }
    }
}

pub struct TubeSop;

impl Sop for TubeSop {
    type Params = TubeParams;

    fn name(&self) -> &'static str {
        "tube"
    }

    fn input_count(&self) -> (usize, usize) {
        (0, 0)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;

        if params.cols < 3 {
            return Err(SopError::InvalidParam(format!(
                "cols must be >= 3, got {}",
                params.cols
            )));
        }
        if params.rows < 2 {
            return Err(SopError::InvalidParam(format!(
                "rows must be >= 2, got {}",
                params.rows
            )));
        }

        let rows = params.rows as usize;
        let cols = params.cols as usize;
        let c = params.center;
        let half_h = params.height * 0.5;

        let mut geo = Geometry::new();

        // Generate ring points for each row
        let mut ring_handles: Vec<Vec<_>> = Vec::with_capacity(rows);
        for row_idx in 0..rows {
            let t = row_idx as f32 / (rows - 1) as f32; // 0..1
            let radius = params.radius_bottom + (params.radius_top - params.radius_bottom) * t;
            let y = -half_h + t * params.height;

            let mut ring = Vec::with_capacity(cols);
            for col_idx in 0..cols {
                let angle = TAU * col_idx as f32 / cols as f32;
                let (sin_a, cos_a) = angle.sin_cos();
                let pos = c + Vec3::new(cos_a * radius, y, sin_a * radius);
                ring.push(geo.add_point(pos));
            }
            ring_handles.push(ring);
        }

        // Generate quads between adjacent rings
        for row_idx in 0..(rows - 1) {
            let cur = &ring_handles[row_idx];
            let next = &ring_handles[row_idx + 1];
            for col_idx in 0..cols {
                let next_col = (col_idx + 1) % cols;
                geo.add_face(&[
                    cur[col_idx],
                    next[col_idx],
                    next[next_col],
                    cur[next_col],
                ]);
            }
        }

        // Bottom cap (row 0)
        if params.caps == TubeCap::Bottom || params.caps == TubeCap::Both {
            let bottom_y = -half_h;
            let bot_center = geo.add_point(c + Vec3::new(0.0, bottom_y, 0.0));
            let bottom_ring = &ring_handles[0];
            for col_idx in 0..cols {
                let next_col = (col_idx + 1) % cols;
                // Winding: outward normal points down (-Y)
                geo.add_face(&[bot_center, bottom_ring[col_idx], bottom_ring[next_col]]);
            }
        }

        // Top cap (last row)
        if params.caps == TubeCap::Top || params.caps == TubeCap::Both {
            let top_y = half_h;
            let top_center = geo.add_point(c + Vec3::new(0.0, top_y, 0.0));
            let top_ring = &ring_handles[rows - 1];
            for col_idx in 0..cols {
                let next_col = (col_idx + 1) % cols;
                // Winding: outward normal points up (+Y)
                geo.add_face(&[top_center, top_ring[next_col], top_ring[col_idx]]);
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
    fn tube_default() {
        let sop = TubeSop;
        let params = TubeParams::default(); // cols=24, rows=2, no caps
        let geo = generate(&sop, &params).unwrap();

        // 2 rows * 24 cols = 48 points
        assert_eq!(geo.num_points(), 48);
        // (rows-1) * cols = 1 * 24 = 24 prims
        assert_eq!(geo.num_prims(), 24);
    }

    #[test]
    fn tube_with_caps() {
        let sop = TubeSop;
        let params = TubeParams {
            cols: 8,
            rows: 2,
            caps: TubeCap::Both,
            ..Default::default()
        };
        let geo = generate(&sop, &params).unwrap();

        // ring points: 2*8 = 16, + 2 cap centers = 18
        assert_eq!(geo.num_points(), 18);
        // side quads: (2-1)*8 = 8
        // bottom cap tris: 8
        // top cap tris: 8
        // total: 8 + 8 + 8 = 24
        assert_eq!(geo.num_prims(), 24);
    }

    #[test]
    fn tube_cone() {
        let sop = TubeSop;
        let params = TubeParams {
            radius_bottom: 1.0,
            radius_top: 0.0,
            height: 2.0,
            cols: 8,
            rows: 2,
            ..Default::default()
        };
        let geo = generate(&sop, &params).unwrap();

        // Top row should be at radius 0 — all x,z should be 0
        let points: Vec<_> = geo.points().collect();
        // Top 8 points (last ring, row index 1)
        for i in 8..16 {
            assert_relative_eq!(points[i].x, 0.0, epsilon = 1e-5);
            assert_relative_eq!(points[i].z, 0.0, epsilon = 1e-5);
        }
    }
}
