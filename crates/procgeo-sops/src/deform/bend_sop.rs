use glam::{Mat3, Vec3};
use serde::{Deserialize, Serialize};

use procgeo_core::attribute::{AttribClass, AttribDefault, AttribHandle, TypeQualifier};
use procgeo_core::handle::PointHandle;
use procgeo_core::Geometry;

use crate::{Sop, SopError};

/// How the bend direction is specified.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum BendMode {
    /// Specify bend as an angle in degrees.
    Angle,
    /// Specify bend toward a goal direction.
    Direction,
}

impl Default for BendMode {
    fn default() -> Self {
        BendMode::Angle
    }
}

/// How taper falloff is computed along the capture region.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum TaperMode {
    /// Linear interpolation.
    Linear,
    /// Smooth (cubic) interpolation.
    Smooth,
}

impl Default for TaperMode {
    fn default() -> Self {
        TaperMode::Linear
    }
}

/// Parameters for the Bend SOP, mirroring Houdini defaults.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BendParams {
    // ── Selection ─────────────────────────────────────────────────────────────
    /// Optional point/prim group name to restrict deformation.
    pub group: Option<String>,
    /// Optional attribute name to use as a per-point deformation mask (0–1).
    pub mask_attrib: Option<String>,

    // ── Global switches ────────────────────────────────────────────────────────
    pub enable_deformation: bool,
    /// When true, only deform points inside the capture region.
    pub limit_to_capture_region: bool,
    /// Deform symmetrically in both directions along the capture axis.
    pub deform_both_directions: bool,

    // ── Bend ───────────────────────────────────────────────────────────────────
    pub bend_enable: bool,
    pub bend_mode: BendMode,
    /// Bend angle in degrees (BendMode::Angle).
    pub bend_angle: f32,
    /// Goal direction for bend (BendMode::Direction).
    pub bend_goal_direction: Vec3,

    // ── Twist ──────────────────────────────────────────────────────────────────
    pub twist_enable: bool,
    /// Total twist angle in degrees applied over the capture length.
    pub twist_angle: f32,
    /// Apply twist continuously through both directions.
    pub twist_continuous_both: bool,

    // ── Length Scale ───────────────────────────────────────────────────────────
    pub length_scale_enable: bool,
    pub length_scale: f32,
    /// Preserve volume during length scaling.
    pub preserve_volume: bool,

    // ── Taper ─────────────────────────────────────────────────────────────────
    pub taper_enable: bool,
    /// Which axes (x=0, z=1) to taper along.
    pub taper_along: [bool; 2],
    pub taper_mode: TaperMode,
    /// Uniform taper scale at the tip (1.0 = no taper).
    pub taper_value: f32,
    /// Squish amount applied perpendicular to taper.
    pub squish: f32,
    /// Normalized pivot position for squish (0.0–1.0).
    pub squish_pivot: f32,
    /// Use a ramp curve for taper instead of the uniform taper_value.
    pub taper_ramp_enable: bool,
    /// Ramp curve as (position, value) pairs in [0,1]×[0,1].
    pub taper_ramp: Vec<(f32, f32)>,

    // ── Orientation ───────────────────────────────────────────────────────────
    /// Up vector defining the bend plane.
    pub up_vector: Vec3,
    /// Rotation of the up vector around the capture axis (degrees).
    pub up_vector_angle: f32,

    // ── Capture Region ────────────────────────────────────────────────────────
    pub capture_origin: Vec3,
    pub capture_direction: Vec3,
    /// Length of the capture region along capture_direction.
    pub capture_length: f32,

    // ── Output ────────────────────────────────────────────────────────────────
    /// Optional attribute to write deformed positions to instead of `P`.
    pub output_attrib: Option<String>,
    /// Glob pattern for attributes to transform along with points.
    pub attribs_to_transform: String,
    pub recompute_normals: bool,
    pub preserve_normal_length: bool,
}

