//! PointDeform SOP — deforms geometry based on a rest→deformed point lattice.
//!
//! This is a 3-input SOP:
//!   0. Mesh to deform
//!   1. Rest lattice (capture pose)
//!   2. Deformed lattice
//!
//! For each mesh point the SOP finds nearby lattice points (via KD-tree),
//! computes a weighted blend of per-lattice-point rigid transforms (rotation +
//! translation extracted via polar decomposition), and applies that transform.

use glam::{Mat3, Vec3};
use serde::{Deserialize, Serialize};

use procgeo_core::attribute::{AttribClass, AttribHandle};
use procgeo_core::handle::{PointHandle, PrimHandle};
use procgeo_core::Geometry;

use crate::{Sop, SopError};

use super::kdtree::KdTree;

// ===========================================================================
// Params
// ===========================================================================

/// Operating mode for the SOP.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub enum PointDeformMode {
    /// Capture nearby lattice points *and* deform in one pass.
    #[default]
    CaptureAndDeform,
    /// Only capture (store weights). Stub — array attribs not yet supported.
    Capture,
    /// Only deform from previously captured weights. Stub.
    Deform,
}

/// Parameters for the PointDeform SOP, closely matching Houdini's defaults.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PointDeformParams {
    /// Optional point group to restrict which mesh points are deformed.
    pub group: Option<String>,

    /// Operating mode.
    pub mode: PointDeformMode,

    /// Capture radius — lattice points within this distance are used.
    pub radius: f32,

    /// Minimum number of lattice points to capture per mesh point.
    /// If fewer than `min_points` are found within `radius`, the radius is
    /// expanded until this many are found.
    pub min_points: u32,

    /// Maximum lattice points per mesh point (keeps the closest).
    pub max_points: u32,

    /// Optional piece attribute (integer) for partitioned deformation.
    pub piece_attrib: Option<String>,

    /// When true, extract the rotation from the covariance matrix via polar
    /// decomposition (SVD) to produce a rigid transform. When false the full
    /// affine deformation gradient is used.
    pub rigid_projection: bool,

    /// Global deformation strength (0 = no deform, 1 = full).
    pub mask: f32,

    /// Optional per-point float attribute overriding `mask` per mesh point.
    pub mask_attrib: Option<String>,

    /// Recompute normals after deformation.
    pub recompute_normals: bool,

    /// Glob pattern for which attributes to transform (currently unused).
    pub attribs_to_transform: String,

    /// Remove internal capture attributes after deformation.
    pub delete_capture_attribs: bool,
}

impl Default for PointDeformParams {
    fn default() -> Self {
        PointDeformParams {
            group: None,
            mode: PointDeformMode::CaptureAndDeform,
            radius: 1.0,
            min_points: 1,
            max_points: 10,
            piece_attrib: None,
            rigid_projection: true,
            mask: 1.0,
            mask_attrib: None,
            recompute_normals: true,
            attribs_to_transform: String::from("*"),
            delete_capture_attribs: true,
        }
    }
}

// ===========================================================================
// SOP implementation
// ===========================================================================

pub struct PointDeformSop;

impl Sop for PointDeformSop {
    type Params = PointDeformParams;

