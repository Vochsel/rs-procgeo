// Cross-field computation on triangle meshes.
//
// A cross-field assigns a pair of orthogonal directions (u, v) to each triangle,
// representing the desired quad edge directions. The field has 4-fold rotational
// symmetry (90° rotations are equivalent).
//
// Algorithm:
// 1. Initialize field from principal curvature directions
// 2. Constrain field at sharp features to align with feature edges
// 3. Smooth field using angle-based relaxation in the 4-symmetry representation
// 4. Detect singularities (points where the field has non-zero index)

use std::f32::consts::PI;

use glam::Vec3;

use procgeo_core::{Geometry, PointHandle};

use super::adjacency::{EdgeKey, MeshAdjacency};
use super::features::SharpEdges;

/// Cross-field: one direction per face (the second is the 90° rotation around the face normal).
#[derive(Clone, Debug)]
pub struct CrossField {
    /// Primary field direction per face (tangent to the surface).
    pub directions: Vec<Vec3>,
    /// Face normals.
    pub normals: Vec<Vec3>,
    /// Singularity index per vertex (+1 or -1, 0 for regular).
    pub singularities: Vec<i32>,
    /// Singular vertex indices.
    pub singular_vertices: Vec<usize>,
}

impl CrossField {
    /// Get the cross-field direction for face fi.
    pub fn direction(&self, fi: usize) -> Vec3 {
        self.directions[fi]
    }

    /// Get the secondary (90°-rotated) direction for face fi.
    pub fn secondary_direction(&self, fi: usize) -> Vec3 {
        self.normals[fi].cross(self.directions[fi]).normalize_or_zero()
    }
}

/// Compute a curvature-aligned cross-field on the triangle mesh.
///
/// Uses the angle-based representation: each direction is encoded as an angle θ
/// relative to a local reference frame on each face. The 4-symmetry is encoded
/// by working with 4θ, so that 90° rotations become full 360° rotations.
pub fn compute_cross_field(
    geo: &Geometry,
    adj: &MeshAdjacency,
    sharp: &SharpEdges,
    curvature_weight: f32,
    iterations: u32,
) -> CrossField {
    let num_faces = adj.faces.len();

    // Step 1: Compute face normals and local reference frames
    let mut normals = Vec::with_capacity(num_faces);
    let mut ref_dirs = Vec::with_capacity(num_faces); // reference direction per face

    for fi in 0..num_faces {
        let n = adj.face_normal(geo, fi);
        normals.push(n);

        // Build a local tangent reference direction: first edge of the face
        let pts = &adj.faces[fi].points;
        let p0 = geo.point_pos(PointHandle::from_index(pts[0]));
        let p1 = geo.point_pos(PointHandle::from_index(pts[1]));
        let e = (p1 - p0).normalize_or_zero();
        // Project e onto the tangent plane (should already be tangent for planar faces)
        let t = (e - n * e.dot(n)).normalize_or_zero();
        ref_dirs.push(if t.length_squared() > 0.01 { t } else { arbitrary_tangent(n) });
    }

    // Step 2: Initialize field angles from curvature
    let mut angles = vec![0.0f32; num_faces]; // angle of direction relative to ref_dir

    // Initialize from principal curvature directions
    initialize_from_curvature(geo, adj, &normals, &ref_dirs, &mut angles, curvature_weight);

    // Step 3: Apply sharp feature constraints
    apply_sharp_constraints(geo, adj, sharp, &normals, &ref_dirs, &mut angles);

    // Step 4: Smooth the field
    let constrained = build_constraint_mask(adj, sharp);
    for _ in 0..iterations {
        smooth_field_iteration(adj, &normals, &ref_dirs, &mut angles, &constrained);
    }

    // Step 5: Convert angles back to 3D directions
    let mut directions = Vec::with_capacity(num_faces);
    for fi in 0..num_faces {
        let dir = angle_to_direction(angles[fi], &ref_dirs[fi], &normals[fi]);
        directions.push(dir);
    }

    // Step 6: Detect singularities
    let singularities = detect_singularities(geo, adj, &angles, &ref_dirs, &normals);
    let singular_vertices: Vec<usize> = singularities
        .iter()
        .enumerate()
        .filter(|&(_, &s)| s != 0)
        .map(|(i, _)| i)
        .collect();

    CrossField {
        directions,
        normals,
        singularities,
        singular_vertices,
    }
}

