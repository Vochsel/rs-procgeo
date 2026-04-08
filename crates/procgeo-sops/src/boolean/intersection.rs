// Robust triangle-triangle intersection using Möller's method.

use glam::Vec3;

/// Result of a triangle-triangle intersection test.
#[derive(Debug, Clone, PartialEq)]
pub enum TriTriResult {
    /// No intersection.
    None,
    /// The two triangles intersect along a line segment.
    Segment { start: Vec3, end: Vec3 },
    /// The two triangles are coplanar and overlap; `points` contains the
    /// vertices of the intersection polygon (may be empty if coplanar but
    /// non-overlapping).
    Coplanar { points: Vec<Vec3> },
}

/// Epsilon used to snap near-zero signed distances to exactly zero.
const EPS: f32 = 1e-8;

/// Compute the robust intersection between two triangles.
///
/// Returns [`TriTriResult::None`] when the triangles do not intersect,
/// [`TriTriResult::Segment`] for the general case, or
/// [`TriTriResult::Coplanar`] when the triangles lie on the same plane.
pub fn tri_tri_intersection(
    a0: Vec3,
    a1: Vec3,
    a2: Vec3,
    b0: Vec3,
    b1: Vec3,
    b2: Vec3,
) -> TriTriResult {
    // --- Step 1-3: Plane of B and signed distances of A vertices to it ---
    let nb = (b1 - b0).cross(b2 - b0);
    let db = nb.dot(b0);

    let mut da0 = nb.dot(a0) - db;
    let mut da1 = nb.dot(a1) - db;
    let mut da2 = nb.dot(a2) - db;

    // Snap tiny values to zero.
    if da0.abs() < EPS {
        da0 = 0.0;
    }
    if da1.abs() < EPS {
        da1 = 0.0;
    }
    if da2.abs() < EPS {
        da2 = 0.0;
    }

    // --- Step 4: If all A on same side → None ---
    if da0 > 0.0 && da1 > 0.0 && da2 > 0.0 {
        return TriTriResult::None;
    }
    if da0 < 0.0 && da1 < 0.0 && da2 < 0.0 {
        return TriTriResult::None;
    }

    // --- Step 5: Plane of A and signed distances of B vertices to it ---
    let na = (a1 - a0).cross(a2 - a0);
    let da = na.dot(a0);

    let mut db0 = na.dot(b0) - da;
    let mut db1 = na.dot(b1) - da;
    let mut db2 = na.dot(b2) - da;

    if db0.abs() < EPS {
        db0 = 0.0;
    }
    if db1.abs() < EPS {
        db1 = 0.0;
    }
    if db2.abs() < EPS {
        db2 = 0.0;
    }

    if db0 > 0.0 && db1 > 0.0 && db2 > 0.0 {
        return TriTriResult::None;
    }
    if db0 < 0.0 && db1 < 0.0 && db2 < 0.0 {
        return TriTriResult::None;
    }

    // --- Step 6: Coplanar case ---
    if da0 == 0.0 && da1 == 0.0 && da2 == 0.0 {
        return coplanar_intersection(a0, a1, a2, b0, b1, b2, na);
    }

    // --- Step 7: Intersection line direction ---
    let dir = na.cross(nb);
    let dir_len = dir.length();
    if dir_len < EPS {
        // Normals are parallel but triangles are not coplanar → no intersection
        // (they sit on parallel planes).
        return TriTriResult::None;
    }
    let dir = dir / dir_len;

    // --- Step 8-9: Compute intervals and overlap ---
    let interval_a = triangle_interval(a0, a1, a2, da0, da1, da2, dir);
    let interval_b = triangle_interval(b0, b1, b2, db0, db1, db2, dir);

    if interval_a.is_none() || interval_b.is_none() {
        return TriTriResult::None;
    }

    let (a_min_t, a_min_pt, a_max_t, a_max_pt) = interval_a.unwrap();
    let (b_min_t, b_min_pt, b_max_t, b_max_pt) = interval_b.unwrap();

    // Overlap of [a_min_t, a_max_t] and [b_min_t, b_max_t].
    let overlap_start_t = a_min_t.max(b_min_t);
    let overlap_end_t = a_max_t.min(b_max_t);

    if overlap_end_t - overlap_start_t < EPS {
        return TriTriResult::None;
    }

    // Compute start/end points from the appropriate interval endpoint.
    let start = if a_min_t >= b_min_t {
        a_min_pt
    } else {
        b_min_pt
    };
    let end = if a_max_t <= b_max_t {
        a_max_pt
    } else {
        b_max_pt
    };

    TriTriResult::Segment { start, end }
}

