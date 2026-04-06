use glam::Vec3;
use serde::{Deserialize, Serialize};

use procgeo_core::{Geometry, PointHandle, PolyType, Primitive, PrimHandle};

use crate::{Sop, SopError};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResampleParams {
    /// Target segment length.
    pub length: f32,
    /// Maximum number of output segments.
    pub max_segments: u32,
}

impl Default for ResampleParams {
    fn default() -> Self {
        ResampleParams {
            length: 0.1,
            max_segments: 1000,
        }
    }
}

pub struct ResampleSop;

/// Resample an open polyline to have evenly spaced points at `length` intervals.
fn resample_polyline(
    pts: &[Vec3],
    length: f32,
    max_segments: u32,
) -> Vec<Vec3> {
    if pts.len() < 2 {
        return pts.to_vec();
    }

    // Compute cumulative distances along the polyline
    let mut cumulative = vec![0.0_f32];
    for i in 1..pts.len() {
        let d = (pts[i] - pts[i - 1]).length();
        cumulative.push(cumulative[i - 1] + d);
    }

    let total_length = *cumulative.last().unwrap();
    if total_length < 1e-10 || length <= 0.0 {
        return pts.to_vec();
    }

    let num_segments = ((total_length / length).round() as u32).max(1).min(max_segments);
    let actual_length = total_length / num_segments as f32;

    let mut result = Vec::with_capacity(num_segments as usize + 1);

    // Always include first point
    result.push(pts[0]);

    // Walk along original polyline, placing points at uniform intervals
    let mut seg_idx = 0usize; // current segment in original polyline

    for s in 1..=num_segments {
        let target = s as f32 * actual_length;

        // Advance seg_idx until we pass the target
        while seg_idx + 1 < cumulative.len() - 1 && cumulative[seg_idx + 1] < target {
            seg_idx += 1;
        }

        // Interpolate within segment [seg_idx, seg_idx+1]
        let seg_start = cumulative[seg_idx];
        let seg_end = cumulative[seg_idx + 1];
        let seg_len = seg_end - seg_start;

        let t = if seg_len > 1e-10 {
            ((target - seg_start) / seg_len).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let p = pts[seg_idx].lerp(pts[seg_idx + 1], t);
        result.push(p);
    }

    result
}

impl Sop for ResampleSop {
    type Params = ResampleParams;

    fn name(&self) -> &'static str {
        "resample"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 1)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;
        let geo = inputs[0];
        let mut out = Geometry::new();

        for prim_idx in 0..geo.num_prims() {
            let ph = PrimHandle::from_index(prim_idx);
            let prim = geo.prim(ph);

            match prim {
                Primitive::Polygon(poly) => {
                    let orig_pts = geo.prim_points(ph);
                    match poly.poly_type {
                        PolyType::Open => {
                            // Resample this polyline
                            let positions: Vec<Vec3> = orig_pts
                                .iter()
                                .map(|&h| geo.point_pos(h))
                                .collect();

                            let resampled = resample_polyline(
                                &positions,
                                params.length,
                                params.max_segments,
                            );

                            // Add new points and build polyline
                            let new_handles: Vec<PointHandle> = resampled
                                .iter()
                                .map(|&pos| out.add_point(pos))
                                .collect();

                            out.add_polyline(&new_handles);
                        }
                        PolyType::Closed => {
                            // Pass through closed polygons unchanged
                            let new_handles: Vec<PointHandle> = orig_pts
                                .iter()
                                .map(|&h| out.add_point(geo.point_pos(h)))
                                .collect();
                            out.add_face(&new_handles);
                        }
                    }
                }
            }
        }

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use glam::Vec3;
    use crate::creation::box_sop::{BoxSop, BoxParams};
    use crate::{GeometryExt, generate};

    /// Build a 5-point open polyline along X from 0 to 4
    fn make_line() -> Geometry {
        let mut geo = Geometry::new();
        let handles: Vec<PointHandle> = (0..=4)
            .map(|i| geo.add_point(Vec3::new(i as f32, 0.0, 0.0)))
            .collect();
        geo.add_polyline(&handles);
        geo
    }

    #[test]
    fn resample_line() {
        // Line from 0 to 4 (total length 4), resample at length 1.0 → 5 points (0,1,2,3,4)
        let geo = make_line();
        let params = ResampleParams {
            length: 1.0,
            max_segments: 1000,
        };
        let result = geo.apply(&ResampleSop, &params).unwrap();

        // Should produce 1 polyline prim
        assert_eq!(result.num_prims(), 1, "should have 1 polyline");

        let ph = PrimHandle::from_index(0);
        let pts = result.prim_points(ph);
        assert_eq!(pts.len(), 5, "should have 5 points (segments=4, pts=5)");

        // Points should be evenly spaced at x = 0, 1, 2, 3, 4
        for (i, &h) in pts.iter().enumerate() {
            let pos = result.point_pos(h);
            assert_relative_eq!(pos.x, i as f32, epsilon = 1e-4);
            assert_relative_eq!(pos.y, 0.0, epsilon = 1e-4);
            assert_relative_eq!(pos.z, 0.0, epsilon = 1e-4);
        }
    }

    #[test]
    fn resample_preserves_closed() {
        // A closed polygon should pass through unchanged
        let box_geo = generate(&BoxSop, &BoxParams::default()).unwrap();
        let num_prims_before = box_geo.num_prims();

        let params = ResampleParams::default();
        let result = box_geo.apply(&ResampleSop, &params).unwrap();

        assert_eq!(
            result.num_prims(),
            num_prims_before,
            "closed polygons should pass through unchanged"
        );

        // Each prim should still be closed
        use procgeo_core::{Primitive, PolyType};
        for i in 0..result.num_prims() {
            let ph = PrimHandle::from_index(i);
            let prim = result.prim(ph);
            match prim {
                Primitive::Polygon(poly) => {
                    assert_eq!(poly.poly_type, PolyType::Closed, "prim {i} should be closed");
                }
            }
        }
    }
}
