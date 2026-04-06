use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};

use glam::Vec3;

use procgeo_core::{AttribClass, AttribDefault, Geometry, PointHandle, PrimHandle, TypeQualifier};

use crate::{Sop, SopError};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScatterParams {
    /// Number of points to scatter.
    pub count: u32,
    /// Random seed for deterministic results.
    pub seed: u64,
}

impl Default for ScatterParams {
    fn default() -> Self {
        ScatterParams { count: 100, seed: 0 }
    }
}

pub struct ScatterSop;

/// Compute the area of a closed polygon via triangle fan from first vertex.
fn poly_area(positions: &[Vec3]) -> f32 {
    let n = positions.len();
    if n < 3 {
        return 0.0;
    }
    let v0 = positions[0];
    let mut area = 0.0_f32;
    for i in 1..n - 1 {
        let v1 = positions[i];
        let v2 = positions[i + 1];
        let cross = (v1 - v0).cross(v2 - v0);
        area += cross.length() * 0.5;
    }
    area
}

impl Sop for ScatterSop {
    type Params = ScatterParams;

    fn name(&self) -> &'static str {
        "scatter"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 1)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;
        let geo = inputs[0];

        let num_prims = geo.num_prims();
        if num_prims == 0 || params.count == 0 {
            return Ok(Geometry::new());
        }

        // Compute face areas
        let mut areas: Vec<f32> = Vec::with_capacity(num_prims);
        for prim_idx in 0..num_prims {
            let ph = PrimHandle::from_index(prim_idx);
            let pt_handles = geo.prim_points(ph);
            let positions: Vec<Vec3> = pt_handles.iter().map(|&h| geo.point_pos(h)).collect();
            areas.push(poly_area(&positions));
        }

        let total_area: f32 = areas.iter().sum();
        if total_area <= 0.0 {
            return Ok(Geometry::new());
        }

        // Build CDF (cumulative distribution function)
        let mut cdf: Vec<f32> = Vec::with_capacity(num_prims);
        let mut running = 0.0_f32;
        for &a in &areas {
            running += a / total_area;
            cdf.push(running);
        }
        // Ensure last entry is exactly 1.0
        if let Some(last) = cdf.last_mut() {
            *last = 1.0;
        }

        let mut out = Geometry::new();
        out.add_attrib(
            AttribClass::Point,
            "sourceprim",
            AttribDefault::Int(0),
            TypeQualifier::None,
        )?;
        let sourceprim_handle = out.find_attrib::<i32>(AttribClass::Point, "sourceprim")?;

        let mut rng = StdRng::seed_from_u64(params.seed);

        for _ in 0..params.count {
            // Pick a face via binary search on CDF
            let r: f32 = rng.random_range(0.0f32..1.0f32);
            let prim_idx = cdf.partition_point(|&c| c < r).min(num_prims - 1);

            let ph = PrimHandle::from_index(prim_idx);
            let pt_handles = geo.prim_points(ph);
            let positions: Vec<Vec3> = pt_handles.iter().map(|&h| geo.point_pos(h)).collect();
            let n = positions.len();

            // Pick a random triangle in the fan from positions[0]
            // Build weights proportional to triangle area
            let num_tris = n - 2;
            let mut tri_areas: Vec<f32> = Vec::with_capacity(num_tris);
            for i in 0..num_tris {
                let a = positions[0];
                let b = positions[i + 1];
                let c = positions[i + 2];
                let area = (b - a).cross(c - a).length() * 0.5;
                tri_areas.push(area);
            }
            let total_tri_area: f32 = tri_areas.iter().sum();

            let tri_idx = if total_tri_area > 0.0 {
                let r2: f32 = rng.random_range(0.0f32..1.0f32);
                let mut running = 0.0_f32;
                let mut chosen = 0;
                for (i, &ta) in tri_areas.iter().enumerate() {
                    running += ta / total_tri_area;
                    if r2 <= running {
                        chosen = i;
                        break;
                    }
                    chosen = i;
                }
                chosen
            } else {
                0
            };

            // Random barycentric coords in the selected triangle
            let va = positions[0];
            let vb = positions[tri_idx + 1];
            let vc = positions[tri_idx + 2];

            let mut u: f32 = rng.random_range(0.0f32..1.0f32);
            let mut v: f32 = rng.random_range(0.0f32..1.0f32);
            if u + v > 1.0 {
                u = 1.0 - u;
                v = 1.0 - v;
            }
            let pos = va * (1.0 - u - v) + vb * u + vc * v;

            let new_pt = out.add_point(pos);
            out.set_attrib(&sourceprim_handle, new_pt.index(), prim_idx as i32)?;
        }

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creation::grid::{GridSop, GridParams};
    use crate::{GeometryExt, generate};
    use approx::assert_relative_eq;

    fn make_grid() -> Geometry {
        generate(&GridSop, &GridParams::default()).unwrap()
    }

    #[test]
    fn scatter_count() {
        let params = ScatterParams { count: 100, seed: 0 };
        let result = make_grid().apply(&ScatterSop, &params).unwrap();
        assert_eq!(result.num_points(), 100);
        assert_eq!(result.num_prims(), 0);
    }

    #[test]
    fn scatter_on_grid() {
        // All scattered points should lie within the grid bounding box
        let grid = make_grid();
        let bbox = grid.bounding_box();

        let params = ScatterParams { count: 200, seed: 42 };
        let result = grid.apply(&ScatterSop, &params).unwrap();
        assert_eq!(result.num_points(), 200);

        for pt_pos in result.points() {
            assert!(
                pt_pos.x >= bbox.min.x - 1e-4 && pt_pos.x <= bbox.max.x + 1e-4,
                "point x={} outside grid bbox x [{}, {}]",
                pt_pos.x, bbox.min.x, bbox.max.x
            );
            assert!(
                pt_pos.z >= bbox.min.z - 1e-4 && pt_pos.z <= bbox.max.z + 1e-4,
                "point z={} outside grid bbox z [{}, {}]",
                pt_pos.z, bbox.min.z, bbox.max.z
            );
        }
    }

    #[test]
    fn scatter_deterministic() {
        let grid = make_grid();
        let params = ScatterParams { count: 50, seed: 123 };
        let r1 = grid.clone().apply(&ScatterSop, &params).unwrap();
        let r2 = grid.apply(&ScatterSop, &params).unwrap();

        assert_eq!(r1.num_points(), r2.num_points());
        for i in 0..r1.num_points() {
            let ph = PointHandle::from_index(i);
            let p1 = r1.point_pos(ph);
            let p2 = r2.point_pos(ph);
            assert_relative_eq!(p1.x, p2.x, epsilon = 1e-6);
            assert_relative_eq!(p1.y, p2.y, epsilon = 1e-6);
            assert_relative_eq!(p1.z, p2.z, epsilon = 1e-6);
        }
    }
}