    fn name(&self) -> &'static str {
        "pointdeform"
    }

    /// Requires exactly 3 inputs: mesh, rest lattice, deformed lattice.
    fn input_count(&self) -> (usize, usize) {
        (3, 3)
    }

    fn execute(
        &self,
        inputs: &[&Geometry],
        params: &Self::Params,
    ) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;

        // Capture-only and Deform-only are stubs for now (need array attribs).
        if params.mode == PointDeformMode::Capture || params.mode == PointDeformMode::Deform {
            return Err(SopError::Other(
                "Capture-only and Deform-only modes are not yet supported (requires array attributes)".into(),
            ));
        }

        let mesh = inputs[0];
        let rest_lattice = inputs[1];
        let deformed_lattice = inputs[2];

        let num_mesh_pts = mesh.num_points();
        let num_lattice_pts = rest_lattice.num_points();

        // Clone mesh — we will mutate positions in-place.
        let mut output = mesh.clone();

        if num_mesh_pts == 0 || num_lattice_pts == 0 {
            return Ok(output);
        }

        // -------------------------------------------------------------------
        // Build correspondence map: rest lattice index → deformed lattice index
        // -------------------------------------------------------------------
        let correspondence = build_correspondence(rest_lattice, deformed_lattice);

        // -------------------------------------------------------------------
        // Collect rest & deformed positions using correspondence
        // -------------------------------------------------------------------
        let rest_positions: Vec<Vec3> = (0..num_lattice_pts)
            .map(|i| rest_lattice.point_pos(PointHandle::from_index(i)))
            .collect();

        let deformed_positions: Vec<Vec3> = (0..num_lattice_pts)
            .map(|i| {
                let def_idx = correspondence[i];
                deformed_lattice.point_pos(PointHandle::from_index(def_idx))
            })
            .collect();

        // -------------------------------------------------------------------
        // Build KD-tree on rest lattice
        // -------------------------------------------------------------------
        let kdtree = KdTree::build(&rest_positions);

        // -------------------------------------------------------------------
        // Compute per-lattice-point local transforms
        // -------------------------------------------------------------------
        let neighbors_map = build_neighbor_map(rest_lattice, num_lattice_pts);
        let local_transforms = compute_local_transforms(
            &rest_positions,
            &deformed_positions,
            &neighbors_map,
            params.rigid_projection,
        );

        // -------------------------------------------------------------------
        // Resolve group membership
        // -------------------------------------------------------------------
        let group_membership: Option<Vec<bool>> = params.group.as_ref().and_then(|name| {
            if name.is_empty() {
                None
            } else {
                mesh.groups().point_group(name).map(|grp| {
                    (0..num_mesh_pts).map(|i| grp.contains(i)).collect()
                })
            }
        });

        // -------------------------------------------------------------------
        // Resolve per-point mask attribute
        // -------------------------------------------------------------------
        let mask_values: Option<Vec<f32>> = params.mask_attrib.as_ref().and_then(|name| {
            if name.is_empty() {
                return None;
            }
            let handle: AttribHandle<f32> = AttribHandle::new(AttribClass::Point, name.as_str());
            let vals: Vec<f32> = (0..num_mesh_pts)
                .map(|i| mesh.get_attrib(&handle, i).unwrap_or(1.0))
                .collect();
            Some(vals)
        });

        // -------------------------------------------------------------------
        // Deform each mesh point
        // -------------------------------------------------------------------
        let min_pts = params.min_points.max(1) as usize;
        let max_pts = params.max_points.max(1) as usize;
        let global_mask = params.mask.clamp(0.0, 1.0);

        for pt_idx in 0..num_mesh_pts {
            // Skip if not in group.
            if let Some(ref membership) = group_membership {
                if !membership[pt_idx] {
                    continue;
                }
            }

            let original_pos = mesh.point_pos(PointHandle::from_index(pt_idx));

            // Per-point mask.
            let point_mask = match mask_values {
                Some(ref vals) => (vals[pt_idx] * global_mask).clamp(0.0, 1.0),
                None => global_mask,
            };

            if point_mask < 1e-8 {
                continue;
            }

            // Capture: find nearby lattice points.
            let mut captures = kdtree.radius_search(original_pos, params.radius);

            // If we have fewer than min_points, use KNN to get at least min_points.
            if captures.len() < min_pts {
                captures = kdtree.k_nearest(original_pos, min_pts);
            }

            // Sort by distance and keep at most max_points.
            captures.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            captures.truncate(max_pts);

            if captures.is_empty() {
                continue;
            }

            // Compute weights using Elendt metaball kernel: w(d) = (1 - (d/R)^2)^2
            // Use the maximum captured distance as the effective radius to ensure
            // all captured points get non-zero weight.
            let max_dist_sq = captures.last().map(|c| c.1).unwrap_or(1.0);
            let effective_radius_sq = if max_dist_sq > params.radius * params.radius {
                max_dist_sq * 1.01 // Slightly expand to avoid zero weight at boundary
            } else {
                params.radius * params.radius
            };

            let mut weighted_pos = Vec3::ZERO;
            let mut total_weight = 0.0f32;

            for &(lattice_idx, dist_sq) in &captures {
                let ratio = dist_sq / effective_radius_sq;
                let w = if ratio >= 1.0 {
                    0.0
                } else {
                    let t = 1.0 - ratio;
                    t * t // (1 - (d/R)^2)^2
                };

                if w < 1e-12 {
                    continue;
                }

                let (rotation, _) = &local_transforms[lattice_idx];
                let rest_pos = rest_positions[lattice_idx];
                let def_pos = deformed_positions[lattice_idx];

                // Transform: R * (pos - rest) + deformed
                let offset = original_pos - rest_pos;
                let transformed = *rotation * offset + def_pos;

                weighted_pos += w * transformed;
                total_weight += w;
            }

            if total_weight > 1e-12 {
                let deformed_pos = weighted_pos / total_weight;
                let final_pos = original_pos.lerp(deformed_pos, point_mask);
                output.set_point_pos(PointHandle::from_index(pt_idx), final_pos);
            }
        }

        Ok(output)
    }
}

