// Field-aligned curve tracing for patch boundary generation.
//
// Traces separatrices from singularities and samples additional streamlines
// to decompose the surface into quad-like patches. Curves follow the
// cross-field directions and snap to sharp features.

use std::collections::{HashMap, HashSet};

use glam::Vec3;

use procgeo_core::{Geometry, PointHandle};

use super::adjacency::{EdgeKey, MeshAdjacency};
use super::cross_field::CrossField;
use super::features::SharpEdges;

/// A traced curve: sequence of points on the surface.
#[derive(Clone, Debug)]
pub struct TracedCurve {
    /// Positions along the curve.
    pub points: Vec<Vec3>,
    /// Face indices the curve passes through.
    pub face_sequence: Vec<usize>,
    /// Whether this curve is a separatrix (from a singularity).
    pub is_separatrix: bool,
    /// Whether this curve is a sharp feature edge chain.
    pub is_feature: bool,
}

/// Result of tracing: all curves that form patch boundaries.
#[derive(Clone, Debug)]
pub struct TraceResult {
    pub curves: Vec<TracedCurve>,
    /// Intersection points where curves meet (node positions).
    pub nodes: Vec<Vec3>,
    /// For each node, indices of curves that pass through it.
    pub node_curves: Vec<Vec<usize>>,
}

/// Trace field-aligned curves to create a patch decomposition skeleton.
pub fn trace_field_curves(
    geo: &Geometry,
    adj: &MeshAdjacency,
    field: &CrossField,
    sharp: &SharpEdges,
) -> TraceResult {
    let mut curves = Vec::new();
    let mut nodes = Vec::new();
    let mut node_curves: Vec<Vec<usize>> = Vec::new();

    // Step 1: Add sharp feature curves as patch boundaries
    let feature_curves = trace_sharp_features(geo, adj, sharp);
    for fc in feature_curves {
        let ci = curves.len();
        // Track endpoints as nodes
        if !fc.points.is_empty() {
            add_node(&fc.points[0], ci, &mut nodes, &mut node_curves);
            if fc.points.len() > 1 {
                add_node(fc.points.last().unwrap(), ci, &mut nodes, &mut node_curves);
            }
        }
        curves.push(fc);
    }

    // Step 2: Trace separatrices from singularities
    for &vi in &field.singular_vertices {
        let pos = geo.point_pos(PointHandle::from_index(vi));
        let ring = &adj.point_faces[vi];
        if ring.is_empty() {
            continue;
        }

        // Trace 4 separatrices from each singularity (one per cross-field arm)
        for arm in 0..4 {
            let start_face = ring[0];
            let base_dir = field.direction(start_face);
            let n = field.normals[start_face];
            let secondary = n.cross(base_dir).normalize_or_zero();

            let dir = match arm {
                0 => base_dir,
                1 => secondary,
                2 => -base_dir,
                3 => -secondary,
                _ => unreachable!(),
            };

            let curve = trace_streamline(geo, adj, field, pos, start_face, dir, sharp);
            if curve.points.len() >= 2 {
                let ci = curves.len();
                add_node(&curve.points[0], ci, &mut nodes, &mut node_curves);
                add_node(
                    curve.points.last().unwrap(),
                    ci,
                    &mut nodes,
                    &mut node_curves,
                );
                curves.push(curve);
            }
        }
    }

    // Step 3: If too few curves, add sampled streamlines
    if curves.len() < 4 {
        let extra = sample_additional_curves(geo, adj, field, sharp, &curves);
        for ec in extra {
            let ci = curves.len();
            if !ec.points.is_empty() {
                add_node(&ec.points[0], ci, &mut nodes, &mut node_curves);
                if ec.points.len() > 1 {
                    add_node(ec.points.last().unwrap(), ci, &mut nodes, &mut node_curves);
                }
            }
            curves.push(ec);
        }
    }

    TraceResult {
        curves,
        nodes,
        node_curves,
    }
}

/// Add a node at the given position, merging with nearby existing nodes.
fn add_node(
    pos: &Vec3,
    curve_idx: usize,
    nodes: &mut Vec<Vec3>,
    node_curves: &mut Vec<Vec<usize>>,
) {
    let merge_dist = 1e-4;
    for (ni, np) in nodes.iter().enumerate() {
        if (*np - *pos).length() < merge_dist {
            if !node_curves[ni].contains(&curve_idx) {
                node_curves[ni].push(curve_idx);
            }
            return;
        }
    }
    nodes.push(*pos);
    node_curves.push(vec![curve_idx]);
}

