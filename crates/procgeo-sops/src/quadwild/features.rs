// Sharp feature detection based on dihedral angle between adjacent faces.
//
// Edges where the dihedral angle exceeds a threshold are marked as "sharp features".
// These constrain the cross-field and serve as patch boundaries during tracing.

use std::collections::HashSet;

use procgeo_core::Geometry;

use super::adjacency::{EdgeKey, MeshAdjacency};

/// Set of edges flagged as sharp features.
#[derive(Clone, Debug)]
pub struct SharpEdges {
    pub edges: HashSet<EdgeKey>,
    /// Per-point: is this point on a sharp feature?
    pub is_sharp_point: Vec<bool>,
    /// Feature corners: points where 3+ sharp edges meet, or boundary corners.
    pub corner_points: Vec<usize>,
}

/// Detect sharp edges based on dihedral angle threshold.
///
/// An edge is sharp if:
/// - It's a boundary edge, OR
/// - The dihedral angle between its two adjacent faces exceeds `threshold_deg`.
pub fn detect_sharp_edges(geo: &Geometry, adj: &MeshAdjacency, threshold_deg: f32) -> SharpEdges {
    let threshold_rad = threshold_deg.to_radians();
    let mut sharp_set = HashSet::new();
    let mut is_sharp_point = vec![false; adj.num_points];

    // All boundary edges are sharp
    for &ek in &adj.boundary_edges {
        sharp_set.insert(ek);
        is_sharp_point[ek.0] = true;
        is_sharp_point[ek.1] = true;
    }

    // Check dihedral angles for interior edges
    for (&ek, face_list) in &adj.edge_faces {
        if face_list.len() != 2 {
            continue;
        }
        let f0 = face_list[0];
        let f1 = face_list[1];

        let n0 = adj.face_normal(geo, f0);
        let n1 = adj.face_normal(geo, f1);

        // Dihedral angle: angle between face normals
        let cos_angle = n0.dot(n1).clamp(-1.0, 1.0);
        let dihedral = cos_angle.acos();

        if dihedral > threshold_rad {
            sharp_set.insert(ek);
            is_sharp_point[ek.0] = true;
            is_sharp_point[ek.1] = true;
        }
    }

    // Detect corners: points where 3+ sharp edges meet
    let mut sharp_edge_count = vec![0u32; adj.num_points];
    for &ek in &sharp_set {
        sharp_edge_count[ek.0] += 1;
        sharp_edge_count[ek.1] += 1;
    }
    let corner_points: Vec<usize> = (0..adj.num_points)
        .filter(|&vi| {
            // Corner if 3+ sharp edges meet, or exactly 1 (dead end), or boundary corner
            let count = sharp_edge_count[vi];
            count >= 3 || count == 1
        })
        .collect();

    SharpEdges {
        edges: sharp_set,
        is_sharp_point,
        corner_points,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    fn make_crease_mesh() -> (Geometry, MeshAdjacency) {
        // Two triangles meeting at a 90-degree dihedral angle
        let mut geo = Geometry::new();
        let p0 = geo.add_point(Vec3::new(0.0, 0.0, 0.0));
        let p1 = geo.add_point(Vec3::new(1.0, 0.0, 0.0));
        let p2 = geo.add_point(Vec3::new(0.5, 0.0, 1.0)); // flat on XZ
        let p3 = geo.add_point(Vec3::new(0.5, 1.0, 0.0)); // bent up on XY
        geo.add_face(&[p0, p1, p2]); // lies in XZ plane, normal ~+Y
        geo.add_face(&[p1, p0, p3]); // lies in XY plane, normal ~+Z
        let adj = MeshAdjacency::build(&geo).unwrap();
        (geo, adj)
    }

    #[test]
    fn detects_sharp_crease() {
        let (geo, adj) = make_crease_mesh();
        // Threshold 45 degrees should detect the ~90 degree crease
        let sharp = detect_sharp_edges(&geo, &adj, 45.0);
        let shared_edge = EdgeKey::new(0, 1);
        assert!(
            sharp.edges.contains(&shared_edge),
            "should detect the crease edge as sharp"
        );
    }

    #[test]
    fn flat_mesh_no_interior_sharp() {
        let mut geo = Geometry::new();
        let p0 = geo.add_point(Vec3::new(0.0, 0.0, 0.0));
        let p1 = geo.add_point(Vec3::new(1.0, 0.0, 0.0));
        let p2 = geo.add_point(Vec3::new(1.0, 1.0, 0.0));
        let p3 = geo.add_point(Vec3::new(0.0, 1.0, 0.0));
        geo.add_face(&[p0, p1, p2]);
        geo.add_face(&[p0, p2, p3]);
        let adj = MeshAdjacency::build(&geo).unwrap();

        let sharp = detect_sharp_edges(&geo, &adj, 35.0);
        let shared = EdgeKey::new(0, 2);
        // The shared interior edge is flat, should not be sharp
        assert!(
            !sharp.edges.contains(&shared),
            "flat interior edge should not be sharp"
        );
        // But boundary edges should be sharp
        assert_eq!(sharp.edges.len(), 4);
    }
}
