use glam::Vec3;
use serde::{Deserialize, Serialize};

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

/// Bend SOP — deforms geometry by bending, twisting, tapering, and length-scaling
/// along a capture axis.  Full deformation math is implemented in Task 2; this
/// stub wires up the trait and passes the geometry through unchanged.
pub struct BendSop;

impl Sop for BendSop {
    type Params = BendParams;

    fn name(&self) -> &'static str {
        "bend"
    }

    /// 1 required input (the geometry to deform); an optional second input can
    /// supply a capture region override.
    fn input_count(&self) -> (usize, usize) {
        (1, 2)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;

        if !params.enable_deformation {
            return Ok(inputs[0].clone());
        }

        // Stub: return geometry unchanged until Task 2 implements the math.
        Ok(inputs[0].clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creation::box_sop::{BoxParams, BoxSop};
    use crate::{GeometryExt, generate};

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
}
