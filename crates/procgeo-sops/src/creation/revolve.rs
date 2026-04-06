use std::f32::consts::TAU;

use glam::{Mat3, Vec3};
use serde::{Deserialize, Serialize};

use procgeo_core::Geometry;
use procgeo_core::handle::PrimHandle;

use crate::{Sop, SopError};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RevolveParams {
    /// Center point of the revolution axis.
    pub origin: Vec3,
    /// Direction of the revolution axis (will be normalized).
    pub axis: Vec3,
    /// Number of edges around the revolution.
    pub divisions: u32,
    /// Start angle in degrees.
    pub start_angle: f32,
    /// End angle in degrees.
    pub end_angle: f32,
    /// Whether to generate end caps on partial revolutions.
    pub end_caps: bool,
}

impl Default for RevolveParams {
    fn default() -> Self {
        RevolveParams {
            origin: Vec3::ZERO,
            axis: Vec3::Y,
            divisions: 24,
            start_angle: 0.0,
            end_angle: 360.0,
            end_caps: false,
        }
    }
}

pub struct RevolveSop;

/// Build a rotation matrix for `angle` radians around a normalized `axis`
/// using Rodrigues' rotation formula.
fn rotation_matrix(axis: Vec3, angle: f32) -> Mat3 {
    let (s, c) = angle.sin_cos();
    let t = 1.0 - c;
    let Vec3 { x, y, z } = axis;
    Mat3::from_cols(
        Vec3::new(t * x * x + c, t * x * y + s * z, t * x * z - s * y),
        Vec3::new(t * x * y - s * z, t * y * y + c, t * y * z + s * x),
        Vec3::new(t * x * z + s * y, t * y * z - s * x, t * z * z + c),
    )
}

impl Sop for RevolveSop {
    type Params = RevolveParams;