/// Trace sharp feature edges as connected polyline curves.
fn trace_sharp_features(
    geo: &Geometry,
    adj: &MeshAdjacency,
    sharp: &SharpEdges,
) -> Vec<TracedCurve> {
    let mut curves = Vec::new();
    let mut visited_edges: HashSet<EdgeKey> = HashSet::new();

    // Build adjacency of sharp edges
    let mut sharp_adj: HashMap<usize, Vec<(usize, EdgeKey)>> = HashMap::new();
    for &ek in &sharp.edges {
        sharp_adj.entry(ek.0).or_default().push((ek.1, ek));
        sharp_adj.entry(ek.1).or_default().push((ek.0, ek));
    }

    // Start from corner points or any unvisited sharp point
    let mut start_points: Vec<usize> = sharp.corner_points.clone();
    // Also include any sharp point as potential start
    for &ek in &sharp.edges {
        if !start_points.contains(&ek.0) {
            start_points.push(ek.0);
        }
    }

    for &start in &start_points {
        if let Some(neighbors) = sharp_adj.get(&start) {
            for &(next, ek) in neighbors {
                if visited_edges.contains(&ek) {
                    continue;
                }

                // Trace a chain of sharp edges
                let mut chain_points = vec![geo.point_pos(PointHandle::from_index(start))];
                let mut chain_faces = Vec::new();
                let mut current = start;
                let mut next_pt = next;

                loop {
                    let ek = EdgeKey::new(current, next_pt);
                    if visited_edges.contains(&ek) {
                        break;
                    }
                    visited_edges.insert(ek);

                    chain_points.push(geo.point_pos(PointHandle::from_index(next_pt)));

                    // Record a face this edge belongs to
                    if let Some(face_list) = adj.edge_faces.get(&ek) {
                        if let Some(&fi) = face_list.first() {
                            chain_faces.push(fi);
                        }
                    }

                    // Continue to next sharp edge from next_pt (if not a corner)
                    let is_corner = sharp.corner_points.contains(&next_pt);
                    if is_corner {
                        break;
                    }

                    let mut found_next = false;
                    if let Some(next_neighbors) = sharp_adj.get(&next_pt) {
                        for &(nn, ne) in next_neighbors {
                            if nn != current && !visited_edges.contains(&ne) {
                                current = next_pt;
                                next_pt = nn;
                                found_next = true;
                                break;
                            }
                        }
                    }
                    if !found_next {
                        break;
                    }
                }

                if chain_points.len() >= 2 {
                    curves.push(TracedCurve {
                        points: chain_points,
                        face_sequence: chain_faces,
                        is_separatrix: false,
                        is_feature: true,
                    });
                }
            }
        }
    }

    curves
}

