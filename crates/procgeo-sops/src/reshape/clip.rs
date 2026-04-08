use std::collections::{HashMap, HashSet};

use glam::Vec3;
use serde::{Deserialize, Serialize};

use procgeo_core::{Geometry, PointHandle, PolyType, PrimHandle, Primitive};

use crate::{Sop, SopError};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClipParams {
    /// A point on the clip plane.
    pub origin: Vec3,
    /// Plane normal (defines "above" direction).
    pub normal: Vec3,
    /// If true, keep faces on the "above" side; if false, keep "below" side.
    pub keep_above: bool,
    /// Seal the cut by generating a cap face on each clip-plane boundary loop.
    pub create_cap: bool,
}

impl Default for ClipParams {
    fn default() -> Self {
        ClipParams {
            origin: Vec3::ZERO,
            normal: Vec3::Y,
            keep_above: true,
            create_cap: false,
        }
    }
}

pub struct ClipSop;

/// Returns true if the point is on the "above" side (dot >= 0).
fn is_above(pos: Vec3, origin: Vec3, normal: Vec3) -> bool {
    (pos - origin).dot(normal) >= 0.0
}

/// Compute the intersection of edge A→B with the plane.
/// Returns the parameter t such that intersection = A + t*(B-A).
fn edge_plane_t(a: Vec3, b: Vec3, origin: Vec3, normal: Vec3) -> f32 {
    let denom = (b - a).dot(normal);
    if denom.abs() < 1e-10 {
        return 0.0;
    }
    (origin - a).dot(normal) / denom
}

/// Identifies a clipped vertex as either an original point or an intersection on an edge.
#[derive(Clone)]
enum ClipVertex {
    /// An original point that survived clipping.
    Original(usize),
    /// An intersection point on the edge between two original point indices.
    /// The edge key is stored with the smaller index first for consistent hashing.
    Intersection { edge: (usize, usize), pos: Vec3 },
}

/// Clip a single polygon against the plane. Returns the clipped polygon's vertices
/// with identity information for point deduplication, or None if entirely discarded.
fn clip_polygon(
    positions: &[Vec3],
    point_indices: &[usize],
    origin: Vec3,
    normal: Vec3,
    keep_above: bool,
) -> Option<Vec<ClipVertex>> {
    let n = positions.len();
    if n == 0 {
        return None;
    }

    // Classify each vertex
    let above: Vec<bool> = positions
        .iter()
        .map(|&p| is_above(p, origin, normal))
        .collect();

    let keep_side = keep_above;
    let all_kept = above.iter().all(|&a| a == keep_side);
    let all_discarded = above.iter().all(|&a| a != keep_side);

    if all_kept {
        return Some(
            point_indices
                .iter()
                .map(|&idx| ClipVertex::Original(idx))
                .collect(),
        );
    }
    if all_discarded {
        return None;
    }

    // Sutherland-Hodgman clipping against single plane
    let mut output: Vec<ClipVertex> = Vec::new();

    for i in 0..n {
        let current = positions[i];
        let next = positions[(i + 1) % n];
        let current_above = above[i];
        let next_above = above[(i + 1) % n];

        let current_kept = current_above == keep_side;
        let next_kept = next_above == keep_side;

        if current_kept {
            output.push(ClipVertex::Original(point_indices[i]));
        }

        // If we cross the boundary, add intersection point
        if current_kept != next_kept {
            let t = edge_plane_t(current, next, origin, normal);
            let intersection = current + t * (next - current);
            let idx_a = point_indices[i];
            let idx_b = point_indices[(i + 1) % n];
            let edge = if idx_a < idx_b {
                (idx_a, idx_b)
            } else {
                (idx_b, idx_a)
            };
            output.push(ClipVertex::Intersection {
                edge,
                pos: intersection,
            });
        }
    }

    if output.len() < 3 {
        return None;
    }

    Some(output)
}

fn edge_key(a: usize, b: usize) -> (usize, usize) {
    (a.min(b), a.max(b))
}

/// Compute a face normal via Newell's method.
fn face_normal(positions: &[Vec3]) -> Vec3 {
    let n = positions.len();
    let mut nx = 0.0_f32;
    let mut ny = 0.0_f32;
    let mut nz = 0.0_f32;
    for i in 0..n {
        let cur = positions[i];
        let next = positions[(i + 1) % n];
        nx += (cur.y - next.y) * (cur.z + next.z);
        ny += (cur.z - next.z) * (cur.x + next.x);
        nz += (cur.x - next.x) * (cur.y + next.y);
    }
    Vec3::new(nx, ny, nz).normalize_or_zero()
}

fn point_on_plane(pos: Vec3, origin: Vec3, normal: Vec3, eps: f32) -> bool {
    (pos - origin).dot(normal).abs() <= eps
}

