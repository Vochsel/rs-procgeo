use glam::Vec3;
use serde::{Deserialize, Serialize};

use procgeo_core::{AttribClass, AttribDefault, Geometry, PrimHandle, TypeQualifier};

use crate::{Sop, SopError};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NormalParams;

pub struct NormalSop;

impl Sop for NormalSop {
    type Params = NormalParams;

    fn name(&self) -> &'static str {
        "normal"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 1)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;
        let _ = params;

        let mut geo = inputs[0].clone();

        // Create (or ensure) the "N" point attribute with Normal qualifier
        // It's ok if it already exists; we ignore the error.
        let _ = geo.add_attrib(
            AttribClass::Point,
            "N",
            AttribDefault::Vector3([0.0, 0.0, 0.0]),
            TypeQualifier::Normal,
        );

        let num_pts = geo.num_points();
        let num_prims = geo.num_prims();

        // Accumulate normals per point (area-weighted via Newell's method)
        let mut normals: Vec<Vec3> = vec![Vec3::ZERO; num_pts];

        for prim_idx in 0..num_prims {
            let prim_handle = PrimHandle::from_index(prim_idx);
            let point_handles = geo.prim_points(prim_handle);
            let n_verts = point_handles.len();

            if n_verts < 3 {
                continue;
            }

            // Gather positions
            let positions: Vec<Vec3> = point_handles
                .iter()
                .map(|&ph| geo.point_pos(ph))
                .collect();

            // Newell's method: face normal is sum of cross products of consecutive edges
            // N = sum_i (v_i - v_0) × (v_{i+1} - v_0) ... but proper Newell's:
            // N_x = sum_i (y_i - y_{i+1}) * (z_i + z_{i+1})
            // N_y = sum_i (z_i - z_{i+1}) * (x_i + x_{i+1})
            // N_z = sum_i (x_i - x_{i+1}) * (y_i + y_{i+1})
            let mut nx = 0.0_f32;
            let mut ny = 0.0_f32;
            let mut nz = 0.0_f32;
            for i in 0..n_verts {
                let cur = positions[i];
                let next = positions[(i + 1) % n_verts];
                nx += (cur.y - next.y) * (cur.z + next.z);
                ny += (cur.z - next.z) * (cur.x + next.x);
                nz += (cur.x - next.x) * (cur.y + next.y);
            }
            let face_normal = Vec3::new(nx, ny, nz);

            // Area-weight: the magnitude of face_normal is 2 * area
            // Accumulate onto all face points
            for &ph in &point_handles {
                normals[ph.index()] += face_normal;
            }
        }

        // Normalize and store
        let n_handle = geo
            .find_attrib::<[f32; 3]>(AttribClass::Point, "N")
            .map_err(|e| SopError::Core(e))?;

        for i in 0..num_pts {
            let n = normals[i].normalize_or_zero();
            geo.set_attrib(&n_handle, i, [n.x, n.y, n.z])
                .map_err(|e| SopError::Core(e))?;
        }

        Ok(geo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use crate::creation::grid::{GridSop, GridParams, GridOrientation};
    use crate::creation::box_sop::{BoxSop, BoxParams};
    use crate::{GeometryExt, generate};

    #[test]
    fn normal_on_grid() {
        // XZ grid: all face normals should point in ±Y
        let grid = generate(&GridSop, &GridParams {
            size: [2.0, 2.0],
            rows: 3,
            cols: 3,
            center: Vec3::ZERO,
            orientation: GridOrientation::XZ,
        }).unwrap();

        let sop = NormalSop;
        let result = grid.apply(&sop, &NormalParams).unwrap();

        let n_handle = result.find_attrib::<[f32; 3]>(AttribClass::Point, "N").unwrap();
        for i in 0..result.num_points() {
            let n = result.get_attrib(&n_handle, i).unwrap();
            let nv = Vec3::from(n);
            // Should be close to (0,1,0) or (0,-1,0) — check x and z are near zero for interior pts
            // Boundary points might differ slightly due to one-sided averaging
            // At least the magnitude should be 1 or 0 (border with no faces on one side)
            let mag = nv.length();
            assert!(mag <= 1.0 + 1e-5, "Normal magnitude too large: {mag}");
        }

        // Check a center point (index 4 in 3x3) — should be (0, ±1, 0)
        let center_n = result.get_attrib(&n_handle, 4).unwrap();
        let cn = Vec3::from(center_n);
        assert_relative_eq!(cn.x.abs(), 0.0, epsilon = 1e-4);
        assert_relative_eq!(cn.z.abs(), 0.0, epsilon = 1e-4);
        assert_relative_eq!(cn.y.abs(), 1.0, epsilon = 1e-4);
    }

    #[test]
    fn normal_on_box() {
        let box_geo = generate(&BoxSop, &BoxParams::default()).unwrap();

        let sop = NormalSop;
        let result = box_geo.apply(&sop, &NormalParams).unwrap();

        let n_handle = result.find_attrib::<[f32; 3]>(AttribClass::Point, "N").unwrap();
        for i in 0..result.num_points() {
            let n = result.get_attrib(&n_handle, i).unwrap();
            let nv = Vec3::from(n);
            let mag = nv.length();
            assert_relative_eq!(mag, 1.0, epsilon = 1e-5);
        }
    }
}