/// Build an arbitrary tangent vector perpendicular to normal.
fn arbitrary_tangent(n: Vec3) -> Vec3 {
    let up = if n.y.abs() < 0.9 { Vec3::Y } else { Vec3::X };
    n.cross(up).normalize_or_zero()
}

/// Convert an angle (relative to reference frame) to a 3D direction.
fn angle_to_direction(angle: f32, ref_dir: &Vec3, normal: &Vec3) -> Vec3 {
    let bitangent = normal.cross(*ref_dir).normalize_or_zero();
    let dir = *ref_dir * angle.cos() + bitangent * angle.sin();
    dir.normalize_or_zero()
}

/// Convert a 3D tangent direction to an angle relative to the face's reference frame.
fn direction_to_angle(dir: Vec3, ref_dir: &Vec3, normal: &Vec3) -> f32 {
    let bitangent = normal.cross(*ref_dir).normalize_or_zero();
    let x = dir.dot(*ref_dir);
    let y = dir.dot(bitangent);
    y.atan2(x)
}

/// Compute the rotation angle needed to transport a direction from face f0 to face f1
/// across their shared edge. This measures how the local reference frames differ.
fn transport_angle(
    _adj: &MeshAdjacency,
    normals: &[Vec3],
    ref_dirs: &[Vec3],
    f0: usize,
    f1: usize,
) -> f32 {
    let r0 = ref_dirs[f0];
    let r1 = ref_dirs[f1];
    let _n0 = normals[f0];
    let n1 = normals[f1];

    // Project r0 into the tangent plane of f1
    // First, rotate r0 to be in f1's tangent plane using the shared edge as pivot
    let r0_in_f1 = (r0 - n1 * r0.dot(n1)).normalize_or_zero();

    if r0_in_f1.length_squared() < 0.01 {
        return 0.0;
    }

    // Compute the angle between the projected r0 and r1 in f1's tangent plane
    let b1 = n1.cross(r1).normalize_or_zero();
    let x = r0_in_f1.dot(r1);
    let y = r0_in_f1.dot(b1);
    y.atan2(x)
}

/// Initialize field from estimated principal curvature directions.
fn initialize_from_curvature(
    geo: &Geometry,
    adj: &MeshAdjacency,
    normals: &[Vec3],
    ref_dirs: &[Vec3],
    angles: &mut [f32],
    curvature_weight: f32,
) {
    for fi in 0..adj.faces.len() {
        let pts = &adj.faces[fi].points;
        if pts.len() < 3 {
            continue;
        }

        // Estimate curvature direction from shape operator approximation
        // Use the direction of maximum normal variation as principal curvature direction
        let n = normals[fi];
        let mut max_curv_dir = ref_dirs[fi];
        let mut max_curv = 0.0f32;

        for i in 0..pts.len() {
            let j = (i + 1) % pts.len();
            let pi = geo.point_pos(PointHandle::from_index(pts[i]));
            let pj = geo.point_pos(PointHandle::from_index(pts[j]));
            let edge = pj - pi;
            let edge_len = edge.length();
            if edge_len < 1e-10 {
                continue;
            }

            // Normal variation along edge
            let ni = adj.vertex_normal(geo, pts[i]);
            let nj = adj.vertex_normal(geo, pts[j]);
            let dn = nj - ni;

            // Curvature in edge direction: |dn| / |edge|
            let curv = dn.length() / edge_len;

            if curv > max_curv {
                max_curv = curv;
                // Project edge onto tangent plane
                let t = (edge - n * edge.dot(n)).normalize_or_zero();
                if t.length_squared() > 0.01 {
                    max_curv_dir = t;
                }
            }
        }

        let curv_angle = direction_to_angle(max_curv_dir, &ref_dirs[fi], &normals[fi]);
        angles[fi] = curv_angle * curvature_weight;
    }
}