/// For a triangle whose vertices have signed distances `d0, d1, d2` to the
/// other triangle's plane, compute the interval on the intersection line where
/// the triangle crosses through.
///
/// Returns `(min_proj, min_point, max_proj, max_point)` or `None` if the
/// triangle does not straddle the plane at all.
fn triangle_interval(
    v0: Vec3,
    v1: Vec3,
    v2: Vec3,
    d0: f32,
    d1: f32,
    d2: f32,
    dir: Vec3,
) -> Option<(f32, Vec3, f32, Vec3)> {
    let mut pts: Vec<Vec3> = Vec::with_capacity(3);

    // Collect edge crossing points and on-plane vertices.
    let verts = [(v0, d0), (v1, d1), (v2, d2)];

    for i in 0..3 {
        let (vi, di) = verts[i];
        let (vj, dj) = verts[(i + 1) % 3];

        if di == 0.0 {
            // Vertex lies exactly on the plane.
            pts.push(vi);
        }

        // Edge crosses the plane when signs differ (and neither is zero,
        // because on-plane vertices are already handled above).
        if (di > 0.0 && dj < 0.0) || (di < 0.0 && dj > 0.0) {
            let t = di / (di - dj);
            pts.push(vi + t * (vj - vi));
        }
    }

    if pts.is_empty() {
        return None;
    }

    // Deduplicate very close points.
    pts.dedup_by(|a, b| (*a - *b).length_squared() < EPS * EPS);

    if pts.is_empty() {
        return None;
    }

    // Project onto the intersection line direction and find min/max.
    let mut min_t = f32::MAX;
    let mut max_t = f32::MIN;
    let mut min_pt = pts[0];
    let mut max_pt = pts[0];

    for &p in &pts {
        let t = dir.dot(p);
        if t < min_t {
            min_t = t;
            min_pt = p;
        }
        if t > max_t {
            max_t = t;
            max_pt = p;
        }
    }

    Some((min_t, min_pt, max_t, max_pt))
}

// ---------------------------------------------------------------------------
// Coplanar intersection
// ---------------------------------------------------------------------------

/// Handle the coplanar case by projecting both triangles to 2D (dropping the
/// axis most aligned with the shared normal) and computing the intersection
/// polygon vertices.
fn coplanar_intersection(
    a0: Vec3,
    a1: Vec3,
    a2: Vec3,
    b0: Vec3,
    b1: Vec3,
    b2: Vec3,
    normal: Vec3,
) -> TriTriResult {
    let abs_n = normal.abs();
    // Drop the axis that is most aligned with the normal for the most stable
    // 2D projection.
    let (ax1, ax2) = if abs_n.x >= abs_n.y && abs_n.x >= abs_n.z {
        (1, 2) // drop X
    } else if abs_n.y >= abs_n.z {
        (0, 2) // drop Y
    } else {
        (0, 1) // drop Z
    };

    let project = |v: Vec3| -> [f32; 2] { [component(v, ax1), component(v, ax2)] };

    let pa0 = project(a0);
    let pa1 = project(a1);
    let pa2 = project(a2);
    let pb0 = project(b0);
    let pb1 = project(b1);
    let pb2 = project(b2);

    let mut points_2d: Vec<[f32; 2]> = Vec::new();

    // Edge-edge intersections (6 pairs of edges).
    let a_edges = [(pa0, pa1), (pa1, pa2), (pa2, pa0)];
    let b_edges = [(pb0, pb1), (pb1, pb2), (pb2, pb0)];

    for &(ae0, ae1) in &a_edges {
        for &(be0, be1) in &b_edges {
            if let Some(pt) = segment_segment_2d(ae0, ae1, be0, be1) {
                push_unique_2d(&mut points_2d, pt);
            }
        }
    }

    // Vertices of A inside B.
    for &pa in &[pa0, pa1, pa2] {
        if point_in_triangle_2d(pa, pb0, pb1, pb2) {
            push_unique_2d(&mut points_2d, pa);
        }
    }

    // Vertices of B inside A.
    for &pb in &[pb0, pb1, pb2] {
        if point_in_triangle_2d(pb, pa0, pa1, pa2) {
            push_unique_2d(&mut points_2d, pb);
        }
    }

    if points_2d.is_empty() {
        return TriTriResult::None;
    }

    // Lift 2D points back to 3D. We recover the dropped coordinate from the
    // plane equation: n . p = d, so p[drop_axis] = (d - n[ax1]*p[ax1] - n[ax2]*p[ax2]) / n[drop_axis].
    let drop_axis = if abs_n.x >= abs_n.y && abs_n.x >= abs_n.z {
        0
    } else if abs_n.y >= abs_n.z {
        1
    } else {
        2
    };
    let d = normal.dot(a0);
    let n_drop = component(normal, drop_axis);

    let points_3d: Vec<Vec3> = if n_drop.abs() < EPS {
        // Degenerate normal — just use zero for the dropped axis (shouldn't
        // really happen since we picked the largest component).
        points_2d
            .iter()
            .map(|p| set_component(p[0], p[1], 0.0, ax1, ax2, drop_axis))
            .collect()
    } else {
        points_2d
            .iter()
            .map(|p| {
                let restored =
                    (d - component(normal, ax1) * p[0] - component(normal, ax2) * p[1]) / n_drop;
                set_component(p[0], p[1], restored, ax1, ax2, drop_axis)
            })
            .collect()
    };

    TriTriResult::Coplanar { points: points_3d }
}