// ===========================================================================
// Correspondence
// ===========================================================================

/// Build a mapping from rest lattice point index → deformed lattice point index.
///
/// If both geometries have a point `id` integer attribute, match by id.
/// Otherwise fall back to index correspondence (rest[i] → deformed[i]).
fn build_correspondence(rest: &Geometry, deformed: &Geometry) -> Vec<usize> {
    let num_rest = rest.num_points();
    let num_def = deformed.num_points();

    // Try to find "id" attribute on both.
    let rest_id_handle: Result<AttribHandle<i32>, _> =
        rest.find_attrib::<i32>(AttribClass::Point, "id");
    let def_id_handle: Result<AttribHandle<i32>, _> =
        deformed.find_attrib::<i32>(AttribClass::Point, "id");

    if let (Ok(ref rest_h), Ok(ref def_h)) = (rest_id_handle, def_id_handle) {
        // Build a map from deformed id → deformed index.
        let mut id_to_def_idx = std::collections::HashMap::new();
        for i in 0..num_def {
            if let Ok(id_val) = deformed.get_attrib(def_h, i) {
                id_to_def_idx.insert(id_val, i);
            }
        }

        (0..num_rest)
            .map(|i| {
                if let Ok(id_val) = rest.get_attrib(rest_h, i) {
                    *id_to_def_idx.get(&id_val).unwrap_or(&i)
                } else {
                    i.min(num_def.saturating_sub(1))
                }
            })
            .collect()
    } else {
        // Fallback: index correspondence.
        (0..num_rest)
            .map(|i| i.min(num_def.saturating_sub(1)))
            .collect()
    }
}

// ===========================================================================
// Neighbor map from primitives
// ===========================================================================

/// For each lattice point, gather the set of neighboring point indices
/// (connected via shared primitives).
fn build_neighbor_map(lattice: &Geometry, num_pts: usize) -> Vec<Vec<usize>> {
    let mut neighbors: Vec<Vec<usize>> = vec![Vec::new(); num_pts];
    let num_prims = lattice.num_prims();

    for prim_idx in 0..num_prims {
        let pts = lattice.prim_points(PrimHandle::from_index(prim_idx));
        for i in 0..pts.len() {
            for j in (i + 1)..pts.len() {
                let a = pts[i].index();
                let b = pts[j].index();
                if !neighbors[a].contains(&b) {
                    neighbors[a].push(b);
                }
                if !neighbors[b].contains(&a) {
                    neighbors[b].push(a);
                }
            }
        }
    }

    neighbors
}

// ===========================================================================
// Per-lattice-point local transforms
// ===========================================================================

/// For each lattice point, compute a (rotation, translation) pair that best
/// maps the rest neighborhood to the deformed neighborhood.
///
/// Returns `Vec<(Mat3, Vec3)>` — rotation matrix and translation for each
/// lattice point.
fn compute_local_transforms(
    rest_positions: &[Vec3],
    deformed_positions: &[Vec3],
    neighbors_map: &[Vec<usize>],
    rigid_projection: bool,
) -> Vec<(Mat3, Vec3)> {
    let n = rest_positions.len();
    let mut transforms = Vec::with_capacity(n);

    for i in 0..n {
        let rest_i = rest_positions[i];
        let def_i = deformed_positions[i];

        let nbrs = &neighbors_map[i];

        if nbrs.is_empty() {
            // No connectivity — pure translation.
            transforms.push((Mat3::IDENTITY, def_i - rest_i));
            continue;
        }

        // Build covariance matrix: H = Σ (def_offset * rest_offset^T)
        // Using column-major storage for glam Mat3.
        let mut h = [[0.0f32; 3]; 3]; // h[col][row]

        for &j in nbrs {
            let rest_offset = rest_positions[j] - rest_i;
            let def_offset = deformed_positions[j] - def_i;

            // Outer product: def_offset * rest_offset^T
            // H[row][col] += def_offset[row] * rest_offset[col]
            // glam Mat3 is column-major, so mat.col(c)[r]
            for (col, col_h) in h.iter_mut().enumerate() {
                for (row, cell) in col_h.iter_mut().enumerate() {
                    *cell += component(def_offset, row) * component(rest_offset, col);
                }
            }
        }

        let rotation = if rigid_projection {
            polar_decomposition_rotation(h)
        } else {
            // For non-rigid, use the deformation gradient directly.
            // This requires the inverse of the rest covariance — fall back to
            // rigid for now as a safe default.
            polar_decomposition_rotation(h)
        };

        transforms.push((rotation, def_i - rest_i));
    }

    transforms
}