/// Find boundary loops that lie on the current clip plane.
fn find_planar_boundary_loops(
    geo: &Geometry,
    origin: Vec3,
    normal: Vec3,
    eps: f32,
) -> Vec<Vec<PointHandle>> {
    let mut edge_count: HashMap<(usize, usize), usize> = HashMap::new();

    for prim_idx in 0..geo.num_prims() {
        let ph = PrimHandle::from_index(prim_idx);
        let prim = geo.prim(ph);
        let pts = geo.prim_points(ph);
        let edge_count_for_prim = match prim {
            Primitive::Polygon(poly) => match poly.poly_type {
                PolyType::Closed => pts.len(),
                PolyType::Open => pts.len().saturating_sub(1),
            },
        };

        for i in 0..edge_count_for_prim {
            let a = pts[i].index();
            let b = pts[(i + 1) % pts.len()].index();
            *edge_count.entry(edge_key(a, b)).or_insert(0) += 1;
        }
    }

    let mut planar_edges: Vec<(usize, usize)> = Vec::new();
    for ((a, b), count) in edge_count {
        if count != 1 {
            continue;
        }

        let pa = geo.point_pos(PointHandle::from_index(a));
        let pb = geo.point_pos(PointHandle::from_index(b));
        if point_on_plane(pa, origin, normal, eps) && point_on_plane(pb, origin, normal, eps) {
            planar_edges.push((a, b));
        }
    }

    if planar_edges.is_empty() {
        return Vec::new();
    }

    let mut adjacency: HashMap<usize, Vec<usize>> = HashMap::new();
    let mut unused: HashSet<(usize, usize)> = HashSet::new();
    for &(a, b) in &planar_edges {
        adjacency.entry(a).or_default().push(b);
        adjacency.entry(b).or_default().push(a);
        unused.insert(edge_key(a, b));
    }

    let mut loops: Vec<Vec<PointHandle>> = Vec::new();
    for &(start, next) in &planar_edges {
        let start_edge = edge_key(start, next);
        if !unused.remove(&start_edge) {
            continue;
        }

        let mut loop_pts = vec![start, next];
        let mut prev = start;
        let mut current = next;
        let mut closed = false;

        for _ in 0..=planar_edges.len() {
            let Some(neighbors) = adjacency.get(&current) else {
                break;
            };

            let Some(candidate) = neighbors.iter().copied().find(|&neighbor| {
                neighbor != prev && unused.contains(&edge_key(current, neighbor))
            }) else {
                break;
            };

            unused.remove(&edge_key(current, candidate));
            prev = current;
            current = candidate;

            if current == start {
                closed = true;
                break;
            }

            loop_pts.push(current);
        }

        if closed && loop_pts.len() >= 3 {
            loops.push(
                loop_pts
                    .into_iter()
                    .map(PointHandle::from_index)
                    .collect::<Vec<_>>(),
            );
        }
    }

    loops
}

impl Sop for ClipSop {
    type Params = ClipParams;