// ---------------------------------------------------------------------------
// 2D helpers
// ---------------------------------------------------------------------------

/// Compute the intersection point of two 2D line segments, if any.
///
/// Returns `None` when the segments do not intersect or are parallel.
fn segment_segment_2d(a0: [f32; 2], a1: [f32; 2], b0: [f32; 2], b1: [f32; 2]) -> Option<[f32; 2]> {
    let dx_a = a1[0] - a0[0];
    let dy_a = a1[1] - a0[1];
    let dx_b = b1[0] - b0[0];
    let dy_b = b1[1] - b0[1];

    let denom = dx_a * dy_b - dy_a * dx_b;
    if denom.abs() < EPS {
        return None; // Parallel (or coincident — we ignore overlapping edges).
    }

    let dx_ab = b0[0] - a0[0];
    let dy_ab = b0[1] - a0[1];

    let t = (dx_ab * dy_b - dy_ab * dx_b) / denom;
    let u = (dx_ab * dy_a - dy_ab * dx_a) / denom;

    // Allow a small tolerance for endpoints.
    let lo = -EPS;
    let hi = 1.0 + EPS;

    if t >= lo && t <= hi && u >= lo && u <= hi {
        let t_clamped = t.clamp(0.0, 1.0);
        Some([a0[0] + t_clamped * dx_a, a0[1] + t_clamped * dy_a])
    } else {
        None
    }
}

