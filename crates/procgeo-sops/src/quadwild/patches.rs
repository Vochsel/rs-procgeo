// Patch decomposition: partition triangle faces into quad-like patches
// using the traced curves as boundaries.
//
// Each patch is a connected region of triangles bounded by traced curves
// and/or mesh boundaries. Patches ideally have 4 sides (quad-like),
// but may have 3 or 5+ sides (T-junctions, irregular patches).

use std::collections::{HashMap, HashSet, VecDeque};

use glam::Vec3;

use procgeo_core::{Geometry, PointHandle};

use super::adjacency::{EdgeKey, MeshAdjacency};
use super::tracing::TraceResult;

/// A single patch in the decomposition.
#[derive(Clone, Debug)]
pub struct Patch {
    /// Triangle face indices belonging to this patch.
    pub faces: Vec<usize>,
    /// Boundary edges of this patch (ordered).
    pub boundary_edges: Vec<EdgeKey>,
    /// Boundary vertices in order around the patch.
    pub boundary_verts: Vec<usize>,
    /// Corner vertex indices (where patch sides meet).
    pub corners: Vec<usize>,
    /// Sides: each side is a sequence of boundary vertices between consecutive corners.
    pub sides: Vec<Vec<usize>>,
    /// Number of sides (ideally 4 for a quad patch).
    pub num_sides: usize,
}

/// Full patch decomposition.
#[derive(Clone, Debug)]
pub struct PatchDecomposition {
    pub patches: Vec<Patch>,
    /// Face-to-patch mapping: patch_id[face_idx] = patch index.
    pub face_patch: Vec<usize>,
}

/// Decompose the mesh into patches using traced curves as separators.
pub fn decompose_patches(
    geo: &Geometry,
    adj: &MeshAdjacency,
    trace: &TraceResult,
) -> PatchDecomposition {
    let num_faces = adj.faces.len();

    // Step 1: Mark edges that are cut by traced curves
    let cut_edges = compute_cut_edges(geo, adj, trace);

    // Step 2: Flood-fill to assign faces to patches (faces separated by cut edges)
    let face_patch = flood_fill_patches(adj, &cut_edges, num_faces);
    let num_patches = face_patch.iter().copied().max().map(|m| m + 1).unwrap_or(0);

    // Step 3: Build patch structures
    let mut patches = Vec::with_capacity(num_patches);
    for pi in 0..num_patches {
        let faces: Vec<usize> = face_patch
            .iter()
            .enumerate()
            .filter(|&(_, &p)| p == pi)
            .map(|(fi, _)| fi)
            .collect();

        let patch = build_patch(geo, adj, &faces, &cut_edges);
        patches.push(patch);
    }

    PatchDecomposition {
        patches,
        face_patch,
    }
}

/// Determine which mesh edges are "cut" by the traced curves.
fn compute_cut_edges(
    geo: &Geometry,
    adj: &MeshAdjacency,
    trace: &TraceResult,
) -> HashSet<EdgeKey> {
    let mut cut = HashSet::new();

    for curve in &trace.curves {
        // For each segment of the curve, find the mesh edge it crosses
        for i in 0..curve.face_sequence.len().saturating_sub(1) {
            let f0 = curve.face_sequence[i];
            let f1 = curve.face_sequence[i + 1];
            if f0 == f1 {
                continue;
            }

            // Find the shared edge between f0 and f1
            for &ek in &adj.faces[f0].edges {
                if adj.faces[f1].edges.contains(&ek) {
                    cut.insert(ek);
                }
            }
        }

        // Also mark edges that the curve points are close to
        for (i, pos) in curve.points.iter().enumerate() {
            if i >= curve.face_sequence.len() {
                break;
            }
            let fi = curve.face_sequence[i];
            let pts = &adj.faces[fi].points;
            let n = pts.len();

            for ei in 0..n {
                let pa = geo.point_pos(PointHandle::from_index(pts[ei]));
                let pb = geo.point_pos(PointHandle::from_index(pts[(ei + 1) % n]));
                let edge_mid = (pa + pb) * 0.5;
                let edge_len = (pb - pa).length();

                if edge_len > 0.0 && (*pos - edge_mid).length() < edge_len * 0.5 {
                    let ek = adj.faces[fi].edges[ei];
                    // Only cut if the curve crosses this edge (not just passes nearby)
                    if is_crossing(pos, &curve.points, &pa, &pb) {
                        cut.insert(ek);
                    }
                }
            }
        }
    }

    // Add boundary edges as cuts
    for &ek in &adj.boundary_edges {
        cut.insert(ek);
    }

    cut
}

