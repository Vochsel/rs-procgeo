// Triangle splitting for Boolean SOP — given a triangle and a set of cut edges
// lying on its surface, split it into smaller triangles using constrained
// incremental point insertion.

use glam::Vec3;

/// Epsilon for geometric comparisons.
const EPS: f32 = 1e-6;

// ---------------------------------------------------------------------------
// Public data types
// ---------------------------------------------------------------------------

/// A triangle fragment produced by splitting.
#[derive(Debug, Clone)]
pub struct TriFragment {
    pub v0: Vec3,
    pub v1: Vec3,
    pub v2: Vec3,
    /// Index of the original primitive this fragment came from.
    pub source_prim: usize,
    /// Mesh identifier: 0 = A, 1 = B.
    pub mesh_id: u8,
}

impl TriFragment {
    /// Geometric centroid of the fragment.
    #[inline]
    pub fn centroid(&self) -> Vec3 {
        (self.v0 + self.v1 + self.v2) / 3.0
    }

    /// Face normal (not necessarily unit length).
    #[inline]
    pub fn normal(&self) -> Vec3 {
        (self.v1 - self.v0).cross(self.v2 - self.v0)
    }

    /// Signed area magnitude (half the cross-product length).
    #[inline]
    pub fn area(&self) -> f32 {
        self.normal().length() * 0.5
    }
}

/// A cut edge lying on a triangle's surface.
#[derive(Debug, Clone)]
pub struct CutEdge {
    pub start: Vec3,
    pub end: Vec3,
}

// ---------------------------------------------------------------------------
// Barycentric coordinates
// ---------------------------------------------------------------------------

/// Compute barycentric coordinates `(u, v, w)` of point `p` with respect to
/// triangle `(a, b, c)`. The coordinates satisfy `u + v + w ≈ 1` and
/// `p ≈ u*a + v*b + w*c`. The point is inside (or on the boundary) if all
/// components are >= -EPS.
pub fn barycentric(p: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    let v0 = b - a;
    let v1 = c - a;
    let v2 = p - a;

    let d00 = v0.dot(v0);
    let d01 = v0.dot(v1);
    let d11 = v1.dot(v1);
    let d20 = v2.dot(v0);
    let d21 = v2.dot(v1);

    let denom = d00 * d11 - d01 * d01;
    if denom.abs() < EPS * EPS {
        // Degenerate triangle — return invalid coordinates.
        return Vec3::new(-1.0, -1.0, -1.0);
    }

    let inv_denom = 1.0 / denom;
    let v = (d11 * d20 - d01 * d21) * inv_denom;
    let w = (d00 * d21 - d01 * d20) * inv_denom;
    let u = 1.0 - v - w;

    Vec3::new(u, v, w)
}

/// Returns `true` if the barycentric coordinates indicate a point inside or on
/// the boundary of the triangle (all components >= -EPS).
#[inline]
fn bary_inside(bary: Vec3) -> bool {
    bary.x >= -EPS && bary.y >= -EPS && bary.z >= -EPS
}

// ---------------------------------------------------------------------------
// Point deduplication
// ---------------------------------------------------------------------------

/// Push `p` into `pts` only if no existing point is within `eps` distance.
/// Returns the index of `p` in `pts` (existing or newly inserted).
fn push_unique(pts: &mut Vec<Vec3>, p: Vec3, eps: f32) -> usize {
    let eps_sq = eps * eps;
    for (i, existing) in pts.iter().enumerate() {
        if (*existing - p).length_squared() < eps_sq {
            return i;
        }
    }
    let idx = pts.len();
    pts.push(p);
    idx
}

// ---------------------------------------------------------------------------
// 2D triangulation helpers
// ---------------------------------------------------------------------------

/// A 2D point used during triangulation.
#[derive(Debug, Clone, Copy)]
struct Pt2 {
    pub x: f32,
    pub y: f32,
}

/// A triangle in the triangulation, represented by indices into the point list.
#[derive(Debug, Clone, Copy)]
struct Tri2 {
    a: usize,
    b: usize,
    c: usize,
}

/// Signed area of a 2D triangle (positive = CCW).
#[inline]
fn signed_area_2d(a: Pt2, b: Pt2, c: Pt2) -> f32 {
    0.5 * ((b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y))
}

