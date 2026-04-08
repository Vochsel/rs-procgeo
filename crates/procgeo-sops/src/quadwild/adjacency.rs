// Mesh adjacency data structures for topology navigation.
//
// Builds face-face adjacency, edge maps, vertex-face rings, and boundary info
// from procgeo Geometry.

use std::collections::HashMap;

use glam::Vec3;

use procgeo_core::{Geometry, PointHandle, PrimHandle};

/// Oriented half-edge key: (point_a, point_b) where a < b for canonical form.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EdgeKey(pub usize, pub usize);

impl EdgeKey {
    pub fn new(a: usize, b: usize) -> Self {
        if a < b { EdgeKey(a, b) } else { EdgeKey(b, a) }
    }
}

/// Per-face adjacency info.
#[derive(Clone, Debug)]
pub struct FaceData {
    /// Point indices of this face (3 for triangles).
    pub points: Vec<usize>,
    /// Adjacent face index per edge. `adj[i]` is the face sharing edge (points[i], points[(i+1)%n]).
    /// `None` for boundary edges.
    pub adj: Vec<Option<usize>>,
    /// Edge key per edge of this face.
    pub edges: Vec<EdgeKey>,
}

/// Full mesh adjacency structure.
#[derive(Clone, Debug)]
pub struct MeshAdjacency {
    /// Per-face topology data.
    pub faces: Vec<FaceData>,
    /// Map from edge key to the (up to 2) face indices sharing that edge.
    pub edge_faces: HashMap<EdgeKey, Vec<usize>>,
    /// Map from point index to list of face indices containing that point.
    pub point_faces: Vec<Vec<usize>>,
    /// Number of points.
    pub num_points: usize,
    /// Set of boundary edge keys.
    pub boundary_edges: Vec<EdgeKey>,
    /// Boundary point flags.
    pub is_boundary_point: Vec<bool>,
}

impl MeshAdjacency {
    pub fn build(geo: &Geometry) -> Result<Self, String> {
        let num_points = geo.num_points();
        let num_faces = geo.num_prims();

        let mut faces = Vec::with_capacity(num_faces);
        let mut edge_faces: HashMap<EdgeKey, Vec<usize>> = HashMap::new();
        let mut point_faces = vec![Vec::new(); num_points];

        // Build face data and edge map
        for fi in 0..num_faces {
            let ph = PrimHandle::from_index(fi);
            let pts: Vec<usize> = geo.prim_points(ph).iter().map(|p| p.index()).collect();
            let n = pts.len();

            let mut edges = Vec::with_capacity(n);
            for i in 0..n {
                let ek = EdgeKey::new(pts[i], pts[(i + 1) % n]);
                edges.push(ek);
                edge_faces.entry(ek).or_default().push(fi);
            }

            for &pi in &pts {
                point_faces[pi].push(fi);
            }

            faces.push(FaceData {
                points: pts,
                adj: vec![None; n],
                edges,
            });
        }

        // Build face-face adjacency
        for fi in 0..num_faces {
            let n = faces[fi].edges.len();
            for ei in 0..n {
                let ek = faces[fi].edges[ei];
                if let Some(neighbors) = edge_faces.get(&ek) {
                    for &nfi in neighbors {
                        if nfi != fi {
                            faces[fi].adj[ei] = Some(nfi);
                        }
                    }
                }
            }
        }

        // Boundary detection
        let mut boundary_edges = Vec::new();
        let mut is_boundary_point = vec![false; num_points];
        for (&ek, face_list) in &edge_faces {
            if face_list.len() == 1 {
                boundary_edges.push(ek);
                is_boundary_point[ek.0] = true;
                is_boundary_point[ek.1] = true;
            }
        }

        Ok(MeshAdjacency {
            faces,
            edge_faces,
            point_faces,
            num_points,
            boundary_edges,
            is_boundary_point,
        })
    }

    /// Get face normal for face index fi.
    pub fn face_normal(&self, geo: &Geometry, fi: usize) -> Vec3 {
        let pts = &self.faces[fi].points;
        if pts.len() < 3 {
            return Vec3::Y;
        }
        let p0 = geo.point_pos(PointHandle::from_index(pts[0]));
        let p1 = geo.point_pos(PointHandle::from_index(pts[1]));
        let p2 = geo.point_pos(PointHandle::from_index(pts[2]));
        let e1 = p1 - p0;
        let e2 = p2 - p0;
        e1.cross(e2).normalize_or_zero()
    }

