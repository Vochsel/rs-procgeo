use std::f32::consts::TAU;

use serde::{Deserialize, Serialize};

use glam::Vec3;

use procgeo_core::{Geometry, PointHandle, PrimHandle};

use crate::{Sop, SopError};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PolyWireParams {
    /// Tube radius around the curve.
    pub radius: f32,
    /// Number of sides in the cross-section circle.
    pub divisions: u32,
}

impl Default for PolyWireParams {
    fn default() -> Self {
        PolyWireParams {
            radius: 0.1,
            divisions: 8,
        }
    }
}

pub struct PolyWireSop;

/// Compute a stable local frame at a curve point given a tangent direction.
/// Returns (normal, bitangent) perpendicular to `tangent`.
fn local_frame(tangent: Vec3) -> (Vec3, Vec3) {
    let t = tangent.normalize_or_zero();
    // Choose a reference up vector that is not parallel to the tangent
    let up = if t.dot(Vec3::Y).abs() < 0.999 {
        Vec3::Y
    } else {
        Vec3::X
    };
    let normal = t.cross(up).normalize_or_zero();
    let bitangent = t.cross(normal).normalize_or_zero();
    (normal, bitangent)
}

impl Sop for PolyWireSop {
    type Params = PolyWireParams;

    fn name(&self) -> &'static str {
        "poly_wire"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 1)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;
        let geo = inputs[0];

        if params.divisions < 3 {
            return Err(SopError::InvalidParam(format!(
                "divisions must be >= 3, got {}",
                params.divisions
            )));
        }

        let divs = params.divisions as usize;
        let mut out = Geometry::new();

        for prim_idx in 0..geo.num_prims() {
            let prim_handle = PrimHandle::from_index(prim_idx);
            let prim = geo.prim(prim_handle);

            // Only process open polylines
            let is_open = match prim {
                procgeo_core::Primitive::Polygon(p) => p.poly_type == procgeo_core::PolyType::Open,
            };

            if !is_open {
                // Pass-through closed polygons unchanged
                let pt_handles = geo.prim_points(prim_handle);
                let new_pts: Vec<PointHandle> = pt_handles
                    .iter()
                    .map(|&h| out.add_point(geo.point_pos(h)))
                    .collect();
                out.add_face(&new_pts);
                continue;
            }

            let pt_handles = geo.prim_points(prim_handle);
            let num_curve_pts = pt_handles.len();
            if num_curve_pts < 2 {
                continue;
            }

            let positions: Vec<Vec3> = pt_handles.iter().map(|&h| geo.point_pos(h)).collect();

            // Generate a ring of points at each curve point
            let mut rings: Vec<Vec<PointHandle>> = Vec::with_capacity(num_curve_pts);

            for i in 0..num_curve_pts {
                // Compute tangent
                let tangent = if i == 0 {
                    positions[1] - positions[0]
                } else if i == num_curve_pts - 1 {
                    positions[num_curve_pts - 1] - positions[num_curve_pts - 2]
                } else {
                    positions[i + 1] - positions[i - 1]
                };

                let (normal, bitangent) = local_frame(tangent);
                let center = positions[i];

                let mut ring = Vec::with_capacity(divs);
                for j in 0..divs {
                    let angle = TAU * j as f32 / divs as f32;
                    let (sin_a, cos_a) = angle.sin_cos();
                    let pos =
                        center + normal * cos_a * params.radius + bitangent * sin_a * params.radius;
                    ring.push(out.add_point(pos));
                }
                rings.push(ring);
            }

            // Connect adjacent rings with quads (same pattern as TubeSop)
            for i in 0..(num_curve_pts - 1) {
                let cur = &rings[i];
                let next = &rings[i + 1];
                for j in 0..divs {
                    let next_j = (j + 1) % divs;
                    out.add_face(&[cur[j], cur[next_j], next[next_j], next[j]]);
                }
            }
        }

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creation::line::{LineParams, LineSop};
    use crate::{GeometryExt, generate};
    use approx::assert_relative_eq;
    use glam::Vec3;

    fn make_line(points: u32) -> Geometry {
        generate(
            &LineSop,
            &LineParams {
                origin: Vec3::ZERO,
                direction: Vec3::Y,
                length: 1.0,
                points,
            },
        )
        .unwrap()
    }

    #[test]
    fn polywire_straight_line() {
        // 2-point line → 1 segment → 1 ring-to-ring connection.
        // With 8 divisions: 2 rings * 8 pts = 16 points, 1 * 8 quads = 8 prims.
        let params = PolyWireParams {
            radius: 0.1,
            divisions: 8,
        };
        let result = make_line(2).apply(&PolyWireSop, &params).unwrap();

        assert_eq!(
            result.num_points(),
            16,
            "expected 2 rings * 8 divisions = 16 points"
        );
        assert_eq!(
            result.num_prims(),
            8,
            "expected 1 segment * 8 divisions = 8 quads"
        );
    }

    #[test]
    fn polywire_multi_segment() {
        // 4-point polyline → 3 segments.
        // With 8 divisions: 4 rings * 8 pts = 32 points, 3 * 8 = 24 quads.
        let params = PolyWireParams {
            radius: 0.1,
            divisions: 8,
        };
        let result = make_line(4).apply(&PolyWireSop, &params).unwrap();

        assert_eq!(result.num_points(), 32, "expected 4 rings * 8 = 32 points");
        assert_eq!(result.num_prims(), 24, "expected 3 segments * 8 = 24 quads");
    }

    #[test]
    fn polywire_radius() {
        // A straight line along Y from 0 to 1 with radius 0.5.
        // The bounding box should span from -0.5 to 0.5 in both X and Z.
        let params = PolyWireParams {
            radius: 0.5,
            divisions: 32, // High divisions for a close-to-circular cross-section
        };
        let result = make_line(2).apply(&PolyWireSop, &params).unwrap();

        let bb = result.bounding_box();
        // X extent should be approximately [-0.5, 0.5]
        assert_relative_eq!(bb.min.x, -0.5, epsilon = 0.05);
        assert_relative_eq!(bb.max.x, 0.5, epsilon = 0.05);
        // Z extent should be approximately [-0.5, 0.5]
        assert_relative_eq!(bb.min.z, -0.5, epsilon = 0.05);
        assert_relative_eq!(bb.max.z, 0.5, epsilon = 0.05);
    }

    #[test]
    fn polywire_divisions() {
        // More divisions means more points per ring.
        let line = make_line(2);
        let r4 = line
            .clone()
            .apply(
                &PolyWireSop,
                &PolyWireParams {
                    radius: 0.1,
                    divisions: 4,
                },
            )
            .unwrap();

        let r16 = line
            .apply(
                &PolyWireSop,
                &PolyWireParams {
                    radius: 0.1,
                    divisions: 16,
                },
            )
            .unwrap();

        assert_eq!(r4.num_points(), 2 * 4, "4 divisions * 2 rings = 8 points");
        assert_eq!(
            r16.num_points(),
            2 * 16,
            "16 divisions * 2 rings = 32 points"
        );
        assert!(
            r16.num_points() > r4.num_points(),
            "more divisions should produce more points"
        );
    }
}