    fn name(&self) -> &'static str {
        "clip"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 1)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;
        let geo = inputs[0];
        let mut out = Geometry::new();

        let normal = params.normal.normalize_or_zero();

        // Cache original surviving points by their source index
        let mut orig_point_cache: HashMap<usize, PointHandle> = HashMap::new();
        // Cache intersection points by their edge (sorted pair of source indices)
        let mut edge_point_cache: HashMap<(usize, usize), PointHandle> = HashMap::new();

        for prim_idx in 0..geo.num_prims() {
            let ph = PrimHandle::from_index(prim_idx);
            let prim = geo.prim(ph);

            match prim {
                Primitive::Polygon(poly) => {
                    let orig_pts = geo.prim_points(ph);
                    let positions: Vec<Vec3> = orig_pts.iter().map(|&h| geo.point_pos(h)).collect();
                    let point_indices: Vec<usize> = orig_pts.iter().map(|h| h.index()).collect();

                    if let Some(clipped) = clip_polygon(
                        &positions,
                        &point_indices,
                        params.origin,
                        normal,
                        params.keep_above,
                    ) {
                        let new_handles: Vec<PointHandle> = clipped
                            .iter()
                            .map(|cv| match cv {
                                ClipVertex::Original(idx) => {
                                    *orig_point_cache.entry(*idx).or_insert_with(|| {
                                        out.add_point(geo.point_pos(PointHandle::from_index(*idx)))
                                    })
                                }
                                ClipVertex::Intersection { edge, pos } => *edge_point_cache
                                    .entry(*edge)
                                    .or_insert_with(|| out.add_point(*pos)),
                            })
                            .collect();

                        match poly.poly_type {
                            PolyType::Closed => {
                                out.add_face(&new_handles);
                            }
                            PolyType::Open => {
                                out.add_polyline(&new_handles);
                            }
                        }
                    }
                }
            }
        }

        if params.create_cap && normal.length_squared() > 0.0 {
            let desired_cap_normal = if params.keep_above { -normal } else { normal };
            for mut loop_handles in find_planar_boundary_loops(&out, params.origin, normal, 1e-5) {
                let positions: Vec<Vec3> = loop_handles.iter().map(|&h| out.point_pos(h)).collect();
                if face_normal(&positions).dot(desired_cap_normal) < 0.0 {
                    loop_handles.reverse();
                }
                out.add_face(&loop_handles);
            }
        }

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creation::box_sop::{BoxParams, BoxSop};
    use crate::{GeometryExt, generate};
    use glam::Vec3;

    fn make_box() -> Geometry {
        generate(&BoxSop, &BoxParams::default()).unwrap()
    }

    #[test]
    fn clip_box_at_origin() {
        // Clip box at y=0, keep above.
        // Top face (y=+0.5): all above → kept as-is (1 face)
        // Bottom face (y=-0.5): all below → discarded
        // 4 side faces: each crosses y=0 → clipped (4 faces)
        // Total: 5 faces
        let box_geo = make_box();
        let params = ClipParams {
            origin: Vec3::ZERO,
            normal: Vec3::Y,
            keep_above: true,
            create_cap: false,
        };
        let result = box_geo.apply(&ClipSop, &params).unwrap();

        assert_eq!(
            result.num_prims(),
            5,
            "should have 5 faces after clipping at y=0"
        );
    }

    #[test]
    fn clip_all_above() {
        // Clip plane well below the box → all 6 faces kept
        let box_geo = make_box();
        let params = ClipParams {
            origin: Vec3::new(0.0, -10.0, 0.0),
            normal: Vec3::Y,
            keep_above: true,
            create_cap: false,
        };
        let result = box_geo.apply(&ClipSop, &params).unwrap();

        assert_eq!(result.num_prims(), 6, "all 6 faces should be kept");
    }

    #[test]
    fn clip_all_below() {
        // Clip plane well above the box, keep above → 0 faces
        let box_geo = make_box();
        let params = ClipParams {
            origin: Vec3::new(0.0, 10.0, 0.0),
            normal: Vec3::Y,
            keep_above: true,
            create_cap: false,
        };
        let result = box_geo.apply(&ClipSop, &params).unwrap();

        assert_eq!(result.num_prims(), 0, "no faces should be kept");
    }

    #[test]
    fn clip_shares_boundary_points() {
        // Adjacent faces clipped by the same plane must share boundary points.
        // A box clipped at y=0 has 4 side faces that each cross y=0, producing
        // 4 intersection points (one per vertical edge). These must be shared,
        // not duplicated per face. The top face (4 pts) + 4 boundary pts = 8 unique points.
        let box_geo = make_box();
        let params = ClipParams {
            origin: Vec3::ZERO,
            normal: Vec3::Y,
            keep_above: true,
            create_cap: false,
        };
        let result = box_geo.apply(&ClipSop, &params).unwrap();

        // Box has 4 top corners (kept) + 4 edge intersections at y=0 = 8 unique points
        assert_eq!(
            result.num_points(),
            8,
            "clipped faces must share boundary points (got {} unique, expected 8)",
            result.num_points()
        );
    }

    #[test]
    fn clip_cap_closes_cut_surface() {
        let box_geo = make_box();
        let params = ClipParams {
            origin: Vec3::ZERO,
            normal: Vec3::Y,
            keep_above: true,
            create_cap: true,
        };
        let result = box_geo.apply(&ClipSop, &params).unwrap();

        assert_eq!(
            result.num_prims(),
            6,
            "capped half-box should have 6 faces, got {}",
            result.num_prims()
        );
        assert_eq!(
            result.num_points(),
            8,
            "cap should reuse the cut-loop points, got {} points",
            result.num_points()
        );
    }

    #[test]
    fn clip_cap_faces_outward() {
        let box_geo = make_box();
        let params = ClipParams {
            origin: Vec3::ZERO,
            normal: Vec3::Y,
            keep_above: true,
            create_cap: true,
        };
        let result = box_geo.apply(&ClipSop, &params).unwrap();

        let mut cap_normals = Vec::new();
        for prim_idx in 0..result.num_prims() {
            let pts = result.prim_points(PrimHandle::from_index(prim_idx));
            let positions: Vec<Vec3> = pts.iter().map(|&h| result.point_pos(h)).collect();
            if positions.iter().all(|p| p.y.abs() < 1e-5) {
                cap_normals.push(face_normal(&positions));
            }
        }

        assert_eq!(
            cap_normals.len(),
            1,
            "expected exactly one cap face, got {}",
            cap_normals.len()
        );
        assert!(
            cap_normals[0].y < -0.9,
            "top-half cap should face downward, got normal {:?}",
            cap_normals[0]
        );
    }
}