/// Simple check if a curve point is near an edge crossing.
fn is_crossing(pos: &Vec3, curve_points: &[Vec3], ea: &Vec3, eb: &Vec3) -> bool {
    let edge_dir = (*eb - *ea).normalize_or_zero();
    let edge_len = (*eb - *ea).length();
    if edge_len < 1e-10 {
        return false;
    }

    // Check if the curve direction is roughly perpendicular to the edge
    // (a true crossing should go across the edge, not along it)
    let pos_idx = curve_points.iter().position(|p| (*p - *pos).length() < 1e-8);
    if let Some(idx) = pos_idx {
        if idx > 0 && idx < curve_points.len() - 1 {
            let curve_dir = (curve_points[idx + 1] - curve_points[idx - 1]).normalize_or_zero();
            let cross = curve_dir.dot(edge_dir).abs();
            return cross < 0.8; // Not parallel to edge
        }
    }
    true // Default to yes for endpoints
}

/// Flood fill to assign patch IDs to faces.
fn flood_fill_patches(
    adj: &MeshAdjacency,
    cut_edges: &HashSet<EdgeKey>,
    num_faces: usize,
) -> Vec<usize> {
    let mut patch_id = vec![usize::MAX; num_faces];
    let mut current_patch = 0;

    for start in 0..num_faces {
        if patch_id[start] != usize::MAX {
            continue;
        }

        // BFS from this face
        let mut queue = VecDeque::new();
        queue.push_back(start);
        patch_id[start] = current_patch;

        while let Some(fi) = queue.pop_front() {
            let n = adj.faces[fi].edges.len();
            for ei in 0..n {
                let edge = adj.faces[fi].edges[ei];

                // Don't cross cut edges
                if cut_edges.contains(&edge) {
                    continue;
                }

                if let Some(adj_face) = adj.faces[fi].adj[ei] {
                    if patch_id[adj_face] == usize::MAX {
                        patch_id[adj_face] = current_patch;
                        queue.push_back(adj_face);
                    }
                }
            }
        }

        current_patch += 1;
    }

    patch_id
}

/// Build a Patch struct from a set of face indices.
fn build_patch(
    geo: &Geometry,
    adj: &MeshAdjacency,
    faces: &[usize],
    cut_edges: &HashSet<EdgeKey>,
) -> Patch {
    let face_set: HashSet<usize> = faces.iter().copied().collect();

    // Find boundary edges: edges shared with faces outside this patch or boundary edges
    let mut boundary_edges = Vec::new();
    let mut boundary_vert_set = HashSet::new();

    for &fi in faces {
        let n = adj.faces[fi].edges.len();
        for ei in 0..n {
            let edge = adj.faces[fi].edges[ei];
            let is_boundary = match adj.faces[fi].adj[ei] {
                None => true,
                Some(adj_face) => !face_set.contains(&adj_face),
            };
            let is_cut = cut_edges.contains(&edge);

            if is_boundary || is_cut {
                boundary_edges.push(edge);
                boundary_vert_set.insert(edge.0);
                boundary_vert_set.insert(edge.1);
            }
        }
    }

    // Order boundary vertices into a loop
    let boundary_verts = order_boundary_loop(&boundary_edges, &boundary_vert_set);

    // Detect corners: boundary vertices where the angle is sharp
    let corners = detect_patch_corners(geo, adj, &boundary_verts, &face_set);

    // Split boundary into sides at corners
    let sides = split_boundary_at_corners(&boundary_verts, &corners);
    let num_sides = sides.len().max(1);

    Patch {
        faces: faces.to_vec(),
        boundary_edges,
        boundary_verts,
        corners,
        sides,
        num_sides,
    }
}

/// Order boundary edges into a vertex loop.
fn order_boundary_loop(
    edges: &[EdgeKey],
    _verts: &HashSet<usize>,
) -> Vec<usize> {
    if edges.is_empty() {
        return Vec::new();
    }

    // Build adjacency of boundary edges
    let mut vert_adj: HashMap<usize, Vec<usize>> = HashMap::new();
    for &ek in edges {
        vert_adj.entry(ek.0).or_default().push(ek.1);
        vert_adj.entry(ek.1).or_default().push(ek.0);
    }

    // Walk the boundary
    let start = edges[0].0;
    let mut loop_verts = vec![start];
    let mut visited = HashSet::new();
    visited.insert(start);

    let mut current = start;
    loop {
        let neighbors = vert_adj.get(&current);
        if neighbors.is_none() {
            break;
        }
        let mut found = false;
        for &next in neighbors.unwrap() {
            if !visited.contains(&next) {
                loop_verts.push(next);
                visited.insert(next);
                current = next;
                found = true;
                break;
            }
        }
        if !found {
            break;
        }
    }

    loop_verts
}