/// Apply constraints at sharp feature edges: align field with feature direction.
fn apply_sharp_constraints(
    geo: &Geometry,
    adj: &MeshAdjacency,
    sharp: &SharpEdges,
    normals: &[Vec3],
    ref_dirs: &[Vec3],
    angles: &mut [f32],
) {
    for &ek in &sharp.edges {
        let p0 = geo.point_pos(PointHandle::from_index(ek.0));
        let p1 = geo.point_pos(PointHandle::from_index(ek.1));
        let edge_dir = (p1 - p0).normalize_or_zero();

        // Constrain adjacent faces to align with this edge
        if let Some(face_list) = adj.edge_faces.get(&ek) {
            for &fi in face_list {
                let n = normals[fi];
                // Project edge onto tangent plane
                let t = (edge_dir - n * edge_dir.dot(n)).normalize_or_zero();
                if t.length_squared() > 0.01 {
                    let angle = direction_to_angle(t, &ref_dirs[fi], &n);
                    // Snap to nearest 90° multiple of the current angle
                    // in the 4-symmetry representation
                    let theta4 = angle * 4.0;
                    let current4 = angles[fi] * 4.0;
                    let diff = theta4 - current4;
                    // Round to nearest 2π multiple
                    let k = (diff / (2.0 * PI)).round();
                    angles[fi] = (theta4 - k * 2.0 * PI) / 4.0;
                }
            }
        }
    }
}

/// Build constraint mask: which faces have fixed field directions.
fn build_constraint_mask(adj: &MeshAdjacency, sharp: &SharpEdges) -> Vec<bool> {
    let mut constrained = vec![false; adj.faces.len()];
    for &ek in &sharp.edges {
        if let Some(face_list) = adj.edge_faces.get(&ek) {
            for &fi in face_list {
                constrained[fi] = true;
            }
        }
    }
    constrained
}

/// One iteration of field smoothing using 4-symmetry angle relaxation.
///
/// For each unconstrained face, average the transported field angles from
/// neighboring faces, working in the 4θ representation.
fn smooth_field_iteration(
    adj: &MeshAdjacency,
    normals: &[Vec3],
    ref_dirs: &[Vec3],
    angles: &mut [f32],
    constrained: &[bool],
) {
    let old_angles = angles.to_vec();
    let num_faces = adj.faces.len();

    for fi in 0..num_faces {
        if constrained[fi] {
            continue;
        }

        // Collect transported angles from neighbors (in 4θ space)
        let mut sum_sin = 0.0f64;
        let mut sum_cos = 0.0f64;
        let mut count = 0;

        for &maybe_adj in &adj.faces[fi].adj {
            if let Some(fj) = maybe_adj {
                // Transport angle from fj to fi
                let transport = transport_angle(adj, normals, ref_dirs, fj, fi);
                let transported_angle = old_angles[fj] + transport;

                // Work in 4θ space for 4-symmetry
                let theta4 = (transported_angle * 4.0) as f64;
                sum_sin += theta4.sin();
                sum_cos += theta4.cos();
                count += 1;
            }
        }

        if count > 0 {
            let avg_4theta = (sum_sin).atan2(sum_cos);
            angles[fi] = (avg_4theta / 4.0) as f32;
        }
    }
}

/// Detect singularities by computing the field index around each vertex.
///
/// For each vertex, sum the angle defects when transporting the field
/// around the vertex star. Non-zero index indicates a singularity.
fn detect_singularities(
    _geo: &Geometry,
    adj: &MeshAdjacency,
    angles: &[f32],
    ref_dirs: &[Vec3],
    normals: &[Vec3],
) -> Vec<i32> {
    let mut indices = vec![0i32; adj.num_points];

    for vi in 0..adj.num_points {
        if adj.is_boundary_point[vi] {
            continue;
        }

        let ring = &adj.point_faces[vi];
        if ring.len() < 3 {
            continue;
        }

        // Order faces around the vertex
        let ordered = order_faces_around_vertex(adj, vi, ring);
        if ordered.len() < 3 {
            continue;
        }

        // Compute the total angle rotation when going around the vertex
        let mut total_rotation = 0.0f64;

        for i in 0..ordered.len() {
            let fi = ordered[i];
            let fj = ordered[(i + 1) % ordered.len()];

            let transport = transport_angle(adj, normals, ref_dirs, fi, fj);
            let theta_i = angles[fi];
            let theta_j = angles[fj];

            // Angle difference in 4θ space
            let diff = ((theta_j - theta_i - transport) * 4.0) as f64;
            // Wrap to [-π, π] using rem_euclid for correct modular arithmetic
            let wrapped = (diff + PI as f64).rem_euclid(2.0 * PI as f64) - PI as f64;
            total_rotation += wrapped;
        }

        // The singularity index is total_rotation / (2π) in the 4θ representation
        let index = (total_rotation / (2.0 * PI as f64)).round() as i32;
        indices[vi] = index;
    }

    indices
}