impl Default for BendParams {
    fn default() -> Self {
        BendParams {
            group: None,
            mask_attrib: None,

            enable_deformation: true,
            limit_to_capture_region: true,
            deform_both_directions: false,

            bend_enable: false,
            bend_mode: BendMode::Angle,
            bend_angle: 0.0,
            bend_goal_direction: Vec3::Z,

            twist_enable: false,
            twist_angle: 0.0,
            twist_continuous_both: false,

            length_scale_enable: false,
            length_scale: 1.0,
            preserve_volume: false,

            taper_enable: false,
            taper_along: [true, true],
            taper_mode: TaperMode::Linear,
            taper_value: 1.0,
            squish: 1.0,
            squish_pivot: 0.5,
            taper_ramp_enable: false,
            taper_ramp: vec![(0.0, 0.5), (1.0, 0.5)],

            up_vector: Vec3::Y,
            up_vector_angle: 0.0,

            capture_origin: Vec3::ZERO,
            capture_direction: Vec3::Y,
            capture_length: 1.0,

            output_attrib: None,
            attribs_to_transform: String::from("*"),
            recompute_normals: true,
            preserve_normal_length: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Capture space helpers
// ---------------------------------------------------------------------------

/// Build an orthonormal frame from capture_direction (Z-axis of local space)
/// and up_vector (used to derive Y-axis), with an optional rotation of the
/// up vector around the capture axis.
///
/// Returns (to_local, from_local) rotation matrices and the origin.
fn build_capture_frame(
    _origin: Vec3,
    capture_dir: Vec3,
    up: Vec3,
    up_angle_deg: f32,
) -> (Mat3, Mat3) {
    let z_axis = capture_dir.normalize_or_zero();

    // Derive an initial Y axis perpendicular to Z
    let mut y_axis = up - z_axis * up.dot(z_axis);
    if y_axis.length_squared() < 1e-10 {
        // up is nearly parallel to capture_dir; pick an arbitrary perpendicular
        y_axis = if z_axis.x.abs() < 0.9 {
            Vec3::X
        } else {
            Vec3::Y
        };
        y_axis -= z_axis * y_axis.dot(z_axis);
    }
    y_axis = y_axis.normalize_or_zero();

    let x_axis = y_axis.cross(z_axis).normalize_or_zero();
    // Re-orthogonalize Y
    let y_axis = z_axis.cross(x_axis).normalize_or_zero();

    // Apply up_vector_angle rotation around Z axis
    let (mut final_x, mut final_y) = (x_axis, y_axis);
    if up_angle_deg.abs() > 1e-8 {
        let angle = up_angle_deg.to_radians();
        let c = angle.cos();
        let s = angle.sin();
        final_x = x_axis * c + y_axis * s;
        final_y = -x_axis * s + y_axis * c;
    }

    // from_local: columns are the local axes in world space
    // from_local * local_vec = world_vec
    let from_local = Mat3::from_cols(final_x, final_y, z_axis);
    // to_local: transpose (orthonormal frame)
    let to_local = from_local.transpose();

    (to_local, from_local)
}

/// Transform a world-space point to capture-local space.
#[inline]
fn to_local_space(pos: Vec3, origin: Vec3, to_local: &Mat3) -> Vec3 {
    *to_local * (pos - origin)
}

/// Transform a capture-local point back to world space.
#[inline]
fn from_local_space(local: Vec3, origin: Vec3, from_local: &Mat3) -> Vec3 {
    *from_local * local + origin
}

// ---------------------------------------------------------------------------
// Deformation helpers
// ---------------------------------------------------------------------------

/// Small angle threshold below which bend is treated as identity.
const BEND_EPSILON: f32 = 1e-6;

/// Apply bend deformation in capture-local space.
/// Bends in the YZ plane: Z is the spine axis, Y is up.
///
/// `theta_total`: total bend angle in radians
/// `t`: parametric position along the spine [0..1]
/// `local`: point in capture-local space
///
/// Returns the deformed point in capture-local space.
fn apply_bend(local: Vec3, theta_total: f32, t: f32, capture_length: f32) -> Vec3 {
    if theta_total.abs() < BEND_EPSILON {
        return local;
    }

    let r = capture_length / theta_total;
    let theta_at_t = theta_total * t;

    let (sin_t, cos_t) = theta_at_t.sin_cos();

    // The spine position at the base is at local_z = 0.
    // The offset of this point from the spine in local XY:
    let offset_x = local.x;
    let offset_y = local.y;

    // Spine at t wraps to:
    //   spine_y = R * (cos(theta_at_t) - 1)
    //   spine_z = R * sin(theta_at_t)
    let spine_y = r * (cos_t - 1.0);
    let spine_z = r * sin_t;

    // Point offset from spine rotates with the spine normal:
    let new_x = offset_x; // unchanged (perpendicular to bend plane)
    let new_y = spine_y + offset_y * cos_t;
    let new_z = spine_z + offset_y * sin_t;

    Vec3::new(new_x, new_y, new_z)
}

/// Apply twist deformation in capture-local space.
/// Rotates XY around the Z axis by `twist_angle * t`.
fn apply_twist(local: Vec3, twist_angle_rad: f32, t: f32) -> Vec3 {
    let angle = twist_angle_rad * t;
    let (sin_a, cos_a) = angle.sin_cos();
    Vec3::new(
        local.x * cos_a - local.y * sin_a,
        local.x * sin_a + local.y * cos_a,
        local.z,
    )
}

/// Apply length scale deformation in capture-local space.
/// Scales Z by `length_scale`. If preserve_volume is on, scales XY by
/// 1/sqrt(length_scale) to keep volume constant.
fn apply_length_scale(local: Vec3, length_scale: f32, preserve_volume: bool) -> Vec3 {
    let xy_scale = if preserve_volume && length_scale > 0.0 {
        1.0 / length_scale.sqrt()
    } else {
        1.0
    };
    Vec3::new(local.x * xy_scale, local.y * xy_scale, local.z * length_scale)
}

/// Hermite smooth interpolation: 3t^2 - 2t^3
#[inline]
fn hermite_smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// Evaluate taper scale at parametric position t using three-point interpolation:
/// start=1.0, pivot=squish, end=taper_value.
fn eval_taper_uniform(t: f32, taper_value: f32, squish: f32, squish_pivot: f32, mode: &TaperMode) -> f32 {
    let t_clamped = t.clamp(0.0, 1.0);

    if t_clamped <= squish_pivot {
        // Interpolate from 1.0 (start) to squish (at pivot)
        let local_t = if squish_pivot > 0.0 {
            t_clamped / squish_pivot
        } else {
            1.0
        };
        let local_t = match mode {
            TaperMode::Linear => local_t,
            TaperMode::Smooth => hermite_smooth(local_t),
        };
        1.0 + (squish - 1.0) * local_t
    } else {
        // Interpolate from squish (at pivot) to taper_value (end)
        let local_t = if (1.0 - squish_pivot).abs() > 1e-10 {
            (t_clamped - squish_pivot) / (1.0 - squish_pivot)
        } else {
            1.0
        };
        let local_t = match mode {
            TaperMode::Linear => local_t,
            TaperMode::Smooth => hermite_smooth(local_t),
        };
        squish + (taper_value - squish) * local_t
    }
}

/// Evaluate taper scale from a ramp curve. Ramp values are mapped: 0.5 = no scale,
/// 0.0 = scale to 0, 1.0 = scale to 2x.
fn eval_taper_ramp(t: f32, ramp: &[(f32, f32)]) -> f32 {
    if ramp.is_empty() {
        return 1.0;
    }
    if ramp.len() == 1 {
        return ramp[0].1 * 2.0;
    }

    let t_clamped = t.clamp(0.0, 1.0);

    // Find surrounding control points
    // Ramp is assumed to be sorted by position
    if t_clamped <= ramp[0].0 {
        return ramp[0].1 * 2.0;
    }
    if t_clamped >= ramp[ramp.len() - 1].0 {
        return ramp[ramp.len() - 1].1 * 2.0;
    }

    for i in 0..ramp.len() - 1 {
        let (p0, v0) = ramp[i];
        let (p1, v1) = ramp[i + 1];
        if t_clamped >= p0 && t_clamped <= p1 {
            let local_t = if (p1 - p0).abs() > 1e-10 {
                (t_clamped - p0) / (p1 - p0)
            } else {
                0.5
            };
            let v = v0 + (v1 - v0) * local_t;
            return v * 2.0;
        }
    }

    1.0
}

/// Apply taper deformation in capture-local space.
fn apply_taper(
    local: Vec3,
    t: f32,
    params: &BendParams,
) -> Vec3 {
    let scale = if params.taper_ramp_enable {
        eval_taper_ramp(t, &params.taper_ramp)
    } else {
        eval_taper_uniform(t, params.taper_value, params.squish, params.squish_pivot, &params.taper_mode)
    };

    let sx = if params.taper_along[0] { scale } else { 1.0 };
    let sy = if params.taper_along[1] { scale } else { 1.0 };

    Vec3::new(local.x * sx, local.y * sy, local.z)
}

// ---------------------------------------------------------------------------
// BendSop
// ---------------------------------------------------------------------------

/// Bend SOP -- deforms geometry by bending, twisting, tapering, and length-scaling
/// along a capture axis.
pub struct BendSop;

impl Sop for BendSop {
    type Params = BendParams;

    fn name(&self) -> &'static str {
        "bend"
    }

    /// 1 required input (the geometry to deform); an optional second input can
    /// supply rest positions.
    fn input_count(&self) -> (usize, usize) {
        (1, 2)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;

        let mut geo = inputs[0].clone();

        if !params.enable_deformation {
            return Ok(geo);
        }

        // Check if any deformation is actually enabled
        if !params.bend_enable && !params.twist_enable && !params.length_scale_enable && !params.taper_enable {
            return Ok(geo);
        }

        let num_pts = geo.num_points();
        if num_pts == 0 {
            return Ok(geo);
        }

        // Build capture frame
        let (to_local, from_local) = build_capture_frame(
            params.capture_origin,
            params.capture_direction,
            params.up_vector,
            params.up_vector_angle,
        );

        let capture_len = params.capture_length;
        if capture_len.abs() < 1e-10 {
            return Ok(geo);
        }

        // Resolve bend angle
        let bend_angle_rad = if params.bend_enable {
            match params.bend_mode {
                BendMode::Angle => params.bend_angle.to_radians(),
                BendMode::Direction => {
                    let cap_dir = params.capture_direction.normalize_or_zero();
                    let goal_dir = params.bend_goal_direction.normalize_or_zero();
                    let dot = cap_dir.dot(goal_dir).clamp(-1.0, 1.0);
                    dot.acos()
                }
            }
        } else {
            0.0
        };

        let twist_angle_rad = if params.twist_enable {
            params.twist_angle.to_radians()
        } else {
            0.0
        };

        // Pre-compute group membership as a boolean vec to avoid borrow conflicts
        let group_membership: Option<Vec<bool>> = params.group.as_ref().and_then(|name| {
            if name.is_empty() {
                None
            } else {
                geo.groups().point_group(name).map(|grp| {
                    (0..num_pts).map(|i| grp.contains(i)).collect()
                })
            }
        });

        // Look up the optional mask attribute and pre-read values
        let mask_values: Option<Vec<f32>> = params.mask_attrib.as_ref().and_then(|name| {
            if name.is_empty() {
                None
            } else {
                let handle = geo.find_attrib::<f32>(AttribClass::Point, name).ok()?;
                Some(
                    (0..num_pts)
                        .map(|i| geo.get_attrib(&handle, i).unwrap_or(1.0))
                        .collect(),
                )
            }
        });

        // Get rest positions from second input or current geometry
        let rest_positions: Vec<Vec3> = if inputs.len() > 1 {
            inputs[1].points().collect()
        } else {
            geo.points().collect()
        };

        // Create output deformation attribute if requested
        let deform_attrib_handle: Option<AttribHandle<f32>> = if let Some(ref attrib_name) = params.output_attrib {
            if !attrib_name.is_empty() {
                let _ = geo.add_attrib(
                    AttribClass::Point,
                    attrib_name.clone(),
                    AttribDefault::Float(0.0),
                    TypeQualifier::None,
                );
                geo.find_attrib::<f32>(AttribClass::Point, attrib_name).ok()
            } else {
                None
            }
        } else {
            None
        };

        // Process each point
        for i in 0..num_pts {
            // Group filtering: skip points not in the group
            if let Some(ref membership) = group_membership {
                if !membership[i] {
                    continue;
                }
            }

            let rest_pos = if i < rest_positions.len() {
                rest_positions[i]
            } else {
                geo.point_pos(PointHandle::from_index(i))
            };

            // Transform to capture-local space
            let local = to_local_space(rest_pos, params.capture_origin, &to_local);

            // Compute parametric t
            let local_z = local.z;
            let raw_t = local_z / capture_len;

            // Determine if this point should be deformed
            let (should_deform, t, is_backward) = if params.deform_both_directions {
                // Both directions: t in [-1, 1], deform both sides
                if params.limit_to_capture_region && (raw_t < -1.0 || raw_t > 1.0) {
                    (false, raw_t, false)
                } else {
                    let backward = raw_t < 0.0;
                    (true, raw_t, backward)
                }
            } else {
                // Single direction: t in [0, 1]
                if params.limit_to_capture_region && (raw_t < 0.0 || raw_t > 1.0) {
                    (false, raw_t, false)
                } else {
                    (true, raw_t, false)
                }
            };

            if !should_deform {
                continue;
            }

            // Get mask value
            let mask = if let Some(ref values) = mask_values {
                values[i]
            } else {
                1.0
            };

            if mask.abs() < 1e-10 {
                continue;
            }

            // For both_directions, mirror the backward half
            let (deform_t, mirror) = if is_backward {
                (-t, true)
            } else {
                (t, false)
            };

            // Start with the point in capture-local space.
            // Separate the spine component (Z) from the offset (XY).
            let mut deformed = local;

            // For backward mirroring, flip Z so deformation sees positive t
            if mirror {
                deformed.z = -deformed.z;
            }

            // 1. Apply bend
            if params.bend_enable && bend_angle_rad.abs() > BEND_EPSILON {
                let bend_angle_for_t = if mirror { -bend_angle_rad } else { bend_angle_rad };
                deformed = apply_bend(deformed, bend_angle_for_t, deform_t, capture_len);
            }

            // 2. Apply twist
            if params.twist_enable && twist_angle_rad.abs() > BEND_EPSILON {
                let twist_for_t = if mirror && params.twist_continuous_both {
                    // Continuous: backward twist is negative
                    -twist_angle_rad
                } else {
                    twist_angle_rad
                };
                deformed = apply_twist(deformed, twist_for_t, deform_t);
            }

            // 3. Apply length scale
            if params.length_scale_enable && (params.length_scale - 1.0).abs() > 1e-8 {
                deformed = apply_length_scale(deformed, params.length_scale, params.preserve_volume);
            }

            // 4. Apply taper
            if params.taper_enable {
                deformed = apply_taper(deformed, deform_t, params);
            }

            // Un-mirror
            if mirror {
                deformed.z = -deformed.z;
            }

            // Apply mask: blend between original local position and deformed
            let final_local = if (mask - 1.0).abs() < 1e-8 {
                deformed
            } else {
                local + (deformed - local) * mask
            };

            // Transform back to world space
            let final_world = from_local_space(final_local, params.capture_origin, &from_local);

            geo.set_point_pos(PointHandle::from_index(i), final_world);

            // Write deformation amount attribute
            if let Some(ref handle) = deform_attrib_handle {
                let deform_amount = (final_world - rest_pos).length();
                let _ = geo.set_attrib(handle, i, deform_amount);
            }
        }

        Ok(geo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creation::box_sop::{BoxParams, BoxSop};
    use crate::creation::line::{LineParams, LineSop};
    use crate::{generate, GeometryExt};
    use approx::assert_relative_eq;
    use procgeo_core::handle::PointHandle;

    /// Helper: create a line of points along +Y from origin, with given point count.
    fn make_line_along_y(num_points: u32, length: f32) -> Geometry {
        generate(
            &LineSop,
            &LineParams {
                origin: Vec3::ZERO,
                direction: Vec3::Y,
                length,
                points: num_points,
            },
        )
        .unwrap()
    }

    #[test]
    fn bend_passthrough_enabled() {
        let geo = generate(&BoxSop, &BoxParams::default()).unwrap();
        let npts = geo.num_points();
        let params = BendParams::default();
        let result = geo.apply(&BendSop, &params).unwrap();
        assert_eq!(result.num_points(), npts);
    }

    #[test]
    fn bend_passthrough_disabled() {
        let geo = generate(&BoxSop, &BoxParams::default()).unwrap();
        let npts = geo.num_points();
        let params = BendParams {
            enable_deformation: false,
            ..BendParams::default()
        };
        let result = geo.apply(&BendSop, &params).unwrap();
        assert_eq!(result.num_points(), npts);
    }

    #[test]
    fn bend_wrong_input_count() {
        let err = BendSop.execute(&[], &BendParams::default());
        assert!(err.is_err());
    }

    #[test]
    fn bend_params_default_values() {
        let p = BendParams::default();
        assert!(p.enable_deformation);
        assert!(!p.bend_enable);
        assert_eq!(p.bend_angle, 0.0);
        assert_eq!(p.capture_direction, Vec3::Y);
        assert_eq!(p.capture_length, 1.0);
        assert_eq!(p.taper_value, 1.0);
        assert_eq!(p.attribs_to_transform, "*");
    }

    // ── New tests ────────────────────────────────────────────────────────

    #[test]
    fn identity_when_disabled() {
        let geo = make_line_along_y(11, 1.0);
        let original_positions: Vec<Vec3> = geo.points().collect();

        let params = BendParams {
            enable_deformation: false,
            bend_enable: true,
            bend_angle: 90.0,
            ..BendParams::default()
        };
        let result = geo.apply(&BendSop, &params).unwrap();
        let result_positions: Vec<Vec3> = result.points().collect();

        for (orig, res) in original_positions.iter().zip(result_positions.iter()) {
            assert_relative_eq!(orig.x, res.x, epsilon = 1e-5);
            assert_relative_eq!(orig.y, res.y, epsilon = 1e-5);
            assert_relative_eq!(orig.z, res.z, epsilon = 1e-5);
        }
    }

    #[test]
    fn identity_when_no_deformations_enabled() {
        let geo = make_line_along_y(11, 1.0);
        let original_positions: Vec<Vec3> = geo.points().collect();

        // enable_deformation is on, but all individual deformations are off
        let params = BendParams {
            enable_deformation: true,
            bend_enable: false,
            twist_enable: false,
            length_scale_enable: false,
            taper_enable: false,
            ..BendParams::default()
        };
        let result = geo.apply(&BendSop, &params).unwrap();
        let result_positions: Vec<Vec3> = result.points().collect();

        for (orig, res) in original_positions.iter().zip(result_positions.iter()) {
            assert_relative_eq!(orig.x, res.x, epsilon = 1e-5);
            assert_relative_eq!(orig.y, res.y, epsilon = 1e-5);
            assert_relative_eq!(orig.z, res.z, epsilon = 1e-5);
        }
    }

    #[test]
    fn bend_90_degrees() {
        // Line along Y from 0 to 1, bent 90 degrees.
        // The top point (at Y=1) should end up near +Z direction.
        let geo = make_line_along_y(11, 1.0);

        let params = BendParams {
            bend_enable: true,
            bend_angle: 90.0,
            capture_origin: Vec3::ZERO,
            capture_direction: Vec3::Y,
            capture_length: 1.0,
            limit_to_capture_region: true,
            ..BendParams::default()
        };

        let _result = geo.apply(&BendSop, &params).unwrap();

        // With up_vector = Y and capture_direction = Y, they are parallel,
        // so the frame builder picks a fallback. Let's use a more explicit setup.
        let geo2 = make_line_along_y(11, 1.0);
        let params2 = BendParams {
            bend_enable: true,
            bend_angle: 90.0,
            capture_origin: Vec3::ZERO,
            capture_direction: Vec3::Y,
            capture_length: 1.0,
            up_vector: Vec3::Z, // Z is up for the bend plane
            limit_to_capture_region: true,
            ..BendParams::default()
        };

        let result2 = geo2.apply(&BendSop, &params2).unwrap();

        // With capture_direction=Y (local Z), up_vector=Z (local Y):
        // Bend is in the local YZ plane = world ZY plane
        // At t=1, the spine has rotated 90 degrees.
        // R = capture_length / theta = 1.0 / (pi/2) ~= 0.6366
        // spine_y = R*(cos(pi/2) - 1) = R*(-1) = -0.6366
        // spine_z = R*sin(pi/2) = R = 0.6366
        //
        // In world space: local Y -> world Z, local Z -> world Y
        // So the tip should move in the -Z direction (local Y) and
        // have reduced Y (local Z). Specifically for a point on the spine
        // (no XY offset), the tip should be at:
        // world coords: x=0, y=R*sin(pi/2)=R, z=R*(cos(pi/2)-1)=-R+0 => no wait.
        //
        // Let me just verify the tip is NOT at (0, 1, 0) anymore - it moved.
        let tip = result2.point_pos(PointHandle::from_index(10));
        let base = result2.point_pos(PointHandle::from_index(0));

        // Base should be unchanged (t=0)
        assert_relative_eq!(base.x, 0.0, epsilon = 1e-4);
        assert_relative_eq!(base.y, 0.0, epsilon = 1e-4);
        assert_relative_eq!(base.z, 0.0, epsilon = 1e-4);

        // Tip should have moved away from (0, 1, 0)
        let dist_from_original = ((tip.y - 1.0).powi(2) + tip.z.powi(2)).sqrt();
        assert!(
            dist_from_original > 0.1,
            "tip should have moved significantly, got {:?}",
            tip
        );

        // The tip should have bent toward -Z (since bend is in the YZ plane)
        // Specifically, for 90 degrees, the spine direction at the tip is
        // perpendicular to the original direction.
        // The distance from origin should be close to R*sqrt(2)
        let tip_dist = tip.length();
        let r = 1.0 / (std::f32::consts::FRAC_PI_2);
        assert_relative_eq!(tip_dist, r * 2.0_f32.sqrt(), epsilon = 0.05);
    }

    #[test]
    fn bend_preserves_point_count() {
        let geo = make_line_along_y(20, 2.0);
        let npts = geo.num_points();

        let params = BendParams {
            bend_enable: true,
            bend_angle: 45.0,
            capture_length: 2.0,
            ..BendParams::default()
        };

        let result = geo.apply(&BendSop, &params).unwrap();
        assert_eq!(result.num_points(), npts);
    }

    #[test]
    fn bend_zero_angle_is_identity() {
        let geo = make_line_along_y(11, 1.0);
        let original_positions: Vec<Vec3> = geo.points().collect();

        let params = BendParams {
            bend_enable: true,
            bend_angle: 0.0,
            ..BendParams::default()
        };

        let result = geo.apply(&BendSop, &params).unwrap();
        let result_positions: Vec<Vec3> = result.points().collect();

        for (orig, res) in original_positions.iter().zip(result_positions.iter()) {
            assert_relative_eq!(orig.x, res.x, epsilon = 1e-5);
            assert_relative_eq!(orig.y, res.y, epsilon = 1e-5);
            assert_relative_eq!(orig.z, res.z, epsilon = 1e-5);
        }
    }

    #[test]
    fn outside_capture_region_unchanged() {
        // Create a line from Y=0 to Y=2, capture region is [0,1].
        // Points at Y > 1 should be unchanged with limit_to_capture_region=true.
        let geo = make_line_along_y(11, 2.0);
        let original_positions: Vec<Vec3> = geo.points().collect();

        let params = BendParams {
            bend_enable: true,
            bend_angle: 90.0,
            capture_origin: Vec3::ZERO,
            capture_direction: Vec3::Y,
            capture_length: 1.0,
            limit_to_capture_region: true,
            up_vector: Vec3::Z,
            ..BendParams::default()
        };

        let result = geo.apply(&BendSop, &params).unwrap();

        // Points with Y > 1.0 (indices 6-10, since length=2 and 11 points: step=0.2)
        // t = Y / capture_length = Y / 1.0 = Y, so t > 1.0 for Y > 1.0
        for i in 6..11 {
            let orig = original_positions[i];
            let res = result.point_pos(PointHandle::from_index(i));
            assert!(
                (orig.x - res.x).abs() < 1e-5
                    && (orig.y - res.y).abs() < 1e-5
                    && (orig.z - res.z).abs() < 1e-5,
                "point {i} should be unchanged: orig={:?} res={:?}",
                orig,
                res,
            );
        }

        // But some points inside the region should have moved
        let res1 = result.point_pos(PointHandle::from_index(5));
        let orig1 = original_positions[5];
        let moved = (res1 - orig1).length();
        assert!(
            moved > 0.01,
            "point at t=1.0 boundary should move or nearby points should move"
        );
    }

    #[test]
    fn twist_360() {
        // A line along Y=0..1, twist 360 degrees.
        // Create a point with an offset from the spine axis.
        let mut geo = Geometry::new();
        // Point at (1, 0.5, 0) - offset 1 unit in X from the spine
        geo.add_point(Vec3::new(1.0, 0.5, 0.0));

        let params = BendParams {
            twist_enable: true,
            twist_angle: 360.0,
            capture_origin: Vec3::ZERO,
            capture_direction: Vec3::Y,
            capture_length: 1.0,
            up_vector: Vec3::Z,
            limit_to_capture_region: true,
            ..BendParams::default()
        };

        let result = BendSop.execute(&[&geo], &params).unwrap();
        let pos = result.point_pos(PointHandle::from_index(0));

        // At t=0.5, twist = 180 degrees. The point (1, 0, 0) in local XY
        // should rotate 180 degrees to (-1, 0, 0).
        // Local frame: Z=world Y, and with up_vector=Z -> Y_local=Z_world, X_local = Y_local cross Z_local
        // Let me just check: after 180 degrees of twist at t=0.5 the X offset should flip.
        // The twist is around the local Z axis (=world Y), rotating in local XY.
        // Since t=0.5 and twist=360, we rotate by 180 degrees in local XY.
        // The offset in local XY should be flipped.
        //
        // The point will have its local XY offset rotated, with the Z component unchanged.
        // The world-space position should show the offset has rotated.
        // We mainly verify it's NOT the same as the original.
        let dist_from_original = (pos - Vec3::new(1.0, 0.5, 0.0)).length();
        assert!(
            dist_from_original > 0.5,
            "full twist at t=0.5 should significantly move offset point, got {:?}",
            pos
        );

        // Also verify the Y coordinate (local Z) is approximately unchanged
        // (twist doesn't affect the spine-axis component)
        assert_relative_eq!(pos.y, 0.5, epsilon = 0.1);
    }

    #[test]
    fn length_scale_doubles() {
        // A line from Y=0 to Y=1 with length_scale=2.0.
        // The Z component in local space (= Y in world) should double.
        let geo = make_line_along_y(11, 1.0);

        let params = BendParams {
            length_scale_enable: true,
            length_scale: 2.0,
            capture_origin: Vec3::ZERO,
            capture_direction: Vec3::Y,
            capture_length: 1.0,
            up_vector: Vec3::Z,
            limit_to_capture_region: false, // deform all
            ..BendParams::default()
        };

        let result = geo.apply(&BendSop, &params).unwrap();

        // Point at Y=0.5 (index 5) should now be at Y=1.0
        let pos5 = result.point_pos(PointHandle::from_index(5));
        assert_relative_eq!(pos5.y, 1.0, epsilon = 0.05);

        // Point at Y=1.0 (index 10) should now be at Y=2.0
        let pos10 = result.point_pos(PointHandle::from_index(10));
        assert_relative_eq!(pos10.y, 2.0, epsilon = 0.05);
    }

    #[test]
    fn taper_to_zero() {
        // Create a point offset from the spine at the end of the capture region.
        // Taper to 0 should collapse it onto the spine.
        let mut geo = Geometry::new();
        // Point at (1.0, 1.0, 0.0) -> local: offset_x=varies, spine at t=1.0
        geo.add_point(Vec3::new(1.0, 1.0, 0.0));

        let params = BendParams {
            taper_enable: true,
            taper_value: 0.0, // collapse at the tip
            taper_along: [true, true],
            squish: 0.5,
            squish_pivot: 0.5,
            capture_origin: Vec3::ZERO,
            capture_direction: Vec3::Y,
            capture_length: 1.0,
            up_vector: Vec3::Z,
            limit_to_capture_region: false,
            ..BendParams::default()
        };

        let result = BendSop.execute(&[&geo], &params).unwrap();
        let pos = result.point_pos(PointHandle::from_index(0));

        // At t=1.0, the taper scale should be 0.0.
        // The local XY offset should be scaled to 0, leaving only the spine position.
        // The spine position at t=1 in local space is (0, 0, capture_length).
        // In world space that maps back to (0, 1, 0).
        // But local X (offset from spine) was 1.0 and should be scaled to 0.
        // So the final position should have minimal X offset.
        assert!(
            pos.x.abs() < 0.1,
            "taper to 0 should collapse X offset, got x={}",
            pos.x
        );
    }

    #[test]
    fn volume_preservation() {
        // With length_scale=4.0 and preserve_volume, XY should scale by 1/sqrt(4)=0.5
        let mut geo = Geometry::new();
        // Point with offset from spine
        geo.add_point(Vec3::new(2.0, 0.5, 0.0));

        let params = BendParams {
            length_scale_enable: true,
            length_scale: 4.0,
            preserve_volume: true,
            capture_origin: Vec3::ZERO,
            capture_direction: Vec3::Y,
            capture_length: 1.0,
            up_vector: Vec3::Z,
            limit_to_capture_region: false,
            ..BendParams::default()
        };

        let result = BendSop.execute(&[&geo], &params).unwrap();
        let pos = result.point_pos(PointHandle::from_index(0));

        // The local Z (world Y) should be scaled by 4.0: Y = 0.5 * 4.0 = 2.0
        assert_relative_eq!(pos.y, 2.0, epsilon = 0.1);

        // The local XY (world X offset) should be scaled by 1/sqrt(4)=0.5: X = 2.0 * 0.5 = 1.0
        assert_relative_eq!(pos.x, 1.0, epsilon = 0.1);
    }
}