/// Detect corner vertices on a patch boundary (high angle deviation).
fn detect_patch_corners(
    geo: &Geometry,
    _adj: &MeshAdjacency,
    boundary: &[usize],
    _face_set: &HashSet<usize>,
) -> Vec<usize> {
    if boundary.len() < 3 {
        return boundary.to_vec();
    }

    let mut corners = Vec::new();
    let n = boundary.len();

    for i in 0..n {
        let prev = boundary[(i + n - 1) % n];
        let curr = boundary[i];
        let next = boundary[(i + 1) % n];

        let p_prev = geo.point_pos(PointHandle::from_index(prev));
        let p_curr = geo.point_pos(PointHandle::from_index(curr));
        let p_next = geo.point_pos(PointHandle::from_index(next));

        let e1 = (p_prev - p_curr).normalize_or_zero();
        let e2 = (p_next - p_curr).normalize_or_zero();
        let cos_angle = e1.dot(e2).clamp(-1.0, 1.0);
        let angle = cos_angle.acos();

        // Mark as corner if angle deviates significantly from 180° (straight)
        let deviation = (std::f32::consts::PI - angle).abs();
        if deviation > 0.5 { // ~30 degrees
            corners.push(curr);
        }
    }

    // Ensure at least 3 corners for the patch to be valid
    if corners.len() < 3 && boundary.len() >= 3 {
        // Space corners evenly
        let step = boundary.len() / 4.max(3);
        corners.clear();
        for i in 0..4.min(boundary.len()) {
            corners.push(boundary[i * step % boundary.len()]);
        }
    }

    corners
}

/// Split the boundary vertex loop into sides at corners.
fn split_boundary_at_corners(
    boundary: &[usize],
    corners: &[usize],
) -> Vec<Vec<usize>> {
    if boundary.is_empty() || corners.is_empty() {
        return vec![boundary.to_vec()];
    }

    let corner_set: HashSet<usize> = corners.iter().copied().collect();

    // Find the first corner in the boundary
    let first_corner_idx = boundary.iter().position(|v| corner_set.contains(v));
    if first_corner_idx.is_none() {
        return vec![boundary.to_vec()];
    }
    let start = first_corner_idx.unwrap();

    let mut sides = Vec::new();
    let mut current_side = Vec::new();
    let n = boundary.len();

    for i in 0..n {
        let idx = (start + i) % n;
        let vi = boundary[idx];
        current_side.push(vi);

        if current_side.len() > 1 && corner_set.contains(&vi) {
            sides.push(current_side.clone());
            current_side = vec![vi]; // Start new side from this corner
        }
    }

    // Close the last side back to the first corner
    if current_side.len() > 1 {
        current_side.push(boundary[start]);
        sides.push(current_side);
    }

    if sides.is_empty() {
        sides.push(boundary.to_vec());
    }

    sides
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::adjacency::MeshAdjacency;
    use super::super::cross_field;
    use super::super::features;
    use super::super::tracing;

    fn make_grid() -> (Geometry, MeshAdjacency) {
        let mut geo = Geometry::new();
        let mut pts = Vec::new();
        for r in 0..=4 {
            for c in 0..=4 {
                pts.push(geo.add_point(Vec3::new(c as f32, r as f32, 0.0)));
            }
        }
        let w = 5;
        for r in 0..4 {
            for c in 0..4 {
                geo.add_face(&[pts[r * w + c], pts[r * w + c + 1], pts[(r + 1) * w + c + 1]]);
                geo.add_face(&[pts[r * w + c], pts[(r + 1) * w + c + 1], pts[(r + 1) * w + c]]);
            }
        }
        let adj = MeshAdjacency::build(&geo).unwrap();
        (geo, adj)
    }

    #[test]
    fn decomposition_assigns_all_faces() {
        let (geo, adj) = make_grid();
        let sharp = features::detect_sharp_edges(&geo, &adj, 35.0);
        let field = cross_field::compute_cross_field(&geo, &adj, &sharp, 0.3, 5);
        let trace = tracing::trace_field_curves(&geo, &adj, &field, &sharp);
        let decomp = decompose_patches(&geo, &adj, &trace);

        // Every face should be assigned to a patch
        assert_eq!(decomp.face_patch.len(), adj.faces.len());
        for &pid in &decomp.face_patch {
            assert!(pid < decomp.patches.len());
        }
    }

    #[test]
    fn patches_cover_all_faces() {
        let (geo, adj) = make_grid();
        let sharp = features::detect_sharp_edges(&geo, &adj, 35.0);
        let field = cross_field::compute_cross_field(&geo, &adj, &sharp, 0.3, 5);
        let trace = tracing::trace_field_curves(&geo, &adj, &field, &sharp);
        let decomp = decompose_patches(&geo, &adj, &trace);

        let total_faces: usize = decomp.patches.iter().map(|p| p.faces.len()).sum();
        assert_eq!(total_faces, adj.faces.len());
    }
}