    fn name(&self) -> &'static str {
        "revolve"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 1)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;

        if params.divisions < 3 {
            return Err(SopError::InvalidParam(format!(
                "divisions must be >= 3, got {}",
                params.divisions
            )));
        }

        let axis = params.axis.normalize_or_zero();
        if axis.length_squared() < 0.5 {
            return Err(SopError::InvalidParam(
                "axis must be non-zero".to_string(),
            ));
        }

        let input = inputs[0];
        let origin = params.origin;
        let divs = params.divisions as usize;
        let start_rad = params.start_angle.to_radians();
        let end_rad = params.end_angle.to_radians();
        let sweep = end_rad - start_rad;
        let is_full = (sweep.abs() - TAU).abs() < 1e-5;

        // Number of rings: full revolution shares first/last, partial needs an extra column.
        let num_cols = if is_full { divs } else { divs + 1 };

        // Collect profile points from each input polyline.
        // A "profile" is the ordered sequence of points along each polyline prim.
        let mut profiles: Vec<Vec<Vec3>> = Vec::new();
        for prim_idx in 0..input.num_prims() {
            let ph = PrimHandle::from_index(prim_idx);
            let pts = input.prim_points(ph);
            let profile: Vec<Vec3> = pts
                .iter()
                .map(|&pt| input.point_pos(pt))
                .collect();
            if profile.len() >= 2 {
                profiles.push(profile);
            }
        }

        if profiles.is_empty() {
            return Err(SopError::InvalidParam(
                "input must contain at least one polyline with 2+ points".to_string(),
            ));
        }

        let mut geo = Geometry::new();

        for profile in &profiles {
            let num_rows = profile.len();

            // Generate rotated rings of the profile
            // rings[col][row] = PointHandle
            let mut rings: Vec<Vec<_>> = Vec::with_capacity(num_cols);
            for col_idx in 0..num_cols {
                let t = col_idx as f32 / divs as f32;
                let angle = start_rad + t * sweep;
                let rot = rotation_matrix(axis, angle);

                let mut ring = Vec::with_capacity(num_rows);
                for pos in profile {
                    // Translate so origin is at world origin, rotate, translate back
                    let local = *pos - origin;
                    let rotated = rot * local + origin;
                    ring.push(geo.add_point(rotated));
                }
                rings.push(ring);
            }

            // Generate quads between adjacent columns
            let col_count = if is_full { divs } else { divs };
            for col_idx in 0..col_count {
                let cur = &rings[col_idx];
                let next = if is_full {
                    &rings[(col_idx + 1) % divs]
                } else {
                    &rings[col_idx + 1]
                };
                for row_idx in 0..(num_rows - 1) {
                    geo.add_face(&[
                        cur[row_idx],
                        cur[row_idx + 1],
                        next[row_idx + 1],
                        next[row_idx],
                    ]);
                }
            }

            // End caps for partial revolutions
            if !is_full && params.end_caps && num_rows >= 3 {
                // Start cap: first column of points as a face
                let start_ring: Vec<_> = rings[0].iter().copied().collect();
                geo.add_face(&start_ring);

                // End cap: last column of points as a face (reversed winding)
                let end_ring: Vec<_> = rings[num_cols - 1].iter().rev().copied().collect();
                geo.add_face(&end_ring);
            }
        }

        Ok(geo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn make_profile_line() -> Geometry {
        // Vertical line at x=1: profile for a cylinder-like revolve
        let mut geo = Geometry::new();
        let p0 = geo.add_point(Vec3::new(1.0, 0.0, 0.0));
        let p1 = geo.add_point(Vec3::new(1.0, 1.0, 0.0));
        geo.add_polyline(&[p0, p1]);
        geo
    }

    fn make_vase_profile() -> Geometry {
        // Simple vase profile: 4 points
        let mut geo = Geometry::new();
        let pts: Vec<_> = [
            Vec3::new(0.5, 0.0, 0.0),
            Vec3::new(1.0, 0.5, 0.0),
            Vec3::new(0.7, 1.0, 0.0),
            Vec3::new(0.3, 1.5, 0.0),
        ]
        .iter()
        .map(|&p| geo.add_point(p))
        .collect();
        geo.add_polyline(&pts);
        geo
    }

    #[test]
    fn revolve_full_circle() {
        let profile = make_profile_line();
        let sop = RevolveSop;
        let params = RevolveParams {
            divisions: 8,
            ..Default::default()
        };
        let geo = sop.execute(&[&profile], &params).unwrap();

        // Full revolution: 8 columns, 2 rows = 16 points
        // Quads: 8 columns * 1 row-pair = 8
        assert_eq!(geo.num_points(), 16);
        assert_eq!(geo.num_prims(), 8);
    }

    #[test]
    fn revolve_half_circle() {
        let profile = make_profile_line();
        let sop = RevolveSop;
        let params = RevolveParams {
            divisions: 8,
            start_angle: 0.0,
            end_angle: 180.0,
            ..Default::default()
        };
        let geo = sop.execute(&[&profile], &params).unwrap();

        // Partial: 8+1=9 columns, 2 rows = 18 points
        // Quads: 8 * 1 = 8
        assert_eq!(geo.num_points(), 18);
        assert_eq!(geo.num_prims(), 8);
    }

    #[test]
    fn revolve_half_with_caps() {
        let profile = make_vase_profile();
        let sop = RevolveSop;
        let params = RevolveParams {
            divisions: 8,
            start_angle: 0.0,
            end_angle: 180.0,
            end_caps: true,
            ..Default::default()
        };
        let geo = sop.execute(&[&profile], &params).unwrap();

        // 9 columns * 4 rows = 36 points
        // Quads: 8 * 3 = 24, + 2 caps = 26
        assert_eq!(geo.num_points(), 36);
        assert_eq!(geo.num_prims(), 26);
    }

    #[test]
    fn revolve_symmetry() {
        let profile = make_profile_line();
        let sop = RevolveSop;
        let params = RevolveParams {
            divisions: 16,
            ..Default::default()
        };
        let geo = sop.execute(&[&profile], &params).unwrap();

        let bb = geo.bounding_box();
        // Revolved around Y axis, profile at x=1: should be [-1, 1] in x and z
        assert_relative_eq!(bb.min.x, -1.0, epsilon = 0.05);
        assert_relative_eq!(bb.max.x, 1.0, epsilon = 0.05);
        assert_relative_eq!(bb.min.z, -1.0, epsilon = 0.05);
        assert_relative_eq!(bb.max.z, 1.0, epsilon = 0.05);
    }

    #[test]
    fn revolve_custom_axis() {
        // Revolve around X axis instead of Y
        let mut profile = Geometry::new();
        let p0 = profile.add_point(Vec3::new(0.0, 1.0, 0.0));
        let p1 = profile.add_point(Vec3::new(1.0, 1.0, 0.0));
        profile.add_polyline(&[p0, p1]);

        let sop = RevolveSop;
        let params = RevolveParams {
            axis: Vec3::X,
            divisions: 8,
            ..Default::default()
        };
        let geo = sop.execute(&[&profile], &params).unwrap();

        // Should create a surface swept around X axis
        assert_eq!(geo.num_points(), 16);
        assert_eq!(geo.num_prims(), 8);

        let bb = geo.bounding_box();
        // Profile at y=1 revolved around X: y and z should span [-1, 1]
        assert_relative_eq!(bb.min.y, -1.0, epsilon = 0.05);
        assert_relative_eq!(bb.max.y, 1.0, epsilon = 0.05);
    }

    #[test]
    fn revolve_invalid_divisions() {
        let profile = make_profile_line();
        let sop = RevolveSop;
        let params = RevolveParams {
            divisions: 2,
            ..Default::default()
        };
        assert!(sop.execute(&[&profile], &params).is_err());
    }

    #[test]
    fn revolve_no_polylines() {
        // Geometry with only points, no primitives
        let mut geo = Geometry::new();
        geo.add_point(Vec3::ZERO);
        let sop = RevolveSop;
        assert!(sop.execute(&[&geo], &RevolveParams::default()).is_err());
    }
}
