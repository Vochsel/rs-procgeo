use serde::{Deserialize, Serialize};

use glam::Vec3;

use procgeo_core::{Geometry, PointHandle, PrimHandle};

use crate::{Sop, SopError};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PolyExtrudeParams {
    /// Distance to extrude along the face normal.
    pub distance: f32,
    /// Amount to inset the extruded face toward its centroid (0 = no inset).
    pub inset: f32,
    /// Whether to output the front (extruded/top) face.
    pub output_front: bool,
    /// Whether to output side quads connecting original to extruded edges.
    pub output_side: bool,
}

impl Default for PolyExtrudeParams {
    fn default() -> Self {
        PolyExtrudeParams {
            distance: 1.0,
            inset: 0.0,
            output_front: true,
            output_side: true,
        }
    }
}

/// Compute a face normal via Newell's method.
fn face_normal(positions: &[Vec3]) -> Vec3 {
    let n = positions.len();
    let mut nx = 0.0_f32;
    let mut ny = 0.0_f32;
    let mut nz = 0.0_f32;
    for i in 0..n {
        let cur = positions[i];
        let next = positions[(i + 1) % n];
        nx += (cur.y - next.y) * (cur.z + next.z);
        ny += (cur.z - next.z) * (cur.x + next.x);
        nz += (cur.x - next.x) * (cur.y + next.y);
    }
    Vec3::new(nx, ny, nz).normalize_or_zero()
}

pub struct PolyExtrudeSop;

impl Sop for PolyExtrudeSop {
    type Params = PolyExtrudeParams;

    fn name(&self) -> &'static str {
        "poly_extrude"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 1)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;
        let geo = inputs[0];

        let mut out = Geometry::new();

        // Copy all original points
        let orig_pt_count = geo.num_points();
        let mut orig_handles: Vec<PointHandle> = Vec::with_capacity(orig_pt_count);
        for i in 0..orig_pt_count {
            let ph = PointHandle::from_index(i);
            orig_handles.push(out.add_point(geo.point_pos(ph)));
        }

        // For each face, extrude
        for prim_idx in 0..geo.num_prims() {
            let prim_handle = PrimHandle::from_index(prim_idx);
            let prim = geo.prim(prim_handle);

            // Only extrude closed polygons
            let is_closed = match prim {
                procgeo_core::Primitive::Polygon(p) => {
                    p.poly_type == procgeo_core::PolyType::Closed
                }
            };

            if !is_closed {
                continue;
            }

            let pt_handles = geo.prim_points(prim_handle);
            let n = pt_handles.len();
            if n < 3 {
                continue;
            }

            let positions: Vec<Vec3> = pt_handles.iter().map(|&h| geo.point_pos(h)).collect();

            // Compute face normal
            let normal = face_normal(&positions);

            // Compute face centroid
            let centroid: Vec3 = positions.iter().sum::<Vec3>() / n as f32;

            // Create extruded points
            let mut extruded_handles: Vec<PointHandle> = Vec::with_capacity(n);
            for base_pos in &positions {
                // Offset by distance along normal
                let mut ext_pos = *base_pos + normal * params.distance;
                // Apply inset: move toward centroid
                if params.inset > 0.0 {
                    let centroid_ext = centroid + normal * params.distance;
                    ext_pos = ext_pos + (centroid_ext - ext_pos) * params.inset;
                }
                extruded_handles.push(out.add_point(ext_pos));
            }

            // Create side quads: for each edge (i, i+1)
            if params.output_side {
                for i in 0..n {
                    let next = (i + 1) % n;
                    let orig_i = orig_handles[pt_handles[i].index()];
                    let orig_next = orig_handles[pt_handles[next].index()];
                    let ext_i = extruded_handles[i];
                    let ext_next = extruded_handles[next];
                    // Side quad: orig_i, orig_next, ext_next, ext_i
                    out.add_face(&[orig_i, orig_next, ext_next, ext_i]);
                }
            }

            // Create front (top) face using extruded points
            if params.output_front {
                out.add_face(&extruded_handles);
            }
        }

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creation::box_sop::{BoxSop, BoxParams};
    use crate::{GeometryExt, generate};
    use approx::assert_relative_eq;
    use glam::Vec3;

    fn make_box() -> Geometry {
        generate(&BoxSop, &BoxParams::default()).unwrap()
    }

    fn make_single_quad() -> Geometry {
        let mut geo = Geometry::new();
        let p0 = geo.add_point(Vec3::new(-0.5, 0.0, -0.5));
        let p1 = geo.add_point(Vec3::new( 0.5, 0.0, -0.5));
        let p2 = geo.add_point(Vec3::new( 0.5, 0.0,  0.5));
        let p3 = geo.add_point(Vec3::new(-0.5, 0.0,  0.5));
        // CCW winding from +Y → outward normal points +Y
        geo.add_face(&[p0, p3, p2, p1]);
        geo
    }

    #[test]
    fn extrude_single_quad() {
        // 1 quad face, 4 orig pts → side quads (4) + front face (1) = 5 prims, 8 points
        let params = PolyExtrudeParams::default(); // distance=1, output_front=true, output_side=true
        let result = make_single_quad().apply(&PolyExtrudeSop, &params).unwrap();

        assert_eq!(result.num_prims(), 5, "expected 4 sides + 1 front = 5 prims");
        assert_eq!(result.num_points(), 8, "expected 4 orig + 4 extruded = 8 points");
    }

    #[test]
    fn extrude_distance() {
        // Single quad at y=0, extruded by distance=2.0 along its normal.
        // The normal direction depends on winding; verify extruded points are
        // exactly 2.0 units from the original quad (in absolute terms).
        let params = PolyExtrudeParams {
            distance: 2.0,
            ..Default::default()
        };
        let result = make_single_quad().apply(&PolyExtrudeSop, &params).unwrap();

        // Original 4 points are at y=0; extruded 4 points should be at y=+2.0
        // (quad winding produces +Y outward normal)
        for i in 4..8 {
            let ph = PointHandle::from_index(i);
            let pos = result.point_pos(ph);
            assert_relative_eq!(pos.y, 2.0, epsilon = 1e-4);
        }
    }

    #[test]
    fn extrude_box() {
        // Box has 6 faces. Each face: 4 side quads + 1 front = 5 prims.
        // Total: 6*5 = 30 prims.
        // Points: 8 orig + 6*4 extruded = 32 points (some may share, but we add separately per face).
        let params = PolyExtrudeParams::default();
        let result = make_box().apply(&PolyExtrudeSop, &params).unwrap();

        assert_eq!(result.num_prims(), 30, "expected 6*5=30 prims for extruded box");
        assert_eq!(result.num_points(), 8 + 6 * 4, "expected 8 orig + 24 extruded = 32 points");
    }

    #[test]
    fn extrude_no_front() {
        // Without front face, only 4 sides
        let params = PolyExtrudeParams {
            output_front: false,
            ..Default::default()
        };
        let result = make_single_quad().apply(&PolyExtrudeSop, &params).unwrap();
        assert_eq!(result.num_prims(), 4, "expected only 4 side quads");
    }

    #[test]
    fn extrude_no_sides() {
        // Without sides, only 1 front face
        let params = PolyExtrudeParams {
            output_side: false,
            ..Default::default()
        };
        let result = make_single_quad().apply(&PolyExtrudeSop, &params).unwrap();
        assert_eq!(result.num_prims(), 1, "expected only 1 front face");
    }
}