    /// Get the face index on the other side of an edge, if any.
    pub fn opposite_face(&self, fi: usize, edge: EdgeKey) -> Option<usize> {
        self.edge_faces
            .get(&edge)
            .and_then(|faces| faces.iter().find(|&&f| f != fi).copied())
    }

    /// Get the edge index within face fi for a given edge key.
    pub fn edge_index_in_face(&self, fi: usize, edge: EdgeKey) -> Option<usize> {
        self.faces[fi].edges.iter().position(|&e| e == edge)
    }

    /// Compute the angle at vertex vi in face fi.
    pub fn corner_angle(&self, geo: &Geometry, fi: usize, vi: usize) -> f32 {
        let pts = &self.faces[fi].points;
        let n = pts.len();
        let pos = pts.iter().position(|&p| p == vi);
        let idx = match pos {
            Some(i) => i,
            None => return 0.0,
        };
        let prev = pts[(idx + n - 1) % n];
        let next = pts[(idx + 1) % n];

        let p_cur = geo.point_pos(PointHandle::from_index(vi));
        let p_prev = geo.point_pos(PointHandle::from_index(prev));
        let p_next = geo.point_pos(PointHandle::from_index(next));

        let e1 = (p_prev - p_cur).normalize_or_zero();
        let e2 = (p_next - p_cur).normalize_or_zero();
        e1.dot(e2).clamp(-1.0, 1.0).acos()
    }

    /// Compute vertex normal as area-weighted average of incident face normals.
    pub fn vertex_normal(&self, geo: &Geometry, vi: usize) -> Vec3 {
        let mut n = Vec3::ZERO;
        for &fi in &self.point_faces[vi] {
            let fn_i = self.face_normal(geo, fi);
            let angle = self.corner_angle(geo, fi, vi);
            n += fn_i * angle;
        }
        n.normalize_or_zero()
    }
}

/// Compute average edge length across all faces.
pub fn average_edge_length(geo: &Geometry) -> f32 {
    let mut sum = 0.0f64;
    let mut count = 0u64;
    for fi in 0..geo.num_prims() {
        let ph = PrimHandle::from_index(fi);
        let pts = geo.prim_points(ph);
        let n = pts.len();
        for i in 0..n {
            let p0 = geo.point_pos(pts[i]);
            let p1 = geo.point_pos(pts[(i + 1) % n]);
            sum += (p1 - p0).length() as f64;
            count += 1;
        }
    }
    if count == 0 {
        1.0
    } else {
        (sum / count as f64) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_two_tris() -> Geometry {
        let mut geo = Geometry::new();
        let p0 = geo.add_point(Vec3::new(0.0, 0.0, 0.0));
        let p1 = geo.add_point(Vec3::new(1.0, 0.0, 0.0));
        let p2 = geo.add_point(Vec3::new(1.0, 1.0, 0.0));
        let p3 = geo.add_point(Vec3::new(0.0, 1.0, 0.0));
        geo.add_face(&[p0, p1, p2]);
        geo.add_face(&[p0, p2, p3]);
        geo
    }

    #[test]
    fn adjacency_basic() {
        let geo = make_two_tris();
        let adj = MeshAdjacency::build(&geo).unwrap();
        assert_eq!(adj.faces.len(), 2);
        assert_eq!(adj.num_points, 4);
        // The shared edge (0,2) should give adjacency
        let shared = EdgeKey::new(0, 2);
        assert_eq!(adj.edge_faces[&shared].len(), 2);
    }

    #[test]
    fn boundary_detection() {
        let geo = make_two_tris();
        let adj = MeshAdjacency::build(&geo).unwrap();
        // 4 boundary edges on the square (edges 0-1, 1-2, 2-3, 3-0)
        assert_eq!(adj.boundary_edges.len(), 4);
        assert!(adj.is_boundary_point.iter().all(|&b| b));
    }

    #[test]
    fn face_normal_up() {
        let geo = make_two_tris();
        let adj = MeshAdjacency::build(&geo).unwrap();
        let n = adj.face_normal(&geo, 0);
        assert!((n.z - 1.0).abs() < 1e-5 || (n.z + 1.0).abs() < 1e-5);
    }

    #[test]
    fn avg_edge() {
        let geo = make_two_tris();
        let avg = average_edge_length(&geo);
        assert!(avg > 0.0);
        assert!(avg < 2.0);
    }
}