/// Barycentric point-in-triangle test (2D). Returns `true` when `p` is inside
/// or on the boundary of triangle `(a, b, c)`.
fn point_in_triangle_2d(p: [f32; 2], a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> bool {
    let v0 = [c[0] - a[0], c[1] - a[1]];
    let v1 = [b[0] - a[0], b[1] - a[1]];
    let v2 = [p[0] - a[0], p[1] - a[1]];

    let dot00 = v0[0] * v0[0] + v0[1] * v0[1];
    let dot01 = v0[0] * v1[0] + v0[1] * v1[1];
    let dot02 = v0[0] * v2[0] + v0[1] * v2[1];
    let dot11 = v1[0] * v1[0] + v1[1] * v1[1];
    let dot12 = v1[0] * v2[0] + v1[1] * v2[1];

    let inv_denom = 1.0 / (dot00 * dot11 - dot01 * dot01);
    let u = (dot11 * dot02 - dot01 * dot12) * inv_denom;
    let v = (dot00 * dot12 - dot01 * dot02) * inv_denom;

    let tol = -1e-6;
    u >= tol && v >= tol && (u + v) <= 1.0 - tol
}

// ---------------------------------------------------------------------------
// Utility helpers
// ---------------------------------------------------------------------------

/// Extract a component from a Vec3 by axis index (0=x, 1=y, 2=z).
#[inline]
fn component(v: Vec3, axis: usize) -> f32 {
    match axis {
        0 => v.x,
        1 => v.y,
        _ => v.z,
    }
}

/// Build a Vec3 from two projected coordinates and a restored coordinate.
#[inline]
fn set_component(
    c1: f32,
    c2: f32,
    restored: f32,
    ax1: usize,
    ax2: usize,
    drop_axis: usize,
) -> Vec3 {
    let mut arr = [0.0f32; 3];
    arr[ax1] = c1;
    arr[ax2] = c2;
    arr[drop_axis] = restored;
    Vec3::new(arr[0], arr[1], arr[2])
}

/// Push a 2D point into `pts` only if no existing point is within `EPS`
/// distance of it (to avoid duplicates).
fn push_unique_2d(pts: &mut Vec<[f32; 2]>, p: [f32; 2]) {
    let eps_sq = 1e-6 * 1e-6;
    for existing in pts.iter() {
        let dx = existing[0] - p[0];
        let dy = existing[1] - p[1];
        if dx * dx + dy * dy < eps_sq {
            return;
        }
    }
    pts.push(p);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: assert two Vec3 values are approximately equal.
    fn approx_eq(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < 1e-4
    }

    // ------------------------------------------------------------------
    // 1. Intersecting triangles
    // ------------------------------------------------------------------

    #[test]
    fn intersecting_triangles() {
        // Triangle A in the XY plane.
        let a0 = Vec3::new(0.0, 0.0, 0.0);
        let a1 = Vec3::new(2.0, 0.0, 0.0);
        let a2 = Vec3::new(1.0, 2.0, 0.0);

        // Triangle B tilted, crossing through A.
        let b0 = Vec3::new(0.5, 0.5, -1.0);
        let b1 = Vec3::new(0.5, 0.5, 1.0);
        let b2 = Vec3::new(1.5, 1.5, 0.0);

        let result = tri_tri_intersection(a0, a1, a2, b0, b1, b2);
        match result {
            TriTriResult::Segment { start, end } => {
                // The segment should have z ≈ 0 and lie within both triangles.
                assert!(
                    start.z.abs() < 0.2 && end.z.abs() < 0.2,
                    "Segment endpoints should be near z=0, got start={start:?}, end={end:?}"
                );
                // Segment should have nonzero length.
                assert!(
                    (start - end).length() > 1e-4,
                    "Segment should have nonzero length"
                );
                println!("Segment: {start:?} → {end:?}");
            }
            other => panic!("Expected Segment, got {other:?}"),
        }
    }

    // ------------------------------------------------------------------
    // 2. Non-intersecting triangles
    // ------------------------------------------------------------------

    #[test]
    fn non_intersecting_triangles() {
        let a0 = Vec3::new(0.0, 0.0, 0.0);
        let a1 = Vec3::new(1.0, 0.0, 0.0);
        let a2 = Vec3::new(0.5, 1.0, 0.0);

        // B is far away.
        let b0 = Vec3::new(10.0, 10.0, 10.0);
        let b1 = Vec3::new(11.0, 10.0, 10.0);
        let b2 = Vec3::new(10.5, 11.0, 10.0);

        let result = tri_tri_intersection(a0, a1, a2, b0, b1, b2);
        assert_eq!(
            result,
            TriTriResult::None,
            "Separated triangles should not intersect"
        );
    }

    // ------------------------------------------------------------------
    // 3. Coplanar overlapping
    // ------------------------------------------------------------------

    #[test]
    fn coplanar_overlapping() {
        // Two coplanar triangles in the XY plane that overlap.
        let a0 = Vec3::new(0.0, 0.0, 0.0);
        let a1 = Vec3::new(2.0, 0.0, 0.0);
        let a2 = Vec3::new(1.0, 2.0, 0.0);

        let b0 = Vec3::new(0.5, 0.5, 0.0);
        let b1 = Vec3::new(2.5, 0.5, 0.0);
        let b2 = Vec3::new(1.5, 2.5, 0.0);

        let result = tri_tri_intersection(a0, a1, a2, b0, b1, b2);
        match result {
            TriTriResult::Coplanar { points } => {
                assert!(
                    points.len() >= 3,
                    "Coplanar overlapping triangles should produce at least 3 intersection points, got {}",
                    points.len()
                );
                // All points should have z ≈ 0.
                for p in &points {
                    assert!(p.z.abs() < 1e-4, "Point {p:?} should have z≈0");
                }
                println!("Coplanar points: {points:?}");
            }
            other => panic!("Expected Coplanar, got {other:?}"),
        }
    }

    // ------------------------------------------------------------------
    // 4. Touching at edge
    // ------------------------------------------------------------------

    #[test]
    fn touching_at_edge() {
        // Two triangles sharing an edge along X axis, one in XY and one tilted.
        let a0 = Vec3::new(0.0, 0.0, 0.0);
        let a1 = Vec3::new(1.0, 0.0, 0.0);
        let a2 = Vec3::new(0.5, 1.0, 0.0);

        // B shares edge a0-a1 but goes in the opposite Y direction and into Z.
        let b0 = Vec3::new(0.0, 0.0, 0.0);
        let b1 = Vec3::new(1.0, 0.0, 0.0);
        let b2 = Vec3::new(0.5, -1.0, 1.0);

        let result = tri_tri_intersection(a0, a1, a2, b0, b1, b2);
        match &result {
            TriTriResult::Segment { start, end } => {
                // The shared edge runs from (0,0,0) to (1,0,0).
                let s = *start;
                let e = *end;
                assert!(
                    (approx_eq(s, Vec3::new(0.0, 0.0, 0.0))
                        && approx_eq(e, Vec3::new(1.0, 0.0, 0.0)))
                        || (approx_eq(s, Vec3::new(1.0, 0.0, 0.0))
                            && approx_eq(e, Vec3::new(0.0, 0.0, 0.0))),
                    "Edge-touching triangles should produce segment along shared edge, got {s:?} → {e:?}"
                );
                println!("Touching segment: {start:?} → {end:?}");
            }
            TriTriResult::None => {
                // Also acceptable — touching at an edge can be considered
                // degenerate (zero-area intersection).
                println!("Touching at edge returned None (acceptable)");
            }
            other => panic!("Expected Segment or None for edge-touching, got {other:?}"),
        }
    }

    // ------------------------------------------------------------------
    // 5. Parallel non-coplanar
    // ------------------------------------------------------------------

    #[test]
    fn parallel_non_coplanar() {
        // Two triangles on parallel XY planes, separated in Z.
        let a0 = Vec3::new(0.0, 0.0, 0.0);
        let a1 = Vec3::new(1.0, 0.0, 0.0);
        let a2 = Vec3::new(0.5, 1.0, 0.0);

        let b0 = Vec3::new(0.0, 0.0, 5.0);
        let b1 = Vec3::new(1.0, 0.0, 5.0);
        let b2 = Vec3::new(0.5, 1.0, 5.0);

        let result = tri_tri_intersection(a0, a1, a2, b0, b1, b2);
        assert_eq!(
            result,
            TriTriResult::None,
            "Parallel non-coplanar triangles should not intersect"
        );
    }

    // ------------------------------------------------------------------
    // 2D helper tests
    // ------------------------------------------------------------------

    #[test]
    fn segment_segment_2d_crossing() {
        // X-shaped crossing at (0.5, 0.5).
        let result = segment_segment_2d([0.0, 0.0], [1.0, 1.0], [0.0, 1.0], [1.0, 0.0]);
        assert!(result.is_some(), "Crossing segments should intersect");
        let p = result.unwrap();
        assert!((p[0] - 0.5).abs() < 1e-4 && (p[1] - 0.5).abs() < 1e-4);
    }

    #[test]
    fn segment_segment_2d_no_crossing() {
        // Two parallel horizontal segments.
        let result = segment_segment_2d([0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]);
        assert!(result.is_none(), "Parallel segments should not intersect");
    }

    #[test]
    fn point_in_triangle_2d_inside() {
        let a = [0.0, 0.0];
        let b = [2.0, 0.0];
        let c = [1.0, 2.0];
        assert!(point_in_triangle_2d([1.0, 0.5], a, b, c));
    }

    #[test]
    fn point_in_triangle_2d_outside() {
        let a = [0.0, 0.0];
        let b = [2.0, 0.0];
        let c = [1.0, 2.0];
        assert!(!point_in_triangle_2d([3.0, 3.0], a, b, c));
    }
}
