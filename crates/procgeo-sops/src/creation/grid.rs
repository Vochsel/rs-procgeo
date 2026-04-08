use glam::Vec3;
use serde::{Deserialize, Serialize};

use procgeo_core::Geometry;

use crate::{Sop, SopError};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub enum GridOrientation {
    #[default]
    XZ,
    XY,
    YZ,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GridParams {
    pub size: [f32; 2],
    pub rows: u32,
    pub cols: u32,
    pub center: Vec3,
    pub orientation: GridOrientation,
}

impl Default for GridParams {
    fn default() -> Self {
        GridParams {
            size: [10.0, 10.0],
            rows: 10,
            cols: 10,
            center: Vec3::ZERO,
            orientation: GridOrientation::XZ,
        }
    }
}

pub struct GridSop;

impl Sop for GridSop {
    type Params = GridParams;

    fn name(&self) -> &'static str {
        "grid"
    }

    fn input_count(&self) -> (usize, usize) {
        (0, 0)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;

        let rows = params.rows;
        let cols = params.cols;

        if rows < 2 {
            return Err(SopError::InvalidParam(format!(
                "rows must be >= 2, got {rows}"
            )));
        }
        if cols < 2 {
            return Err(SopError::InvalidParam(format!(
                "cols must be >= 2, got {cols}"
            )));
        }

        let num_points = (rows * cols) as usize;
        let num_prims = ((rows - 1) * (cols - 1)) as usize;
        let mut geo = Geometry::with_capacity(num_points, num_prims);

        let w = params.size[0];
        let h = params.size[1];
        let c = params.center;

        // Generate points
        let mut handles = Vec::with_capacity(num_points);
        for row in 0..rows {
            for col in 0..cols {
                let u = col as f32 / (cols - 1) as f32; // 0..1
                let v = row as f32 / (rows - 1) as f32; // 0..1

                // local coordinates: center at 0, range [-w/2..w/2] x [-h/2..h/2]
                let s = (u - 0.5) * w;
                let t = (v - 0.5) * h;

                let pos = match params.orientation {
                    GridOrientation::XZ => c + Vec3::new(s, 0.0, t),
                    GridOrientation::XY => c + Vec3::new(s, t, 0.0),
                    GridOrientation::YZ => c + Vec3::new(0.0, s, t),
                };
                handles.push(geo.add_point(pos));
            }
        }

        // Generate quads
        for row in 0..(rows - 1) {
            for col in 0..(cols - 1) {
                let idx = |r: u32, c_: u32| (r * cols + c_) as usize;
                let p0 = handles[idx(row,     col    )];
                let p1 = handles[idx(row,     col + 1)];
                let p2 = handles[idx(row + 1, col + 1)];
                let p3 = handles[idx(row + 1, col    )];
                geo.add_face(&[p0, p3, p2, p1]);
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
    fn grid_default() {
        let sop = GridSop;
        let params = GridParams::default(); // 10x10 rows/cols
        let geo = generate(&sop, &params).unwrap();

        assert_eq!(geo.num_points(), 100);  // 10*10
        assert_eq!(geo.num_prims(), 81);    // 9*9
        assert_eq!(geo.num_vertices(), 81 * 4);
    }

    #[test]
    fn grid_2x2() {
        let sop = GridSop;
        let params = GridParams { rows: 2, cols: 2, ..Default::default() };
        let geo = generate(&sop, &params).unwrap();

        assert_eq!(geo.num_points(), 4);
        assert_eq!(geo.num_prims(), 1);
    }

    #[test]
    fn grid_3x3() {
        let sop = GridSop;
        let params = GridParams { rows: 3, cols: 3, ..Default::default() };
        let geo = generate(&sop, &params).unwrap();

        assert_eq!(geo.num_points(), 9);
        assert_eq!(geo.num_prims(), 4);
    }

    #[test]
    fn grid_bounding_box() {
        let sop = GridSop;
        let params = GridParams {
            size: [4.0, 6.0],
            rows: 5,
            cols: 5,
            center: Vec3::ZERO,
            orientation: GridOrientation::XZ,
        };
        let geo = generate(&sop, &params).unwrap();

        let bb = geo.bounding_box();
        assert_relative_eq!(bb.min.x, -2.0, epsilon = 1e-5);
        assert_relative_eq!(bb.max.x,  2.0, epsilon = 1e-5);
        assert_relative_eq!(bb.min.z, -3.0, epsilon = 1e-5);
        assert_relative_eq!(bb.max.z,  3.0, epsilon = 1e-5);
        // Y should be 0 for XZ orientation
        assert_relative_eq!(bb.min.y, 0.0, epsilon = 1e-5);
        assert_relative_eq!(bb.max.y, 0.0, epsilon = 1e-5);
    }

    #[test]
    fn grid_xy_orientation() {
        let sop = GridSop;
        let params = GridParams {
            size: [2.0, 2.0],
            rows: 3,
            cols: 3,
            center: Vec3::ZERO,
            orientation: GridOrientation::XY,
        };
        let geo = generate(&sop, &params).unwrap();

        let bb = geo.bounding_box();
        // Z should be 0 for XY orientation (flat on Z)
        assert_relative_eq!(bb.min.z, 0.0, epsilon = 1e-5);
        assert_relative_eq!(bb.max.z, 0.0, epsilon = 1e-5);
    }

    #[test]
    fn grid_rejects_small() {
        let sop = GridSop;
        let params = GridParams { rows: 1, cols: 5, ..Default::default() };
        let result = generate(&sop, &params);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SopError::InvalidParam(_)));
    }
}
