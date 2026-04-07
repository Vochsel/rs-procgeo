use serde::{Deserialize, Serialize};

use glam::{Mat3, Mat4, Quat, Vec3};
use procgeo_core::{AttribClass, AttribDefault, Geometry, PointHandle, PrimHandle, TypeQualifier};

use crate::{Sop, SopError};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CopyToPointsParams {
    /// Name of an integer prim attribute on the source geometry that selects
    /// which piece of source geometry to copy to each target point.
    /// The target points need a matching integer point attribute with the same name.
    /// If empty, all source geometry is copied to every point.
    pub piece_attrib: String,
}

pub struct CopyToPointsSop;

// ---------------------------------------------------------------------------
// Instancing attribute lookup helpers
// ---------------------------------------------------------------------------

/// Resolved per-copy transform built from Houdini instancing attributes.
struct CopyTransform {
    matrix: Mat4,
}

/// Try to find and read a float point attribute value.
fn read_f32(geo: &Geometry, name: &str, pt: usize) -> Option<f32> {
    geo.find_attrib::<f32>(AttribClass::Point, name)
        .ok()
        .and_then(|h| geo.get_attrib(&h, pt).ok())
}

/// Try to find and read a vec3 point attribute value.
fn read_vec3(geo: &Geometry, name: &str, pt: usize) -> Option<Vec3> {
    geo.find_attrib::<[f32; 3]>(AttribClass::Point, name)
        .ok()
        .and_then(|h| geo.get_attrib(&h, pt).ok())
        .map(Vec3::from)
}

/// Try to find and read a quaternion (vec4) point attribute value.
fn read_quat(geo: &Geometry, name: &str, pt: usize) -> Option<Quat> {
    geo.find_attrib::<[f32; 4]>(AttribClass::Point, name)
        .ok()
        .and_then(|h| geo.get_attrib(&h, pt).ok())
        .map(|a| Quat::from_xyzw(a[0], a[1], a[2], a[3]))
}

/// Try to find and read a 3x3 matrix point attribute.
fn read_mat3(geo: &Geometry, name: &str, pt: usize) -> Option<Mat3> {
    geo.find_attrib::<[f32; 9]>(AttribClass::Point, name)
        .ok()
        .and_then(|h| geo.get_attrib(&h, pt).ok())
        .map(|a| {
            Mat3::from_cols(
                Vec3::new(a[0], a[1], a[2]),
                Vec3::new(a[3], a[4], a[5]),
                Vec3::new(a[6], a[7], a[8]),
            )
        })
}

/// Build a rotation matrix from N and up vectors (Houdini's look-at convention).
/// N is the +Z axis direction, up is the +Y hint.
fn orient_from_n_up(n: Vec3, up: Vec3) -> Mat3 {
    let z = n.normalize_or_zero();
    let x = up.cross(z).normalize_or_zero();
    let y = z.cross(x);
    Mat3::from_cols(x, y, z)
}

/// Compute the per-copy transform for a target point following Houdini's
/// instancing attribute priority:
///
/// 1. `transform` (3x3 matrix) — overrides orientation
/// 2. `orient` (quaternion) — primary orientation
/// 3. `N` + `up` — look-at orientation
/// 4. `v` — fallback direction (used as N)
/// 5. `rot` — additional rotation applied *after* the above
///
/// Scale: `pscale` (uniform) and `scale` (non-uniform) are combined.
/// Translation: `P` (point position) + `trans` (additional offset).
/// Pivot: `pivot` shifts the local origin before transform.
fn compute_copy_transform(target: &Geometry, pt_idx: usize) -> CopyTransform {
    let pos = target.point_pos(PointHandle::from_index(pt_idx));

    // Translation: P + trans
    let trans = read_vec3(target, "trans", pt_idx).unwrap_or(Vec3::ZERO);
    let translation = pos + trans;

    // Pivot
    let pivot = read_vec3(target, "pivot", pt_idx).unwrap_or(Vec3::ZERO);

    // Scale: pscale (uniform) * scale (non-uniform)
    let pscale = read_f32(target, "pscale", pt_idx).unwrap_or(1.0);
    let scale = read_vec3(target, "scale", pt_idx).unwrap_or(Vec3::ONE);
    let final_scale = scale * pscale;

    // Orientation (priority order)
    let orientation: Mat3;

    if let Some(xform) = read_mat3(target, "transform", pt_idx) {
        // Priority 1: explicit transform matrix overrides orientation
        // The transform matrix already encodes rotation + scale, so we use it
        // directly and don't apply separate scale.
        let t = Mat4::from_translation(translation)
            * Mat4::from_translation(pivot)
            * Mat4::from_mat3(xform)
            * Mat4::from_translation(-pivot);
        return CopyTransform { matrix: t };
    } else if let Some(orient) = read_quat(target, "orient", pt_idx) {
        // Priority 2: orient quaternion
        orientation = Mat3::from_quat(orient);
    } else if let Some(n) = read_vec3(target, "N", pt_idx) {
        // Priority 3: N + up vectors
        let up = read_vec3(target, "up", pt_idx).unwrap_or(Vec3::Y);
        orientation = orient_from_n_up(n, up);
    } else if let Some(v) = read_vec3(target, "v", pt_idx) {
        // Priority 4: velocity as direction
        let up = read_vec3(target, "up", pt_idx).unwrap_or(Vec3::Y);
        orientation = orient_from_n_up(v, up);
    } else {
        orientation = Mat3::IDENTITY;
    }

    // Additional rotation (applied after orientation)
    let rot = read_quat(target, "rot", pt_idx);
    let final_orient = if let Some(r) = rot {
        Mat3::from_quat(r) * orientation
    } else {
        orientation
    };

    // Build final matrix: T * pivot * orient * scale * -pivot
    let scale_mat = Mat4::from_scale(final_scale);
    let orient_mat = Mat4::from_mat3(final_orient);
    let matrix = Mat4::from_translation(translation)
        * Mat4::from_translation(pivot)
        * orient_mat
        * scale_mat
        * Mat4::from_translation(-pivot);

    CopyTransform { matrix }
}