/// Order faces around a vertex into a fan.
fn order_faces_around_vertex(
    adj: &MeshAdjacency,
    vi: usize,
    faces: &[usize],
) -> Vec<usize> {
    if faces.is_empty() {
        return Vec::new();
    }

    let mut ordered = Vec::with_capacity(faces.len());
    let mut visited = vec![false; adj.faces.len()];

    ordered.push(faces[0]);
    visited[faces[0]] = true;

    // Walk around the vertex by finding adjacent faces
    for _ in 0..faces.len() {
        let current = *ordered.last().unwrap();
        let pts = &adj.faces[current].points;

        // Find edges of this face that include vertex vi
        let n = pts.len();
        let vi_idx = pts.iter().position(|&p| p == vi);
        if vi_idx.is_none() {
            break;
        }
        let vi_idx = vi_idx.unwrap();

        // Try the next edge (vi -> next_vertex)
        let next_pt = pts[(vi_idx + 1) % n];
        let edge = EdgeKey::new(vi, next_pt);

        let mut found = false;
        if let Some(face_list) = adj.edge_faces.get(&edge) {
            for &fj in face_list {
                if fj != current && !visited[fj] && faces.contains(&fj) {
                    ordered.push(fj);
                    visited[fj] = true;
                    found = true;
                    break;
                }
            }
        }

        if !found {
            // Try the previous edge (prev_vertex -> vi)
            let prev_pt = pts[(vi_idx + n - 1) % n];
            let edge = EdgeKey::new(vi, prev_pt);
            if let Some(face_list) = adj.edge_faces.get(&edge) {
                for &fj in face_list {
                    if fj != current && !visited[fj] && faces.contains(&fj) {
                        ordered.push(fj);
                        visited[fj] = true;
                        break;
                    }
                }
            }
        }
    }

    // If we couldn't order all faces, just return what we have
    if ordered.len() < faces.len() {
        for &fi in faces {
            if !visited[fi] {
                ordered.push(fi);
            }
        }
    }

    ordered
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_flat_grid() -> (Geometry, MeshAdjacency) {
        let mut geo = Geometry::new();
        let mut pts = Vec::new();
        for r in 0..=3 {
            for c in 0..=3 {
                let ph = geo.add_point(Vec3::new(c as f32, r as f32, 0.0));
                pts.push(ph);
            }
        }
        let w = 4;
        for r in 0..3 {
            for c in 0..3 {
                let p0 = pts[r * w + c];
                let p1 = pts[r * w + c + 1];
                let p2 = pts[(r + 1) * w + c + 1];
                let p3 = pts[(r + 1) * w + c];
                geo.add_face(&[p0, p1, p2]);
                geo.add_face(&[p0, p2, p3]);
            }
        }
        let adj = MeshAdjacency::build(&geo).unwrap();
        (geo, adj)
    }

    #[test]
    fn cross_field_produces_directions() {
        let (geo, adj) = make_flat_grid();
        let sharp = super::super::features::detect_sharp_edges(&geo, &adj, 35.0);
        let field = compute_cross_field(&geo, &adj, &sharp, 0.3, 10);
        assert_eq!(field.directions.len(), adj.faces.len());
        // Directions should be unit length and tangent
        for fi in 0..adj.faces.len() {
            let d = field.directions[fi];
            assert!(
                (d.length() - 1.0).abs() < 0.1 || d.length() < 0.01,
                "direction should be roughly unit: {:?}",
                d
            );
            let dot = d.dot(field.normals[fi]).abs();
            assert!(dot < 0.2, "direction should be tangent, dot={dot}");
        }
    }

    #[test]
    fn flat_mesh_no_singularities() {
        let (geo, adj) = make_flat_grid();
        let sharp = super::super::features::detect_sharp_edges(&geo, &adj, 35.0);
        let field = compute_cross_field(&geo, &adj, &sharp, 0.0, 5);
        // On a flat grid, interior vertices should have index 0
        let interior_singular: Vec<_> = field
            .singularities
            .iter()
            .enumerate()
            .filter(|&(ref vi, &s)| s != 0 && !adj.is_boundary_point[*vi])
            .collect();
        assert!(
            interior_singular.is_empty(),
            "flat grid should have no interior singularities: {:?}",
            interior_singular
        );
    }
}