/// Trace a single streamline from a starting position following the cross-field.
fn trace_streamline(
    geo: &Geometry,
    adj: &MeshAdjacency,
    field: &CrossField,
    start_pos: Vec3,
    start_face: usize,
    initial_dir: Vec3,
    sharp: &SharpEdges,
) -> TracedCurve {
    let max_steps = adj.faces.len() * 2;
    let step_size = super::adjacency::average_edge_length(geo) * 0.5;

    let mut points = vec![start_pos];
    let mut face_seq = vec![start_face];
    let mut current_pos = start_pos;
    let mut current_face = start_face;
    let mut current_dir = initial_dir;
    let mut visited_faces: HashSet<usize> = HashSet::new();
    visited_faces.insert(start_face);

    for _ in 0..max_steps {
        // Step forward
        let next_pos = current_pos + current_dir * step_size;

        // Find which face contains the next position (or the nearest face)
        let (next_face, snapped_pos) =
            find_containing_face(geo, adj, current_face, next_pos, &visited_faces);

        // Check if we hit a boundary or sharp edge
        let crossed_sharp = check_sharp_crossing(adj, sharp, current_face, next_face);

        points.push(snapped_pos);
        face_seq.push(next_face);

        if crossed_sharp || next_face == current_face {
            break;
        }

        // Update direction: align with the cross-field in the new face
        let field_dir = field.direction(next_face);
        let n = field.normals[next_face];
        let secondary = n.cross(field_dir).normalize_or_zero();

        // Pick the cross-field arm most aligned with current direction
        let candidates = [field_dir, secondary, -field_dir, -secondary];
        let best = candidates
            .iter()
            .max_by(|a, b| {
                a.dot(current_dir)
                    .partial_cmp(&b.dot(current_dir))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .copied()
            .unwrap_or(field_dir);

        current_pos = snapped_pos;
        current_face = next_face;
        current_dir = best;

        if visited_faces.contains(&next_face) {
            // We've looped back
            break;
        }
        visited_faces.insert(next_face);
    }

    TracedCurve {
        points,
        face_sequence: face_seq,
        is_separatrix: true,
        is_feature: false,
    }
}

/// Find the face containing or nearest to a given position, starting from a seed face.
fn find_containing_face(
    geo: &Geometry,
    adj: &MeshAdjacency,
    seed: usize,
    target: Vec3,
    _visited: &HashSet<usize>,
) -> (usize, Vec3) {
    // Simple approach: walk to the nearest adjacent face
    let mut best_face = seed;
    let mut best_dist = face_centroid_dist(geo, adj, seed, target);

    for &maybe_adj in &adj.faces[seed].adj {
        if let Some(fi) = maybe_adj {
            let dist = face_centroid_dist(geo, adj, fi, target);
            if dist < best_dist {
                best_dist = dist;
                best_face = fi;
            }
        }
    }

    // Project target onto the face plane
    let n = adj.face_normal(geo, best_face);
    let pts = &adj.faces[best_face].points;
    let fc = face_centroid(geo, pts);
    let projected = target - n * (target - fc).dot(n);

    (best_face, projected)
}

fn face_centroid(geo: &Geometry, pts: &[usize]) -> Vec3 {
    if pts.is_empty() {
        return Vec3::ZERO;
    }
    let sum: Vec3 = pts
        .iter()
        .map(|&pi| geo.point_pos(PointHandle::from_index(pi)))
        .sum();
    sum / pts.len() as f32
}

fn face_centroid_dist(geo: &Geometry, adj: &MeshAdjacency, fi: usize, target: Vec3) -> f32 {
    let c = face_centroid(geo, &adj.faces[fi].points);
    (c - target).length_squared()
}

/// Check if moving between two faces crosses a sharp edge.
fn check_sharp_crossing(adj: &MeshAdjacency, sharp: &SharpEdges, f0: usize, f1: usize) -> bool {
    if f0 == f1 {
        return false;
    }
    // Find the shared edge
    for &ek in &adj.faces[f0].edges {
        if adj.faces[f1].edges.contains(&ek) {
            if sharp.edges.contains(&ek) {
                return true;
            }
        }
    }
    false
}

/// Sample additional streamlines when separatrices alone don't provide enough coverage.
fn sample_additional_curves(
    geo: &Geometry,
    adj: &MeshAdjacency,
    field: &CrossField,
    sharp: &SharpEdges,
    _existing: &[TracedCurve],
) -> Vec<TracedCurve> {
    let mut extra = Vec::new();
    let num_faces = adj.faces.len();

    // Sample streamlines from face centroids at regular intervals
    let sample_interval = (num_faces / 8).max(1);

    for fi in (0..num_faces).step_by(sample_interval) {
        let centroid = face_centroid(geo, &adj.faces[fi].points);

        // Trace in both primary directions
        for arm in 0..2 {
            let dir = if arm == 0 {
                field.direction(fi)
            } else {
                field.secondary_direction(fi)
            };

            let curve = trace_streamline(geo, adj, field, centroid, fi, dir, sharp);
            if curve.points.len() >= 3 {
                extra.push(curve);
            }
        }
    }

    extra
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_grid_mesh() -> (Geometry, MeshAdjacency) {
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
                geo.add_face(&[pts[r * w + c], pts[r * w + c + 1], pts[(r + 1) * w + c + 1]]);
                geo.add_face(&[
                    pts[r * w + c],
                    pts[(r + 1) * w + c + 1],
                    pts[(r + 1) * w + c],
                ]);
            }
        }
        let adj = MeshAdjacency::build(&geo).unwrap();
        (geo, adj)
    }

    #[test]
    fn trace_produces_curves() {
        let (geo, adj) = make_grid_mesh();
        let sharp = super::super::features::detect_sharp_edges(&geo, &adj, 35.0);
        let field = super::super::cross_field::compute_cross_field(&geo, &adj, &sharp, 0.3, 5);
        let result = trace_field_curves(&geo, &adj, &field, &sharp);
        assert!(!result.curves.is_empty(), "should produce curves");
    }

    #[test]
    fn sharp_features_traced() {
        let (geo, adj) = make_grid_mesh();
        let sharp = super::super::features::detect_sharp_edges(&geo, &adj, 35.0);
        let feature_curves = trace_sharp_features(&geo, &adj, &sharp);
        // Boundary of a grid should produce feature curves
        assert!(!feature_curves.is_empty(), "should trace boundary features");
        for fc in &feature_curves {
            assert!(fc.is_feature);
            assert!(fc.points.len() >= 2);
        }
    }
}