/// Barycentric coordinates in 2D.
fn bary_2d(p: Pt2, a: Pt2, b: Pt2, c: Pt2) -> (f32, f32, f32) {
    let area = signed_area_2d(a, b, c);
    if area.abs() < EPS * EPS {
        return (-1.0, -1.0, -1.0);
    }
    let inv = 1.0 / area;
    let u = signed_area_2d(p, b, c) * inv;
    let v = signed_area_2d(a, p, c) * inv;
    let w = 1.0 - u - v;
    (u, v, w)
}

/// Classify where point `p` lies relative to triangle `(a, b, c)`:
/// - `None` → outside
/// - `Some(PointLocation::Interior)` → strictly inside
/// - `Some(PointLocation::Edge(ei))` → on edge `ei` (0 = a-b, 1 = b-c, 2 = c-a)
#[derive(Debug, Clone, Copy, PartialEq)]
enum PointLocation {
    Interior,
    Edge(u8), // 0 = edge a→b, 1 = edge b→c, 2 = edge c→a
}

fn classify_point(p: Pt2, a: Pt2, b: Pt2, c: Pt2) -> Option<PointLocation> {
    let (u, v, w) = bary_2d(p, a, b, c);
    let edge_tol = EPS * 100.0; // slightly more tolerant for edge detection
    if u < -EPS || v < -EPS || w < -EPS {
        return None; // outside
    }
    // Check if on an edge (one barycentric coordinate near zero).
    // w near 0 → on edge a→b (edge 0)
    if w.abs() < edge_tol {
        return Some(PointLocation::Edge(0));
    }
    // u near 0 → on edge b→c (edge 1)
    if u.abs() < edge_tol {
        return Some(PointLocation::Edge(1));
    }
    // v near 0 → on edge c→a (edge 2)
    if v.abs() < edge_tol {
        return Some(PointLocation::Edge(2));
    }
    Some(PointLocation::Interior)
}

/// Incremental point insertion triangulation within an initial triangle.
///
/// `pts` must have at least 3 entries; the first 3 form the outer triangle.
/// Additional points are inserted one by one, splitting whichever triangle
/// contains the point.
fn triangulate_2d(pts: &[Pt2]) -> Vec<Tri2> {
    assert!(pts.len() >= 3);

    let mut tris: Vec<Tri2> = vec![Tri2 { a: 0, b: 1, c: 2 }];

    // Ensure the initial triangle is CCW.
    if signed_area_2d(pts[0], pts[1], pts[2]) < 0.0 {
        tris[0] = Tri2 { a: 0, b: 2, c: 1 };
    }

    for pi in 3..pts.len() {
        let p = pts[pi];

        // Find the triangle that contains this point.
        let mut found = None;
        for (ti, tri) in tris.iter().enumerate() {
            if let Some(loc) = classify_point(p, pts[tri.a], pts[tri.b], pts[tri.c]) {
                found = Some((ti, loc));
                break;
            }
        }

        let Some((ti, loc)) = found else {
            // Point is outside all current triangles — skip it. This can happen
            // if the point barely falls outside due to floating-point imprecision.
            continue;
        };

        match loc {
            PointLocation::Interior => {
                // Split the triangle into 3 sub-triangles.
                let old = tris[ti];
                tris[ti] = Tri2 {
                    a: old.a,
                    b: old.b,
                    c: pi,
                };
                tris.push(Tri2 {
                    a: old.b,
                    b: old.c,
                    c: pi,
                });
                tris.push(Tri2 {
                    a: old.c,
                    b: old.a,
                    c: pi,
                });
            }
            PointLocation::Edge(ei) => {
                // The point lies on an edge. Split the current triangle into 2.
                // Also find and split the adjacent triangle sharing that edge
                // (if it exists).
                let old = tris[ti];
                let (ea, eb) = match ei {
                    0 => (old.a, old.b), // edge a→b, opposite vertex c
                    1 => (old.b, old.c), // edge b→c, opposite vertex a
                    _ => (old.c, old.a), // edge c→a, opposite vertex b
                };
                let opp = match ei {
                    0 => old.c,
                    1 => old.a,
                    _ => old.b,
                };

                // Replace old triangle with two sub-triangles.
                tris[ti] = Tri2 {
                    a: ea,
                    b: pi,
                    c: opp,
                };
                tris.push(Tri2 {
                    a: pi,
                    b: eb,
                    c: opp,
                });

                // Find adjacent triangle sharing edge ea→eb (reversed: eb→ea).
                let mut adj_idx = None;
                for (tj, tri) in tris.iter().enumerate() {
                    if tj == ti || tj == tris.len() - 1 {
                        continue;
                    }
                    let verts = [tri.a, tri.b, tri.c];
                    let has_ea = verts.contains(&ea);
                    let has_eb = verts.contains(&eb);
                    if has_ea && has_eb {
                        adj_idx = Some(tj);
                        break;
                    }
                }

                if let Some(aj) = adj_idx {
                    let adj = tris[aj];
                    // Find the opposite vertex of the adjacent triangle.
                    let adj_opp = if adj.a != ea && adj.a != eb {
                        adj.a
                    } else if adj.b != ea && adj.b != eb {
                        adj.b
                    } else {
                        adj.c
                    };
                    // Split the adjacent triangle into 2.
                    tris[aj] = Tri2 {
                        a: ea,
                        b: pi,
                        c: adj_opp,
                    };
                    tris.push(Tri2 {
                        a: pi,
                        b: eb,
                        c: adj_opp,
                    });
                }
            }
        }
    }

    tris
}

