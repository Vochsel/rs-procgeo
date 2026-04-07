// De-triangulation for Boolean SOP — merge coplanar triangle fragments back
// into polygons by removing shared interior edges.

use std::collections::HashMap;

use glam::Vec3;

use super::splitting::TriFragment;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Controls which fragment groups are merged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetriMode {
    /// Merge all coplanar fragment groups back into polygons.
    All,
    /// Only merge groups that were NOT cut (i.e. the group has exactly the same
    /// number of fragments as the original primitive's triangulation). Cut
    /// groups are kept as individual triangles.
    OnlyUnchanged,
    /// No merging — every fragment becomes a 3-vertex polygon.
    None,
}

/// A polygon produced by de-triangulation.
#[derive(Debug, Clone)]
pub struct Polygon {
    /// Ordered boundary vertices.
    pub vertices: Vec<Vec3>,
    /// Index of the original source primitive.
    pub source_prim: usize,
    /// Mesh identifier (0 = A, 1 = B).
    pub mesh_id: u8,
}

// ---------------------------------------------------------------------------
// Quantised edge key
// ---------------------------------------------------------------------------

/// Quantise a `Vec3` to an `(i64, i64, i64)` key for hashing. Multiplying by
/// 1e5 gives ~10-micron resolution which is plenty for edge deduplication.
#[inline]
fn quantize(v: Vec3) -> (i64, i64, i64) {
    let s = 1e5_f32;
    (
        (v.x * s).round() as i64,
        (v.y * s).round() as i64,
        (v.z * s).round() as i64,
    )
}

type QVert = (i64, i64, i64);

/// A directed edge key. We always store the smaller quantised vertex first so
/// that (A→B) and (B→A) map to the same canonical key with a direction flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct EdgeKey {
    lo: QVert,
    hi: QVert,
}

fn edge_key(a: QVert, b: QVert) -> EdgeKey {
    if a <= b {
        EdgeKey { lo: a, hi: b }
    } else {
        EdgeKey { lo: b, hi: a }
    }
}

// ---------------------------------------------------------------------------
// Boundary extraction
// ---------------------------------------------------------------------------

/// Given a set of coplanar triangle fragments, extract the boundary polygon by:
/// 1. Collecting all directed half-edges.
/// 2. Removing edges that appear twice (interior edges shared by two triangles).
/// 3. Chaining the remaining boundary edges into a loop.
///
/// Returns `None` if the boundary cannot be formed into a single closed loop.
fn extract_boundary(fragments: &[&TriFragment]) -> Option<Vec<Vec3>> {
    // Count occurrences of each canonical edge. An edge shared by two
    // adjacent triangles appears twice and is interior.
    let mut edge_count: HashMap<EdgeKey, u32> = HashMap::new();

    // Also maintain a map from quantised start vertex to (quantised end, real end)
    // for boundary edges. We need the directed half-edges so we walk them in
    // order.
    struct HalfEdge {
        q_end: QVert,
        real_start: Vec3,
        real_end: Vec3,
    }

    let mut half_edges: Vec<(QVert, HalfEdge)> = Vec::new();

    for frag in fragments {
        let verts = [frag.v0, frag.v1, frag.v2];
        let qverts: [QVert; 3] = [quantize(verts[0]), quantize(verts[1]), quantize(verts[2])];

        for i in 0..3 {
            let j = (i + 1) % 3;
            let key = edge_key(qverts[i], qverts[j]);
            *edge_count.entry(key).or_insert(0) += 1;

            half_edges.push((
                qverts[i],
                HalfEdge {
                    q_end: qverts[j],
                    real_start: verts[i],
                    real_end: verts[j],
                },
            ));
        }
    }

    // Build adjacency map for boundary half-edges only (those whose canonical
    // edge appears exactly once).
    let mut adjacency: HashMap<QVert, (QVert, Vec3, Vec3)> = HashMap::new();

    for (q_start, he) in &half_edges {
        let key = edge_key(*q_start, he.q_end);
        if edge_count.get(&key).copied().unwrap_or(0) == 1 {
            adjacency.insert(*q_start, (he.q_end, he.real_start, he.real_end));
        }
    }

    if adjacency.is_empty() {
        return None;
    }

    // Walk the boundary loop starting from an arbitrary edge.
    let &start = adjacency.keys().next()?;
    let mut loop_verts: Vec<Vec3> = Vec::new();
    let mut current = start;

    loop {
        let (next, real_start, _real_end) = adjacency.get(&current)?;
        loop_verts.push(*real_start);
        current = *next;
        if current == start {
            break;
        }
        // Safety: prevent infinite loops on malformed input.
        if loop_verts.len() > adjacency.len() + 1 {
            return None;
        }
    }

    if loop_verts.len() < 3 {
        return None;
    }

    Some(loop_verts)
}

// ---------------------------------------------------------------------------
// Coplanarity check
// ---------------------------------------------------------------------------