impl Sop for CopyToPointsSop {
    type Params = CopyToPointsParams;

    fn name(&self) -> &'static str {
        "copy_to_points"
    }

    fn input_count(&self) -> (usize, usize) {
        (2, 2)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;

        let source = inputs[0];
        let target = inputs[1];

        let tgt_npts = target.num_points();

        // Piece attribute support: if set, read an integer attribute from both
        // source prims and target points to match which piece goes where.
        let piece_map: Option<(Vec<i32>, Vec<i32>)> = if !params.piece_attrib.is_empty() {
            let src_handle = source
                .find_attrib::<i32>(AttribClass::Primitive, &params.piece_attrib)
                .map_err(|_| {
                    SopError::InvalidParam(format!(
                        "piece attribute '{}' not found on source primitives",
                        params.piece_attrib
                    ))
                })?;
            let tgt_handle = target
                .find_attrib::<i32>(AttribClass::Point, &params.piece_attrib)
                .map_err(|_| {
                    SopError::InvalidParam(format!(
                        "piece attribute '{}' not found on target points",
                        params.piece_attrib
                    ))
                })?;

            let src_pieces: Vec<i32> = (0..source.num_prims())
                .map(|i| source.get_attrib(&src_handle, i).unwrap_or(0))
                .collect();
            let tgt_pieces: Vec<i32> = (0..tgt_npts)
                .map(|i| target.get_attrib(&tgt_handle, i).unwrap_or(0))
                .collect();
            Some((src_pieces, tgt_pieces))
        } else {
            None
        };

        // Pre-collect source point positions and topology
        let src_npts = source.num_points();
        let src_positions: Vec<Vec3> = (0..src_npts)
            .map(|i| source.point_pos(PointHandle::from_index(i)))
            .collect();

        struct PrimInfo {
            points: Vec<usize>, // source point indices
            is_closed: bool,
        }
        let src_prims: Vec<PrimInfo> = (0..source.num_prims())
            .map(|i| {
                let ph = PrimHandle::from_index(i);
                let pts = source.prim_points(ph);
                let prim = source.prim(ph);
                let is_closed = match prim {
                    procgeo_core::Primitive::Polygon(poly) => {
                        poly.poly_type == procgeo_core::PolyType::Closed
                    }
                };
                PrimInfo {
                    points: pts.iter().map(|p| p.index()).collect(),
                    is_closed,
                }
            })
            .collect();

        // Estimate capacity
        let est_copies = tgt_npts; // upper bound
        let mut out = Geometry::with_capacity(src_npts * est_copies, src_prims.len() * est_copies);

        // Add copynum attribute
        out.add_attrib(
            AttribClass::Point,
            "copynum",
            AttribDefault::Int(0),
            TypeQualifier::None,
        )?;
        let copynum_handle = out.find_attrib::<i32>(AttribClass::Point, "copynum")?;

        for copy_idx in 0..tgt_npts {
            let xform = compute_copy_transform(target, copy_idx);
            let point_offset = out.num_points();

            // Determine which piece to use (if piece_attrib is set)
            let tgt_piece = piece_map
                .as_ref()
                .map(|(_, tgt_pieces)| tgt_pieces[copy_idx]);

            // Add transformed source points
            for (src_pt_idx, src_pos) in src_positions.iter().enumerate() {
                let world_pos = xform.matrix.transform_point3(*src_pos);
                let new_pt = out.add_point(world_pos);
                out.set_attrib(&copynum_handle, new_pt.index(), copy_idx as i32)?;
                let _ = src_pt_idx; // used for indexing only
            }

            // Add prims (filtered by piece if applicable)
            for (prim_idx, prim_info) in src_prims.iter().enumerate() {
                // Skip prims not matching the target piece
                if let Some((ref src_pieces, _)) = piece_map {
                    if let Some(tp) = tgt_piece {
                        if src_pieces[prim_idx] != tp {
                            continue;
                        }
                    }
                }

                let new_pts: Vec<PointHandle> = prim_info
                    .points
                    .iter()
                    .map(|&src_idx| PointHandle::from_index(src_idx + point_offset))
                    .collect();

                if prim_info.is_closed {
                    out.add_face(&new_pts);
                } else {
                    out.add_polyline(&new_pts);
                }
            }
        }

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creation::box_sop::{BoxSop, BoxParams};
    use crate::creation::line::{LineSop, LineParams};
    use crate::generate;
    use approx::assert_relative_eq;
    use glam::Vec3;

    fn make_box() -> Geometry {
        generate(&BoxSop, &BoxParams::default()).unwrap()
    }

    fn make_line(pts: u32) -> Geometry {
        generate(
            &LineSop,
            &LineParams {
                points: pts,
                origin: Vec3::ZERO,
                direction: Vec3::X,
                length: 4.0,
            },
        )
        .unwrap()
    }

    #[test]
    fn copy_box_to_line() {
        // Box: 8 pts, 6 prims. Line with 5 pts, 1 prim.
        // Copy box to line points → 8*5=40 pts, 6*5=30 prims
        let sop = CopyToPointsSop;
        let bx = make_box();
        let ln = make_line(5);
        let result = sop.execute(&[&bx, &ln], &CopyToPointsParams::default()).unwrap();

        assert_eq!(result.num_points(), 40, "expected 40 points (8*5)");
        assert_eq!(result.num_prims(), 30, "expected 30 prims (6*5)");
    }

    #[test]
    fn copy_preserves_topology() {
        let sop = CopyToPointsSop;
        let bx = make_box();
        let ln = make_line(3);
        let result = sop.execute(&[&bx, &ln], &CopyToPointsParams::default()).unwrap();

        // Each face should have 4 vertices (box faces are quads)
        for prim_idx in 0..result.num_prims() {
            let ph = PrimHandle::from_index(prim_idx);
            assert_eq!(result.prim_vertices(ph).len(), 4, "expected quad at prim {prim_idx}");
        }
    }

    #[test]
    fn copy_has_copynum() {
        let sop = CopyToPointsSop;
        let bx = make_box();
        let ln = make_line(3);
        let result = sop.execute(&[&bx, &ln], &CopyToPointsParams::default()).unwrap();

        let handle = result.find_attrib::<i32>(AttribClass::Point, "copynum").unwrap();

        // First 8 points should have copynum=0, next 8 → 1, next 8 → 2
        for copy_idx in 0..3_usize {
            for local_pt in 0..8_usize {
                let global_idx = copy_idx * 8 + local_pt;
                let val = result.get_attrib(&handle, global_idx).unwrap();
                assert_eq!(val, copy_idx as i32, "point {global_idx} should have copynum={copy_idx}");
            }
        }
    }

    #[test]
    fn copy_respects_pscale() {
        // Create target with pscale=2 on one point
        let mut target = Geometry::new();
        target.add_point(Vec3::ZERO);
        target
            .add_attrib(
                AttribClass::Point,
                "pscale",
                AttribDefault::Float(1.0),
                TypeQualifier::None,
            )
            .unwrap();
        let h = target.find_attrib::<f32>(AttribClass::Point, "pscale").unwrap();
        target.set_attrib(&h, 0, 2.0).unwrap();

        let bx = make_box(); // default box is 1x1x1, spans [-0.5, 0.5]

        let sop = CopyToPointsSop;
        let result = sop.execute(&[&bx, &target], &CopyToPointsParams::default()).unwrap();

        let bb = result.bounding_box();
        // pscale=2 should double the box: [-1, 1] on all axes
        assert_relative_eq!(bb.min.x, -1.0, epsilon = 1e-4);
        assert_relative_eq!(bb.max.x, 1.0, epsilon = 1e-4);
        assert_relative_eq!(bb.min.y, -1.0, epsilon = 1e-4);
        assert_relative_eq!(bb.max.y, 1.0, epsilon = 1e-4);
    }

    #[test]
    fn copy_respects_scale_nonuniform() {
        let mut target = Geometry::new();
        target.add_point(Vec3::ZERO);
        target
            .add_attrib(
                AttribClass::Point,
                "scale",
                AttribDefault::Vector3([1.0, 1.0, 1.0]),
                TypeQualifier::None,
            )
            .unwrap();
        let h = target
            .find_attrib::<[f32; 3]>(AttribClass::Point, "scale")
            .unwrap();
        target.set_attrib(&h, 0, [2.0, 1.0, 3.0]).unwrap();

        let bx = make_box();
        let sop = CopyToPointsSop;
        let result = sop.execute(&[&bx, &target], &CopyToPointsParams::default()).unwrap();

        let bb = result.bounding_box();
        // X scaled by 2: [-1, 1], Y unchanged: [-0.5, 0.5], Z scaled by 3: [-1.5, 1.5]
        assert_relative_eq!(bb.min.x, -1.0, epsilon = 1e-4);
        assert_relative_eq!(bb.max.x, 1.0, epsilon = 1e-4);
        assert_relative_eq!(bb.min.y, -0.5, epsilon = 1e-4);
        assert_relative_eq!(bb.max.y, 0.5, epsilon = 1e-4);
        assert_relative_eq!(bb.min.z, -1.5, epsilon = 1e-4);
        assert_relative_eq!(bb.max.z, 1.5, epsilon = 1e-4);
    }

    #[test]
    fn copy_respects_orient() {
        // Rotate box 90 degrees around Y axis via quaternion
        let mut target = Geometry::new();
        target.add_point(Vec3::ZERO);
        target
            .add_attrib(
                AttribClass::Point,
                "orient",
                AttribDefault::Vector4([0.0, 0.0, 0.0, 1.0]),
                TypeQualifier::Quaternion,
            )
            .unwrap();
        let h = target
            .find_attrib::<[f32; 4]>(AttribClass::Point, "orient")
            .unwrap();
        // 90 degrees around Y: quat = (0, sin(45°), 0, cos(45°))
        let q = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
        target.set_attrib(&h, 0, [q.x, q.y, q.z, q.w]).unwrap();

        let bx = make_box();
        let sop = CopyToPointsSop;
        let result = sop.execute(&[&bx, &target], &CopyToPointsParams::default()).unwrap();

        let bb = result.bounding_box();
        // After 90° Y rotation, X and Z extents should swap
        // Original box: X [-0.5, 0.5], Z [-0.5, 0.5] — same size, so just verify centered
        assert_relative_eq!(bb.min.x, -0.5, epsilon = 1e-4);
        assert_relative_eq!(bb.max.x, 0.5, epsilon = 1e-4);
        assert_relative_eq!(bb.min.y, -0.5, epsilon = 1e-4);
        assert_relative_eq!(bb.max.y, 0.5, epsilon = 1e-4);
    }

    #[test]
    fn copy_respects_n_up() {
        // Set N pointing in +X direction (rotates Z->X)
        let mut target = Geometry::new();
        target.add_point(Vec3::ZERO);
        target
            .add_attrib(
                AttribClass::Point,
                "N",
                AttribDefault::Vector3([0.0, 0.0, 1.0]),
                TypeQualifier::Normal,
            )
            .unwrap();
        let h = target
            .find_attrib::<[f32; 3]>(AttribClass::Point, "N")
            .unwrap();
        target.set_attrib(&h, 0, [1.0, 0.0, 0.0]).unwrap();

        let bx = make_box();
        let sop = CopyToPointsSop;
        let result = sop.execute(&[&bx, &target], &CopyToPointsParams::default()).unwrap();

        // Should still produce valid geometry
        assert_eq!(result.num_points(), 8);
        assert_eq!(result.num_prims(), 6);
        // Bounding box should still be 1x1x1 (just rotated)
        let bb = result.bounding_box();
        let size = bb.max - bb.min;
        assert_relative_eq!(size.x, 1.0, epsilon = 0.05);
        assert_relative_eq!(size.y, 1.0, epsilon = 0.05);
        assert_relative_eq!(size.z, 1.0, epsilon = 0.05);
    }

    #[test]
    fn copy_pscale_and_orient_combined() {
        // Scale by 3 and rotate 90° around Y
        let mut target = Geometry::new();
        target.add_point(Vec3::new(10.0, 0.0, 0.0));
        target
            .add_attrib(
                AttribClass::Point,
                "pscale",
                AttribDefault::Float(1.0),
                TypeQualifier::None,
            )
            .unwrap();
        target
            .add_attrib(
                AttribClass::Point,
                "orient",
                AttribDefault::Vector4([0.0, 0.0, 0.0, 1.0]),
                TypeQualifier::Quaternion,
            )
            .unwrap();
        let ps = target.find_attrib::<f32>(AttribClass::Point, "pscale").unwrap();
        target.set_attrib(&ps, 0, 3.0).unwrap();
        let oh = target
            .find_attrib::<[f32; 4]>(AttribClass::Point, "orient")
            .unwrap();
        let q = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
        target.set_attrib(&oh, 0, [q.x, q.y, q.z, q.w]).unwrap();

        let bx = make_box();
        let sop = CopyToPointsSop;
        let result = sop.execute(&[&bx, &target], &CopyToPointsParams::default()).unwrap();

        let bb = result.bounding_box();
        // Box scaled by 3 → 3x3x3 centered at (10, 0, 0)
        assert_relative_eq!((bb.min.x + bb.max.x) * 0.5, 10.0, epsilon = 0.1);
        let size = bb.max - bb.min;
        assert_relative_eq!(size.x, 3.0, epsilon = 0.1);
        assert_relative_eq!(size.y, 3.0, epsilon = 0.1);
        assert_relative_eq!(size.z, 3.0, epsilon = 0.1);
    }

    #[test]
    fn copy_respects_trans() {
        // trans adds an additional offset on top of P
        let mut target = Geometry::new();
        target.add_point(Vec3::new(1.0, 0.0, 0.0));
        target
            .add_attrib(
                AttribClass::Point,
                "trans",
                AttribDefault::Vector3([0.0, 0.0, 0.0]),
                TypeQualifier::None,
            )
            .unwrap();
        let h = target
            .find_attrib::<[f32; 3]>(AttribClass::Point, "trans")
            .unwrap();
        target.set_attrib(&h, 0, [0.0, 5.0, 0.0]).unwrap();

        let bx = make_box();
        let sop = CopyToPointsSop;
        let result = sop
            .execute(&[&bx, &target], &CopyToPointsParams::default())
            .unwrap();

        let bb = result.bounding_box();
        // P=(1,0,0) + trans=(0,5,0) → center at (1,5,0)
        assert_relative_eq!((bb.min.x + bb.max.x) * 0.5, 1.0, epsilon = 1e-4);
        assert_relative_eq!((bb.min.y + bb.max.y) * 0.5, 5.0, epsilon = 1e-4);
        assert_relative_eq!((bb.min.z + bb.max.z) * 0.5, 0.0, epsilon = 1e-4);
    }

    #[test]
    fn copy_respects_velocity_as_direction() {
        // v attribute is used as a fallback orientation (like N) when orient and N are absent
        let mut target = Geometry::new();
        target.add_point(Vec3::ZERO);
        target
            .add_attrib(
                AttribClass::Point,
                "v",
                AttribDefault::Vector3([0.0, 0.0, 0.0]),
                TypeQualifier::Vector,
            )
            .unwrap();
        let h = target
            .find_attrib::<[f32; 3]>(AttribClass::Point, "v")
            .unwrap();
        // Point velocity in +X direction — should orient Z axis to +X
        target.set_attrib(&h, 0, [1.0, 0.0, 0.0]).unwrap();

        let bx = make_box();
        let sop = CopyToPointsSop;
        let result = sop
            .execute(&[&bx, &target], &CopyToPointsParams::default())
            .unwrap();

        // Should produce valid rotated geometry
        assert_eq!(result.num_points(), 8);
        assert_eq!(result.num_prims(), 6);
        let bb = result.bounding_box();
        let size = bb.max - bb.min;
        // Rotated box still has 1x1x1 bounding volume
        assert_relative_eq!(size.x, 1.0, epsilon = 0.05);
        assert_relative_eq!(size.y, 1.0, epsilon = 0.05);
        assert_relative_eq!(size.z, 1.0, epsilon = 0.05);
    }

    #[test]
    fn copy_respects_rot_additional() {
        // rot is applied AFTER orient as an additional rotation
        let mut target = Geometry::new();
        target.add_point(Vec3::ZERO);

        // Set identity orient
        target
            .add_attrib(
                AttribClass::Point,
                "orient",
                AttribDefault::Vector4([0.0, 0.0, 0.0, 1.0]),
                TypeQualifier::Quaternion,
            )
            .unwrap();

        // Set rot to 90° around Y
        target
            .add_attrib(
                AttribClass::Point,
                "rot",
                AttribDefault::Vector4([0.0, 0.0, 0.0, 1.0]),
                TypeQualifier::Quaternion,
            )
            .unwrap();
        let rh = target
            .find_attrib::<[f32; 4]>(AttribClass::Point, "rot")
            .unwrap();
        let q = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
        target.set_attrib(&rh, 0, [q.x, q.y, q.z, q.w]).unwrap();

        // Use a non-uniform box so rotation is visible in the bbox
        let source = generate(
            &BoxSop,
            &BoxParams {
                size: Vec3::new(2.0, 1.0, 0.5),
                ..Default::default()
            },
        )
        .unwrap();

        let sop = CopyToPointsSop;
        let result = sop
            .execute(&[&source, &target], &CopyToPointsParams::default())
            .unwrap();

        let bb = result.bounding_box();
        // Original: X=[-1,1], Y=[-0.5,0.5], Z=[-0.25,0.25]
        // After 90° Y rotation: X and Z swap → X=[-0.25,0.25], Z=[-1,1]
        assert_relative_eq!(bb.max.x - bb.min.x, 0.5, epsilon = 0.05); // was 2.0, now 0.5
        assert_relative_eq!(bb.max.z - bb.min.z, 2.0, epsilon = 0.05); // was 0.5, now 2.0
        assert_relative_eq!(bb.max.y - bb.min.y, 1.0, epsilon = 0.05); // Y unchanged
    }

    #[test]
    fn copy_respects_transform_matrix() {
        // transform (3x3 matrix) overrides all orientation and includes scale
        let mut target = Geometry::new();
        target.add_point(Vec3::ZERO);
        target
            .add_attrib(
                AttribClass::Point,
                "transform",
                AttribDefault::Matrix3([1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]),
                TypeQualifier::Matrix,
            )
            .unwrap();
        let h = target
            .find_attrib::<[f32; 9]>(AttribClass::Point, "transform")
            .unwrap();
        // Scale X by 4, Y by 1, Z by 1 via matrix columns
        #[rustfmt::skip]
        target.set_attrib(&h, 0, [
            4.0, 0.0, 0.0,  // col 0 (X axis)
            0.0, 1.0, 0.0,  // col 1 (Y axis)
            0.0, 0.0, 1.0,  // col 2 (Z axis)
        ]).unwrap();

        let bx = make_box();
        let sop = CopyToPointsSop;
        let result = sop
            .execute(&[&bx, &target], &CopyToPointsParams::default())
            .unwrap();

        let bb = result.bounding_box();
        // X scaled by 4: [-2, 2], Y and Z unchanged: [-0.5, 0.5]
        assert_relative_eq!(bb.min.x, -2.0, epsilon = 1e-4);
        assert_relative_eq!(bb.max.x, 2.0, epsilon = 1e-4);
        assert_relative_eq!(bb.min.y, -0.5, epsilon = 1e-4);
        assert_relative_eq!(bb.max.y, 0.5, epsilon = 1e-4);
    }

    #[test]
    fn copy_transform_overrides_orient() {
        // When both transform and orient are present, transform wins
        let mut target = Geometry::new();
        target.add_point(Vec3::ZERO);

        // orient: 90° around Y (would swap X and Z)
        target
            .add_attrib(
                AttribClass::Point,
                "orient",
                AttribDefault::Vector4([0.0, 0.0, 0.0, 1.0]),
                TypeQualifier::Quaternion,
            )
            .unwrap();
        let oh = target
            .find_attrib::<[f32; 4]>(AttribClass::Point, "orient")
            .unwrap();
        let q = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
        target.set_attrib(&oh, 0, [q.x, q.y, q.z, q.w]).unwrap();

        // transform: identity (should override the orient rotation)
        target
            .add_attrib(
                AttribClass::Point,
                "transform",
                AttribDefault::Matrix3([1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]),
                TypeQualifier::Matrix,
            )
            .unwrap();

        // Use non-uniform box to detect rotation
        let source = generate(
            &BoxSop,
            &BoxParams {
                size: Vec3::new(4.0, 1.0, 1.0),
                ..Default::default()
            },
        )
        .unwrap();

        let sop = CopyToPointsSop;
        let result = sop
            .execute(&[&source, &target], &CopyToPointsParams::default())
            .unwrap();

        let bb = result.bounding_box();
        // transform=identity wins: X should still be the long axis
        assert_relative_eq!(bb.max.x - bb.min.x, 4.0, epsilon = 1e-4);
        assert_relative_eq!(bb.max.z - bb.min.z, 1.0, epsilon = 1e-4);
    }

    #[test]
    fn copy_orient_overrides_n() {
        // When both orient and N are present, orient wins
        let mut target = Geometry::new();
        target.add_point(Vec3::ZERO);

        // N pointing in +X (would rotate Z→X)
        target
            .add_attrib(
                AttribClass::Point,
                "N",
                AttribDefault::Vector3([0.0, 0.0, 1.0]),
                TypeQualifier::Normal,
            )
            .unwrap();
        let nh = target
            .find_attrib::<[f32; 3]>(AttribClass::Point, "N")
            .unwrap();
        target.set_attrib(&nh, 0, [1.0, 0.0, 0.0]).unwrap();

        // orient: identity quaternion (should override N)
        target
            .add_attrib(
                AttribClass::Point,
                "orient",
                AttribDefault::Vector4([0.0, 0.0, 0.0, 1.0]),
                TypeQualifier::Quaternion,
            )
            .unwrap();

        // Non-uniform box: long in Z
        let source = generate(
            &BoxSop,
            &BoxParams {
                size: Vec3::new(1.0, 1.0, 4.0),
                ..Default::default()
            },
        )
        .unwrap();

        let sop = CopyToPointsSop;
        let result = sop
            .execute(&[&source, &target], &CopyToPointsParams::default())
            .unwrap();

        let bb = result.bounding_box();
        // orient=identity wins: Z should still be the long axis (not rotated to X by N)
        assert_relative_eq!(bb.max.z - bb.min.z, 4.0, epsilon = 1e-4);
        assert_relative_eq!(bb.max.x - bb.min.x, 1.0, epsilon = 1e-4);
    }

    #[test]
    fn copy_respects_pivot() {
        // pivot shifts the local origin before transform
        let mut target = Geometry::new();
        target.add_point(Vec3::ZERO);
        target
            .add_attrib(
                AttribClass::Point,
                "pscale",
                AttribDefault::Float(1.0),
                TypeQualifier::None,
            )
            .unwrap();
        let ps = target
            .find_attrib::<f32>(AttribClass::Point, "pscale")
            .unwrap();
        target.set_attrib(&ps, 0, 2.0).unwrap();

        target
            .add_attrib(
                AttribClass::Point,
                "pivot",
                AttribDefault::Vector3([0.0, 0.0, 0.0]),
                TypeQualifier::Point,
            )
            .unwrap();
        let ph = target
            .find_attrib::<[f32; 3]>(AttribClass::Point, "pivot")
            .unwrap();
        // Pivot at the bottom of the box — scaling should anchor there
        target.set_attrib(&ph, 0, [0.0, -0.5, 0.0]).unwrap();

        let bx = make_box(); // [-0.5, 0.5] on all axes
        let sop = CopyToPointsSop;
        let result = sop
            .execute(&[&bx, &target], &CopyToPointsParams::default())
            .unwrap();

        let bb = result.bounding_box();
        // With pivot at bottom (-0.5 Y), scale 2x:
        // Y range: pivot + scale*(orig - pivot) = -0.5 + 2*(-0.5 - -0.5) to -0.5 + 2*(0.5 - -0.5)
        //        = -0.5 + 0 to -0.5 + 2 = [-0.5, 1.5]
        assert_relative_eq!(bb.min.y, -0.5, epsilon = 1e-4);
        assert_relative_eq!(bb.max.y, 1.5, epsilon = 1e-4);
    }

    #[test]
    fn copy_pscale_multiplies_scale() {
        // pscale and scale should combine multiplicatively
        let mut target = Geometry::new();
        target.add_point(Vec3::ZERO);
        target
            .add_attrib(
                AttribClass::Point,
                "pscale",
                AttribDefault::Float(1.0),
                TypeQualifier::None,
            )
            .unwrap();
        target
            .add_attrib(
                AttribClass::Point,
                "scale",
                AttribDefault::Vector3([1.0, 1.0, 1.0]),
                TypeQualifier::None,
            )
            .unwrap();
        let ps = target
            .find_attrib::<f32>(AttribClass::Point, "pscale")
            .unwrap();
        let sc = target
            .find_attrib::<[f32; 3]>(AttribClass::Point, "scale")
            .unwrap();
        target.set_attrib(&ps, 0, 2.0).unwrap();
        target.set_attrib(&sc, 0, [3.0, 1.0, 1.0]).unwrap();

        let bx = make_box(); // [-0.5, 0.5]
        let sop = CopyToPointsSop;
        let result = sop
            .execute(&[&bx, &target], &CopyToPointsParams::default())
            .unwrap();

        let bb = result.bounding_box();
        // X: pscale(2) * scale.x(3) = 6 → [-3, 3]
        // Y: pscale(2) * scale.y(1) = 2 → [-1, 1]
        // Z: pscale(2) * scale.z(1) = 2 → [-1, 1]
        assert_relative_eq!(bb.min.x, -3.0, epsilon = 1e-4);
        assert_relative_eq!(bb.max.x, 3.0, epsilon = 1e-4);
        assert_relative_eq!(bb.min.y, -1.0, epsilon = 1e-4);
        assert_relative_eq!(bb.max.y, 1.0, epsilon = 1e-4);
    }

    #[test]
    fn copy_multiple_targets_varying_attribs() {
        // Three target points with different pscale values
        let mut target = Geometry::new();
        target.add_point(Vec3::new(0.0, 0.0, 0.0));
        target.add_point(Vec3::new(5.0, 0.0, 0.0));
        target.add_point(Vec3::new(10.0, 0.0, 0.0));
        target
            .add_attrib(
                AttribClass::Point,
                "pscale",
                AttribDefault::Float(1.0),
                TypeQualifier::None,
            )
            .unwrap();
        let ps = target
            .find_attrib::<f32>(AttribClass::Point, "pscale")
            .unwrap();
        target.set_attrib(&ps, 0, 1.0).unwrap(); // normal
        target.set_attrib(&ps, 1, 2.0).unwrap(); // double
        target.set_attrib(&ps, 2, 0.5).unwrap(); // half

        let bx = make_box();
        let sop = CopyToPointsSop;
        let result = sop
            .execute(&[&bx, &target], &CopyToPointsParams::default())
            .unwrap();

        assert_eq!(result.num_points(), 24); // 8 * 3
        assert_eq!(result.num_prims(), 18); // 6 * 3

        let bb = result.bounding_box();
        // Leftmost copy at x=0 with pscale=1: X in [-0.5, 0.5]
        // Middle copy at x=5 with pscale=2: X in [4.0, 6.0]
        // Rightmost copy at x=10 with pscale=0.5: X in [9.75, 10.25]
        // Overall: min.x = -0.5, max.x = 10.25
        assert_relative_eq!(bb.min.x, -0.5, epsilon = 1e-4);
        assert_relative_eq!(bb.max.x, 10.25, epsilon = 1e-4);
        // Y: largest pscale is 2 → [-1, 1]
        assert_relative_eq!(bb.min.y, -1.0, epsilon = 1e-4);
        assert_relative_eq!(bb.max.y, 1.0, epsilon = 1e-4);
    }

    #[test]
    fn copy_no_instancing_attribs_uses_position_only() {
        // With no instancing attributes, copies are just offset by P
        let mut target = Geometry::new();
        target.add_point(Vec3::new(3.0, 4.0, 5.0));

        let bx = make_box();
        let sop = CopyToPointsSop;
        let result = sop
            .execute(&[&bx, &target], &CopyToPointsParams::default())
            .unwrap();

        let bb = result.bounding_box();
        // Box centered at (3, 4, 5) with size 1
        assert_relative_eq!((bb.min.x + bb.max.x) * 0.5, 3.0, epsilon = 1e-4);
        assert_relative_eq!((bb.min.y + bb.max.y) * 0.5, 4.0, epsilon = 1e-4);
        assert_relative_eq!((bb.min.z + bb.max.z) * 0.5, 5.0, epsilon = 1e-4);
        assert_relative_eq!(bb.max.x - bb.min.x, 1.0, epsilon = 1e-4);
    }

    #[test]
    fn copy_piece_attrib() {
        // Source: two boxes at different positions, with "piece" prim attrib
        let box_a = generate(&BoxSop, &BoxParams {
            center: Vec3::ZERO,
            ..Default::default()
        }).unwrap();
        let box_b = generate(&BoxSop, &BoxParams {
            center: Vec3::new(100.0, 0.0, 0.0),
            ..Default::default()
        }).unwrap();

        // Merge them and add "piece" attribute
        let merged = crate::merge::MergeSop
            .execute(&[&box_a, &box_b], &crate::merge::MergeParams)
            .unwrap();
        let mut source = merged;
        source
            .add_attrib(
                AttribClass::Primitive,
                "piece",
                AttribDefault::Int(0),
                TypeQualifier::None,
            )
            .unwrap();
        let sh = source
            .find_attrib::<i32>(AttribClass::Primitive, "piece")
            .unwrap();
        // First 6 prims = piece 0, next 6 = piece 1
        for i in 0..6 {
            source.set_attrib(&sh, i, 0).unwrap();
        }
        for i in 6..12 {
            source.set_attrib(&sh, i, 1).unwrap();
        }

        // Target: 2 points with "piece" attribute
        let mut target = Geometry::new();
        target.add_point(Vec3::new(0.0, 0.0, 0.0));
        target.add_point(Vec3::new(5.0, 0.0, 0.0));
        target
            .add_attrib(
                AttribClass::Point,
                "piece",
                AttribDefault::Int(0),
                TypeQualifier::None,
            )
            .unwrap();
        let th = target
            .find_attrib::<i32>(AttribClass::Point, "piece")
            .unwrap();
        target.set_attrib(&th, 0, 0).unwrap(); // first point gets piece 0
        target.set_attrib(&th, 1, 1).unwrap(); // second point gets piece 1

        let sop = CopyToPointsSop;
        let result = sop
            .execute(
                &[&source, &target],
                &CopyToPointsParams {
                    piece_attrib: "piece".to_string(),
                },
            )
            .unwrap();

        // Each target point copies all source points (16 each) but only 6 matching prims
        assert_eq!(result.num_points(), 32); // 16 src points * 2 targets
        assert_eq!(result.num_prims(), 12); // 6 prims per target (only matching piece)
    }
}