// ---------------------------------------------------------------------------
// Main splitting function
// ---------------------------------------------------------------------------

/// Split a triangle along a set of cut edges, producing smaller triangle
/// fragments. Each fragment inherits `source_prim` and `mesh_id`.
///
/// If there are no cuts or no cut endpoints land on the triangle, the original
/// triangle is returned as a single fragment.
pub fn split_triangle(
    v0: Vec3,
    v1: Vec3,
    v2: Vec3,
    cuts: &[CutEdge],
    source_prim: usize,
    mesh_id: u8,
) -> Vec<TriFragment> {
    // 1. No cuts → return original.
    if cuts.is_empty() {
        return vec![TriFragment {
            v0,
            v1,
            v2,
            source_prim,
            mesh_id,
        }];
    }

    // 2. Collect unique points: start with the 3 triangle vertices.
    let mut pts: Vec<Vec3> = Vec::with_capacity(3 + cuts.len() * 2);
    pts.push(v0);
    pts.push(v1);
    pts.push(v2);

    // Add cut edge endpoints that lie on or near the triangle.
    for cut in cuts {
        for &p in &[cut.start, cut.end] {
            let bary = barycentric(p, v0, v1, v2);
            if bary_inside(bary) {
                push_unique(&mut pts, p, EPS);
            }
        }
    }

    // 4. If only the original 3 vertices, return original.
    if pts.len() <= 3 {
        return vec![TriFragment {
            v0,
            v1,
            v2,
            source_prim,
            mesh_id,
        }];
    }

    // 5. Project to 2D using the triangle's local coordinate system.
    let normal = (v1 - v0).cross(v2 - v0);
    let normal_len = normal.length();
    if normal_len < EPS {
        // Degenerate triangle.
        return vec![TriFragment {
            v0,
            v1,
            v2,
            source_prim,
            mesh_id,
        }];
    }
    let n_hat = normal / normal_len;
    let u_axis = (v1 - v0).normalize();
    let v_axis = n_hat.cross(u_axis);

    let project = |p: Vec3| -> Pt2 {
        let d = p - v0;
        Pt2 {
            x: d.dot(u_axis),
            y: d.dot(v_axis),
        }
    };

    let pts_2d: Vec<Pt2> = pts.iter().map(|&p| project(p)).collect();

    // 6. Triangulate.
    let tris = triangulate_2d(&pts_2d);

    // 7. Convert back to 3D, filter degenerates.
    let min_area = EPS * EPS;
    let mut fragments: Vec<TriFragment> = Vec::with_capacity(tris.len());

    for tri in &tris {
        let fv0 = pts[tri.a];
        let fv1 = pts[tri.b];
        let fv2 = pts[tri.c];

        // Filter degenerate triangles.
        let area = (fv1 - fv0).cross(fv2 - fv0).length() * 0.5;
        if area < min_area {
            continue;
        }

        fragments.push(TriFragment {
            v0: fv0,
            v1: fv1,
            v2: fv2,
            source_prim,
            mesh_id,
        });
    }

    // If all fragments were degenerate, return the original triangle.
    if fragments.is_empty() {
        return vec![TriFragment {
            v0,
            v1,
            v2,
            source_prim,
            mesh_id,
        }];
    }

    fragments
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: assert Vec3 approximately equal.
    fn v3_approx_eq(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < 1e-4
    }

    // ------------------------------------------------------------------
    // 1. No cuts → single fragment
    // ------------------------------------------------------------------

    #[test]
    fn no_cuts_returns_original() {
        let v0 = Vec3::new(0.0, 0.0, 0.0);
        let v1 = Vec3::new(1.0, 0.0, 0.0);
        let v2 = Vec3::new(0.5, 1.0, 0.0);

        let frags = split_triangle(v0, v1, v2, &[], 42, 0);

        assert_eq!(
            frags.len(),
            1,
            "no cuts should produce exactly one fragment"
        );
        assert!(v3_approx_eq(frags[0].v0, v0));
        assert!(v3_approx_eq(frags[0].v1, v1));
        assert!(v3_approx_eq(frags[0].v2, v2));
        assert_eq!(frags[0].source_prim, 42);
        assert_eq!(frags[0].mesh_id, 0);
    }

    // ------------------------------------------------------------------
    // 2. Single cut through interior → at least 3 fragments
    // ------------------------------------------------------------------

    #[test]
    fn single_cut_splits_triangle() {
        let v0 = Vec3::new(0.0, 0.0, 0.0);
        let v1 = Vec3::new(2.0, 0.0, 0.0);
        let v2 = Vec3::new(1.0, 2.0, 0.0);

        // A cut that goes through the interior of the triangle.
        let cut = CutEdge {
            start: Vec3::new(0.5, 0.5, 0.0),
            end: Vec3::new(1.5, 0.5, 0.0),
        };

        let frags = split_triangle(v0, v1, v2, &[cut], 7, 1);

        assert!(
            frags.len() >= 3,
            "a single interior cut should produce at least 3 fragments, got {}",
            frags.len()
        );

        // All fragments should preserve source_prim and mesh_id.
        for f in &frags {
            assert_eq!(f.source_prim, 7);
            assert_eq!(f.mesh_id, 1);
            assert!(f.area() > 0.0, "fragment should have positive area");
        }

        // The total area of fragments should approximately equal the original
        // triangle area.
        let original_area = (v1 - v0).cross(v2 - v0).length() * 0.5;
        let frag_area: f32 = frags.iter().map(|f| f.area()).sum();
        assert!(
            (frag_area - original_area).abs() < 1e-3,
            "total fragment area ({frag_area}) should match original ({original_area})"
        );

        println!("Single cut produced {} fragments", frags.len());
    }

    // ------------------------------------------------------------------
    // 3. Barycentric at vertices
    // ------------------------------------------------------------------

    #[test]
    fn barycentric_at_vertices() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(1.0, 0.0, 0.0);
        let c = Vec3::new(0.5, 1.0, 0.0);

        let ba = barycentric(a, a, b, c);
        assert!(
            v3_approx_eq(ba, Vec3::new(1.0, 0.0, 0.0)),
            "barycentric(a, a, b, c) should be (1,0,0), got {ba:?}"
        );

        let bb = barycentric(b, a, b, c);
        assert!(
            v3_approx_eq(bb, Vec3::new(0.0, 1.0, 0.0)),
            "barycentric(b, a, b, c) should be (0,1,0), got {bb:?}"
        );

        let bc = barycentric(c, a, b, c);
        assert!(
            v3_approx_eq(bc, Vec3::new(0.0, 0.0, 1.0)),
            "barycentric(c, a, b, c) should be (0,0,1), got {bc:?}"
        );

        // Centroid should be roughly (1/3, 1/3, 1/3).
        let centroid = (a + b + c) / 3.0;
        let bcentroid = barycentric(centroid, a, b, c);
        let expected = Vec3::new(1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0);
        assert!(
            v3_approx_eq(bcentroid, expected),
            "barycentric of centroid should be ~(1/3,1/3,1/3), got {bcentroid:?}"
        );
    }

    // ------------------------------------------------------------------
    // 4. Cut on edge
    // ------------------------------------------------------------------

    #[test]
    fn cut_on_edge() {
        let v0 = Vec3::new(0.0, 0.0, 0.0);
        let v1 = Vec3::new(2.0, 0.0, 0.0);
        let v2 = Vec3::new(1.0, 2.0, 0.0);

        // Cut endpoints lie exactly on triangle edges.
        let cut = CutEdge {
            start: Vec3::new(1.0, 0.0, 0.0), // midpoint of edge v0→v1
            end: Vec3::new(0.5, 1.0, 0.0),   // midpoint of edge v0→v2
        };

        let frags = split_triangle(v0, v1, v2, &[cut], 5, 0);

        assert!(
            frags.len() >= 2,
            "edge cut should produce at least 2 fragments, got {}",
            frags.len()
        );

        // Verify total area is preserved.
        let original_area = (v1 - v0).cross(v2 - v0).length() * 0.5;
        let frag_area: f32 = frags.iter().map(|f| f.area()).sum();
        assert!(
            (frag_area - original_area).abs() < 1e-3,
            "total fragment area ({frag_area}) should match original ({original_area})"
        );

        println!("Edge cut produced {} fragments", frags.len());
    }

    // ------------------------------------------------------------------
    // 5. Multiple cuts
    // ------------------------------------------------------------------

    #[test]
    fn multiple_cuts() {
        let v0 = Vec3::new(0.0, 0.0, 0.0);
        let v1 = Vec3::new(3.0, 0.0, 0.0);
        let v2 = Vec3::new(1.5, 3.0, 0.0);

        let cuts = vec![
            CutEdge {
                start: Vec3::new(0.75, 0.75, 0.0),
                end: Vec3::new(2.25, 0.75, 0.0),
            },
            CutEdge {
                start: Vec3::new(1.0, 1.5, 0.0),
                end: Vec3::new(2.0, 1.5, 0.0),
            },
        ];

        let frags = split_triangle(v0, v1, v2, &cuts, 10, 1);

        // Two interior cuts with 4 cut endpoints (all inside) should produce
        // more fragments than a single cut.
        assert!(
            frags.len() >= 4,
            "two interior cuts should produce many fragments, got {}",
            frags.len()
        );

        // Verify total area is preserved.
        let original_area = (v1 - v0).cross(v2 - v0).length() * 0.5;
        let frag_area: f32 = frags.iter().map(|f| f.area()).sum();
        assert!(
            (frag_area - original_area).abs() < 1e-2,
            "total fragment area ({frag_area}) should match original ({original_area})"
        );

        // All fragments should have correct metadata.
        for f in &frags {
            assert_eq!(f.source_prim, 10);
            assert_eq!(f.mesh_id, 1);
        }

        println!("Multiple cuts produced {} fragments", frags.len());
    }

    // ------------------------------------------------------------------
    // 6. Fragment methods
    // ------------------------------------------------------------------

    #[test]
    fn fragment_centroid_and_normal() {
        let frag = TriFragment {
            v0: Vec3::new(0.0, 0.0, 0.0),
            v1: Vec3::new(1.0, 0.0, 0.0),
            v2: Vec3::new(0.0, 1.0, 0.0),
            source_prim: 0,
            mesh_id: 0,
        };

        let c = frag.centroid();
        let expected_centroid = Vec3::new(1.0 / 3.0, 1.0 / 3.0, 0.0);
        assert!(
            v3_approx_eq(c, expected_centroid),
            "centroid should be ~(1/3, 1/3, 0), got {c:?}"
        );

        let n = frag.normal();
        // For a CCW triangle in XY plane, normal should point in +Z.
        assert!(
            n.z > 0.0,
            "normal should point in +Z for XY plane triangle, got {n:?}"
        );

        let area = frag.area();
        assert!(
            (area - 0.5).abs() < 1e-4,
            "area of unit right triangle should be 0.5, got {area}"
        );
    }

    // ------------------------------------------------------------------
    // 7. Cut endpoints outside triangle are ignored
    // ------------------------------------------------------------------

    #[test]
    fn cut_outside_triangle_ignored() {
        let v0 = Vec3::new(0.0, 0.0, 0.0);
        let v1 = Vec3::new(1.0, 0.0, 0.0);
        let v2 = Vec3::new(0.5, 1.0, 0.0);

        // Both endpoints are far outside the triangle.
        let cut = CutEdge {
            start: Vec3::new(10.0, 10.0, 0.0),
            end: Vec3::new(20.0, 20.0, 0.0),
        };

        let frags = split_triangle(v0, v1, v2, &[cut], 0, 0);
        assert_eq!(
            frags.len(),
            1,
            "cuts outside the triangle should be ignored"
        );
    }
}