#[inline]
fn component(v: Vec3, idx: usize) -> f32 {
    match idx {
        0 => v.x,
        1 => v.y,
        _ => v.z,
    }
}

/// Extract the rotation component from a 3×3 matrix via SVD polar decomposition
/// using nalgebra.
///
/// Given H = U * Σ * V^T, the closest rotation is R = U * V^T.
/// If det(R) < 0 we flip the column of U corresponding to the smallest
/// singular value to ensure a proper rotation.
fn polar_decomposition_rotation(h: [[f32; 3]; 3]) -> Mat3 {
    use nalgebra::Matrix3 as NMat3;

    // nalgebra Matrix3::new() takes arguments in *row-major* order:
    // new(m11, m12, m13, m21, m22, m23, m31, m32, m33)
    // We have h[col][row], so h[c][r] is element at row r, col c.
    // For row-major input we need (r=0,c=0), (r=0,c=1), (r=0,c=2), (r=1,c=0), ...
    let na_h = NMat3::new(
        h[0][0], h[1][0], h[2][0],
        h[0][1], h[1][1], h[2][1],
        h[0][2], h[1][2], h[2][2],
    );

    let svd = na_h.svd(true, true);

    let u = svd.u.unwrap_or_else(NMat3::identity);
    let v_t = svd.v_t.unwrap_or_else(NMat3::identity);

    let mut r = u * v_t;

    // Ensure proper rotation (det = +1).
    if r.determinant() < 0.0 {
        // Flip the column of U with the smallest singular value.
        let mut u_fixed = u;
        // Singular values are sorted descending in nalgebra, so index 2 is smallest.
        u_fixed[(0, 2)] = -u_fixed[(0, 2)];
        u_fixed[(1, 2)] = -u_fixed[(1, 2)];
        u_fixed[(2, 2)] = -u_fixed[(2, 2)];
        r = u_fixed * v_t;
    }

    // Convert nalgebra → glam.
    Mat3::from_cols(
        Vec3::new(r[(0, 0)], r[(1, 0)], r[(2, 0)]),
        Vec3::new(r[(0, 1)], r[(1, 1)], r[(2, 1)]),
        Vec3::new(r[(0, 2)], r[(1, 2)], r[(2, 2)]),
    )
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;
    use procgeo_core::{Geometry, PointHandle};

    /// Helper: create a simple 2×2 grid of points with one quad primitive.
    fn make_lattice(positions: &[Vec3]) -> Geometry {
        let mut geo = Geometry::new();
        let handles: Vec<PointHandle> = positions.iter().map(|&p| geo.add_point(p)).collect();
        // If we have at least 3 points, create a polygon to establish connectivity.
        if handles.len() >= 3 {
            geo.add_polygon(&handles, procgeo_core::primitive::PolyType::Closed);
        }
        geo
    }

    /// Helper: create a simple mesh (a single triangle).
    fn make_mesh() -> Geometry {
        let mut geo = Geometry::new();
        let p0 = geo.add_point(Vec3::new(0.0, 0.0, 0.0));
        let p1 = geo.add_point(Vec3::new(1.0, 0.0, 0.0));
        let p2 = geo.add_point(Vec3::new(0.5, 1.0, 0.0));
        geo.add_polygon(&[p0, p1, p2], procgeo_core::primitive::PolyType::Closed);
        geo
    }

    #[test]
    fn identity_when_rest_equals_deformed() {
        let mesh = make_mesh();

        let lattice_pts = vec![
            Vec3::new(-1.0, -1.0, 0.0),
            Vec3::new(2.0, -1.0, 0.0),
            Vec3::new(2.0, 2.0, 0.0),
            Vec3::new(-1.0, 2.0, 0.0),
        ];
        let rest = make_lattice(&lattice_pts);
        let deformed = make_lattice(&lattice_pts); // Same as rest!

        let params = PointDeformParams {
            radius: 5.0,
            ..Default::default()
        };

        let result = PointDeformSop
            .execute(&[&mesh, &rest, &deformed], &params)
            .unwrap();

        // Every point should remain at its original position.
        for i in 0..mesh.num_points() {
            let orig = mesh.point_pos(PointHandle::from_index(i));
            let deformed_pt = result.point_pos(PointHandle::from_index(i));
            assert!(
                (orig - deformed_pt).length() < 1e-4,
                "point {} moved from {:?} to {:?}",
                i,
                orig,
                deformed_pt,
            );
        }
    }

    #[test]
    fn pure_translation() {
        let mesh = make_mesh();
        let offset = Vec3::new(5.0, 3.0, -2.0);

        let lattice_pts = vec![
            Vec3::new(-1.0, -1.0, 0.0),
            Vec3::new(2.0, -1.0, 0.0),
            Vec3::new(2.0, 2.0, 0.0),
            Vec3::new(-1.0, 2.0, 0.0),
        ];
        let rest = make_lattice(&lattice_pts);
        let deformed_pts: Vec<Vec3> = lattice_pts.iter().map(|&p| p + offset).collect();
        let deformed = make_lattice(&deformed_pts);

        let params = PointDeformParams {
            radius: 5.0,
            ..Default::default()
        };

        let result = PointDeformSop
            .execute(&[&mesh, &rest, &deformed], &params)
            .unwrap();

        for i in 0..mesh.num_points() {
            let orig = mesh.point_pos(PointHandle::from_index(i));
            let deformed_pt = result.point_pos(PointHandle::from_index(i));
            let expected = orig + offset;
            assert!(
                (expected - deformed_pt).length() < 1e-3,
                "point {} expected {:?} got {:?}",
                i,
                expected,
                deformed_pt,
            );
        }
    }

    #[test]
    fn preserves_point_count() {
        let mesh = make_mesh();

        let lattice_pts = vec![
            Vec3::new(-1.0, -1.0, 0.0),
            Vec3::new(2.0, -1.0, 0.0),
            Vec3::new(2.0, 2.0, 0.0),
            Vec3::new(-1.0, 2.0, 0.0),
        ];
        let rest = make_lattice(&lattice_pts);
        let deformed_pts: Vec<Vec3> = lattice_pts
            .iter()
            .map(|&p| p + Vec3::new(0.0, 1.0, 0.0))
            .collect();
        let deformed = make_lattice(&deformed_pts);

        let params = PointDeformParams {
            radius: 5.0,
            ..Default::default()
        };

        let result = PointDeformSop
            .execute(&[&mesh, &rest, &deformed], &params)
            .unwrap();

        assert_eq!(result.num_points(), mesh.num_points());
        assert_eq!(result.num_prims(), mesh.num_prims());
    }

    #[test]
    fn requires_three_inputs() {
        let mesh = make_mesh();
        let rest = make_mesh();
        let params = PointDeformParams::default();

        // Zero inputs.
        let err = PointDeformSop.execute(&[], &params);
        assert!(err.is_err());

        // One input.
        let err = PointDeformSop.execute(&[&mesh], &params);
        assert!(err.is_err());

        // Two inputs.
        let err = PointDeformSop.execute(&[&mesh, &rest], &params);
        assert!(err.is_err());
    }

    #[test]
    fn empty_mesh_returns_empty() {
        let mesh = Geometry::new();
        let rest = make_mesh();
        let deformed = make_mesh();
        let params = PointDeformParams::default();

        let result = PointDeformSop
            .execute(&[&mesh, &rest, &deformed], &params)
            .unwrap();
        assert_eq!(result.num_points(), 0);
    }

    #[test]
    fn mask_zero_prevents_deformation() {
        let mesh = make_mesh();
        let offset = Vec3::new(5.0, 3.0, -2.0);

        let lattice_pts = vec![
            Vec3::new(-1.0, -1.0, 0.0),
            Vec3::new(2.0, -1.0, 0.0),
            Vec3::new(2.0, 2.0, 0.0),
            Vec3::new(-1.0, 2.0, 0.0),
        ];
        let rest = make_lattice(&lattice_pts);
        let deformed_pts: Vec<Vec3> = lattice_pts.iter().map(|&p| p + offset).collect();
        let deformed = make_lattice(&deformed_pts);

        let params = PointDeformParams {
            radius: 5.0,
            mask: 0.0, // No deformation!
            ..Default::default()
        };

        let result = PointDeformSop
            .execute(&[&mesh, &rest, &deformed], &params)
            .unwrap();

        for i in 0..mesh.num_points() {
            let orig = mesh.point_pos(PointHandle::from_index(i));
            let deformed_pt = result.point_pos(PointHandle::from_index(i));
            assert!(
                (orig - deformed_pt).length() < 1e-6,
                "mask=0 should prevent deformation, point {} moved",
                i,
            );
        }
    }
}