/// Returns `true` if all fragments in the group are coplanar within the given
/// threshold. We compare each fragment's unit normal to the first fragment's
/// normal; the cross-product magnitude must be below `flat_threshold`.
fn is_coplanar(fragments: &[&TriFragment], flat_threshold: f32) -> bool {
    if fragments.len() <= 1 {
        return true;
    }

    let n0 = fragments[0].normal();
    let len0 = n0.length();
    if len0 < 1e-10 {
        return true; // degenerate first triangle
    }
    let n0 = n0 / len0;

    for frag in &fragments[1..] {
        let ni = frag.normal();
        let leni = ni.length();
        if leni < 1e-10 {
            continue; // skip degenerate
        }
        let ni = ni / leni;
        if n0.cross(ni).length() > flat_threshold {
            return false;
        }
    }

    true
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Merge coplanar triangle fragments back into polygons.
///
/// * `mode` controls which groups get merged.
/// * `flat_threshold` is the maximum cross-product magnitude between normals
///   for two fragments to be considered coplanar (a good default is 1e-4).
pub fn detriangulate(
    fragments: &[TriFragment],
    mode: DetriMode,
    flat_threshold: f32,
) -> Vec<Polygon> {
    if mode == DetriMode::None {
        // Each fragment becomes a standalone 3-vertex polygon.
        return fragments
            .iter()
            .map(|f| Polygon {
                vertices: vec![f.v0, f.v1, f.v2],
                source_prim: f.source_prim,
                mesh_id: f.mesh_id,
            })
            .collect();
    }

    // Group fragments by (mesh_id, source_prim).
    let mut groups: HashMap<(u8, usize), Vec<usize>> = HashMap::new();
    for (i, f) in fragments.iter().enumerate() {
        groups.entry((f.mesh_id, f.source_prim)).or_default().push(i);
    }

    let mut polygons: Vec<Polygon> = Vec::new();

    for ((_mesh_id, _source_prim), indices) in &groups {
        let group_frags: Vec<&TriFragment> = indices.iter().map(|&i| &fragments[i]).collect();
        let first = group_frags[0];

        // In OnlyUnchanged mode, skip merging for groups that were cut (more
        // than one fragment from the same source primitive means it was split).
        let should_merge = match mode {
            DetriMode::All => true,
            DetriMode::OnlyUnchanged => group_frags.len() == 1,
            DetriMode::None => unreachable!(),
        };

        if !should_merge || group_frags.len() == 1 {
            // Emit each fragment as a triangle polygon.
            for frag in &group_frags {
                polygons.push(Polygon {
                    vertices: vec![frag.v0, frag.v1, frag.v2],
                    source_prim: frag.source_prim,
                    mesh_id: frag.mesh_id,
                });
            }
            continue;
        }

        // Check coplanarity.
        if !is_coplanar(&group_frags, flat_threshold) {
            // Not coplanar — keep as individual triangles.
            for frag in &group_frags {
                polygons.push(Polygon {
                    vertices: vec![frag.v0, frag.v1, frag.v2],
                    source_prim: frag.source_prim,
                    mesh_id: frag.mesh_id,
                });
            }
            continue;
        }

        // Try to extract a merged boundary polygon.
        match extract_boundary(&group_frags) {
            Some(verts) => {
                polygons.push(Polygon {
                    vertices: verts,
                    source_prim: first.source_prim,
                    mesh_id: first.mesh_id,
                });
            }
            None => {
                // Fallback: keep as individual triangles.
                for frag in &group_frags {
                    polygons.push(Polygon {
                        vertices: vec![frag.v0, frag.v1, frag.v2],
                        source_prim: frag.source_prim,
                        mesh_id: frag.mesh_id,
                    });
                }
            }
        }
    }

    polygons
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn no_detriangulation_preserves_all() {
        let fragments = vec![
            TriFragment {
                v0: Vec3::new(0.0, 0.0, 0.0),
                v1: Vec3::new(1.0, 0.0, 0.0),
                v2: Vec3::new(0.0, 1.0, 0.0),
                source_prim: 0,
                mesh_id: 0,
            },
            TriFragment {
                v0: Vec3::new(1.0, 0.0, 0.0),
                v1: Vec3::new(1.0, 1.0, 0.0),
                v2: Vec3::new(0.0, 1.0, 0.0),
                source_prim: 0,
                mesh_id: 0,
            },
        ];

        let polys = detriangulate(&fragments, DetriMode::None, 1e-4);

        assert_eq!(
            polys.len(),
            2,
            "DetriMode::None should produce one polygon per fragment, got {}",
            polys.len()
        );

        for poly in &polys {
            assert_eq!(
                poly.vertices.len(),
                3,
                "each polygon should have 3 vertices in None mode"
            );
        }
    }

    #[test]
    fn merge_two_coplanar_triangles() {
        // Two coplanar triangles forming a quad in the XY plane:
        //
        //   (0,1)----(1,1)
        //     | \      |
        //     |   \    |
        //     |     \  |
        //   (0,0)----(1,0)
        //
        // Triangle A: (0,0) (1,0) (0,1)
        // Triangle B: (1,0) (1,1) (0,1)
        let fragments = vec![
            TriFragment {
                v0: Vec3::new(0.0, 0.0, 0.0),
                v1: Vec3::new(1.0, 0.0, 0.0),
                v2: Vec3::new(0.0, 1.0, 0.0),
                source_prim: 0,
                mesh_id: 0,
            },
            TriFragment {
                v0: Vec3::new(1.0, 0.0, 0.0),
                v1: Vec3::new(1.0, 1.0, 0.0),
                v2: Vec3::new(0.0, 1.0, 0.0),
                source_prim: 0,
                mesh_id: 0,
            },
        ];

        let polys = detriangulate(&fragments, DetriMode::All, 1e-4);

        assert_eq!(
            polys.len(),
            1,
            "two coplanar tris sharing an edge should merge into 1 polygon, got {}",
            polys.len()
        );

        assert_eq!(
            polys[0].vertices.len(),
            4,
            "merged quad should have 4 vertices, got {}",
            polys[0].vertices.len()
        );

        assert_eq!(polys[0].source_prim, 0);
        assert_eq!(polys[0].mesh_id, 0);
    }
}
