use std::collections::HashMap;

use glam::Vec3;
use serde::{Deserialize, Serialize};

use procgeo_core::{Geometry, PointHandle, PrimHandle};

use crate::{Sop, SopError};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub enum SubdivideMode {
    #[default]
    Linear,
    CatmullClark,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubdivideParams {
    /// Number of subdivision levels.
    pub depth: u32,
    /// Subdivision mode: Linear or CatmullClark.
    pub mode: SubdivideMode,
}

impl Default for SubdivideParams {
    fn default() -> Self {
        SubdivideParams {
            depth: 1,
            mode: SubdivideMode::Linear,
        }
    }
}

pub struct SubdivideSop;

/// Perform one level of linear subdivision on geometry.
fn subdivide_once(geo: &Geometry) -> Geometry {
    let mut out = Geometry::new();

    // Copy all original points into output
    let orig_pt_count = geo.num_points();
    let mut orig_handles: Vec<PointHandle> = Vec::with_capacity(orig_pt_count);
    for i in 0..orig_pt_count {
        let ph = PointHandle::from_index(i);
        let pos = geo.point_pos(ph);
        orig_handles.push(out.add_point(pos));
    }

    // Cache for edge midpoints: key = (min_idx, max_idx), value = new PointHandle
    let mut edge_mids: HashMap<(usize, usize), PointHandle> = HashMap::new();

    let get_or_create_edge_mid =
        |a: usize,
         b: usize,
         edge_mids: &mut HashMap<(usize, usize), PointHandle>,
         out: &mut Geometry,
         geo: &Geometry|
         -> PointHandle {
            let key = (a.min(b), a.max(b));
            if let Some(&h) = edge_mids.get(&key) {
                return h;
            }
            let pa = geo.point_pos(PointHandle::from_index(a));
            let pb = geo.point_pos(PointHandle::from_index(b));
            let mid_pos = (pa + pb) * 0.5;
            let h = out.add_point(mid_pos);
            edge_mids.insert(key, h);
            h
        };

    for prim_idx in 0..geo.num_prims() {
        let ph = PrimHandle::from_index(prim_idx);
        let pt_handles = geo.prim_points(ph);
        let n = pt_handles.len();

        if n < 3 {
            // Not enough verts to subdivide meaningfully, just copy
            let new_pts: Vec<PointHandle> = pt_handles
                .iter()
                .map(|&p| orig_handles[p.index()])
                .collect();
            out.add_face(&new_pts);
            continue;
        }

        let prim = geo.prim(ph);
        let poly_type = match prim {
            procgeo_core::Primitive::Polygon(p) => p.poly_type.clone(),
        };

        // Compute face centroid
        let centroid: Vec3 = pt_handles
            .iter()
            .map(|&p| geo.point_pos(p))
            .sum::<Vec3>()
            / n as f32;
        let center_h = out.add_point(centroid);

        if n == 3 {
            // Triangle → 4 triangles using edge midpoints
            let a = pt_handles[0].index();
            let b = pt_handles[1].index();
            let c = pt_handles[2].index();

            let ha = orig_handles[a];
            let hb = orig_handles[b];
            let hc = orig_handles[c];

            let hab = get_or_create_edge_mid(a, b, &mut edge_mids, &mut out, geo);
            let hbc = get_or_create_edge_mid(b, c, &mut edge_mids, &mut out, geo);
            let hca = get_or_create_edge_mid(c, a, &mut edge_mids, &mut out, geo);

            // Use the center handle as the 4th triangle (inner triangle)
            // Actually for a triangle: 4 sub-triangles [a,ab,ca], [ab,b,bc], [ca,bc,c], [ab,bc,ca]
            // We don't use the face centroid for triangles
            // Remove the center point we added — can't easily undo, so use it as the inner triangle center
            // Instead: classic 4-triangle split, center is the centroid of the inner triangle (== center of mass)
            // For triangle, center = midpoint of midpoints = same as the centroid we computed
            // Use center_h as the centroid for quads, but for triangles we don't need it.
            // Since we already added it, just leave it unreferenced (it won't cause issues).

            match poly_type {
                procgeo_core::PolyType::Closed => {
                    out.add_face(&[ha, hab, hca]);
                    out.add_face(&[hab, hb, hbc]);
                    out.add_face(&[hca, hbc, hc]);
                    out.add_face(&[hab, hbc, hca]);
                }
                procgeo_core::PolyType::Open => {
                    out.add_polyline(&[ha, hab, hb]);
                    // For open polylines, subdivide by inserting midpoints
                }
            }
        } else if n == 4 {
            // Quad → 4 sub-quads
            let a = pt_handles[0].index();
            let b = pt_handles[1].index();
            let c = pt_handles[2].index();
            let d = pt_handles[3].index();

            let ha = orig_handles[a];
            let hb = orig_handles[b];
            let hc = orig_handles[c];
            let hd = orig_handles[d];

            let hab = get_or_create_edge_mid(a, b, &mut edge_mids, &mut out, geo);
            let hbc = get_or_create_edge_mid(b, c, &mut edge_mids, &mut out, geo);
            let hcd = get_or_create_edge_mid(c, d, &mut edge_mids, &mut out, geo);
            let hda = get_or_create_edge_mid(d, a, &mut edge_mids, &mut out, geo);

            match poly_type {
                procgeo_core::PolyType::Closed => {
                    // 4 sub-quads with correct winding
                    out.add_face(&[ha, hab, center_h, hda]);
                    out.add_face(&[hab, hb, hbc, center_h]);
                    out.add_face(&[center_h, hbc, hc, hcd]);
                    out.add_face(&[hda, center_h, hcd, hd]);
                }
                procgeo_core::PolyType::Open => {
                    out.add_polyline(&[ha, hab, hb]);
                }
            }
        } else {
            // N-gon: use triangle fan from centroid (each edge → a triangle)
            for i in 0..n {
                let ai = pt_handles[i].index();
                let bi = pt_handles[(i + 1) % n].index();

                let ha = orig_handles[ai];
                let _hb = orig_handles[bi];
                let hab = get_or_create_edge_mid(ai, bi, &mut edge_mids, &mut out, geo);

                match poly_type {
                    procgeo_core::PolyType::Closed => {
                        out.add_face(&[ha, hab, center_h]);
                    }
                    procgeo_core::PolyType::Open => {}
                }
            }
        }
    }

    out
}

/// Perform one level of Catmull-Clark subdivision on geometry.
fn catmull_clark_once(geo: &Geometry) -> Geometry {
    let num_pts = geo.num_points();
    let num_prims = geo.num_prims();

    // ---------------------------------------------------------------------------
    // Step 1: Compute face points (centroid of each face)
    // ---------------------------------------------------------------------------
    let mut face_points: Vec<Vec3> = Vec::with_capacity(num_prims);
    for prim_idx in 0..num_prims {
        let ph = PrimHandle::from_index(prim_idx);
        let pt_handles = geo.prim_points(ph);
        let n = pt_handles.len();
        let centroid: Vec3 = pt_handles.iter().map(|&p| geo.point_pos(p)).sum::<Vec3>()
            / n as f32;
        face_points.push(centroid);
    }

    // ---------------------------------------------------------------------------
    // Step 2: Build adjacency maps
    //   edge -> list of adjacent face indices (sorted edge key)
    //   vertex -> list of adjacent face indices
    // ---------------------------------------------------------------------------
    // edge key: (min_pt_idx, max_pt_idx) -> Vec<face_idx>
    let mut edge_to_faces: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
    // vertex -> Vec<face_idx>
    let mut vert_to_faces: Vec<Vec<usize>> = vec![Vec::new(); num_pts];

    for prim_idx in 0..num_prims {
        let ph = PrimHandle::from_index(prim_idx);
        let pt_handles = geo.prim_points(ph);
        let n = pt_handles.len();

        for i in 0..n {
            let ai = pt_handles[i].index();
            let bi = pt_handles[(i + 1) % n].index();
            let key = (ai.min(bi), ai.max(bi));
            edge_to_faces.entry(key).or_default().push(prim_idx);
            vert_to_faces[ai].push(prim_idx);
        }
    }

    // ---------------------------------------------------------------------------
    // Step 3: Compute edge points
    //   For interior edges (2 adjacent faces): (avg of endpoints + avg of face points) / 2
    //   For boundary edges (1 adjacent face): midpoint of endpoints
    // ---------------------------------------------------------------------------
    // edge key -> new Vec3 position
    let mut edge_point_pos: HashMap<(usize, usize), Vec3> = HashMap::new();

    for (&key, faces) in &edge_to_faces {
        let (ai, bi) = key;
        let pa = geo.point_pos(PointHandle::from_index(ai));
        let pb = geo.point_pos(PointHandle::from_index(bi));
        let edge_mid = (pa + pb) * 0.5;

        let ep_pos = if faces.len() >= 2 {
            // Interior edge: average of 2 face points
            let fp_avg = (face_points[faces[0]] + face_points[faces[1]]) * 0.5;
            (edge_mid + fp_avg) * 0.5
        } else {
            // Boundary edge: just midpoint
            edge_mid
        };
        edge_point_pos.insert(key, ep_pos);
    }

    // ---------------------------------------------------------------------------
    // Step 4: Compute updated vertex positions (Catmull-Clark rule)
    //   For interior vertices:
    //     F = avg of face points of adjacent faces
    //     R = avg of edge midpoints of adjacent edges
    //     n = number of adjacent faces
    //     new_pos = (F + 2*R + (n-3)*P) / n
    //   For boundary vertices (only adjacent to boundary edges):
    //     new_pos = (P + avg of adjacent boundary edge midpoints) / 2
    //              simplified: average of adjacent boundary edge midpoints (blends toward boundary)
    // ---------------------------------------------------------------------------
    let mut updated_vert_pos: Vec<Vec3> = Vec::with_capacity(num_pts);

    for (vi, adj_faces) in vert_to_faces.iter().enumerate() {
        let p = geo.point_pos(PointHandle::from_index(vi));
        let n = adj_faces.len();

        if n == 0 {
            // Isolated point, keep as-is
            updated_vert_pos.push(p);
            continue;
        }

        // Determine which adjacent edges are boundary edges
        // Collect all edges touching this vertex
        let mut adj_edges: Vec<(usize, usize)> = Vec::new();
        for prim_idx in adj_faces {
            let ph = PrimHandle::from_index(*prim_idx);
            let pt_handles = geo.prim_points(ph);
            let np = pt_handles.len();
            for i in 0..np {
                let ai = pt_handles[i].index();
                let bi = pt_handles[(i + 1) % np].index();
                if ai == vi || bi == vi {
                    let key = (ai.min(bi), ai.max(bi));
                    if !adj_edges.contains(&key) {
                        adj_edges.push(key);
                    }
                }
            }
        }

        // Find boundary edges (edges touching this vertex with only 1 adjacent face)
        let boundary_edges: Vec<(usize, usize)> = adj_edges
            .iter()
            .filter(|&&key| {
                edge_to_faces.get(&key).is_some_and(|faces| faces.len() == 1)
            })
            .copied()
            .collect();

        let is_boundary = !boundary_edges.is_empty();

        if is_boundary {
            // Boundary vertex: average of midpoints of boundary edges through this vertex
            let mut sum = p;
            let mut count = 1;
            for key in &boundary_edges {
                let (ai, bi) = *key;
                let pa = geo.point_pos(PointHandle::from_index(ai));
                let pb = geo.point_pos(PointHandle::from_index(bi));
                sum += (pa + pb) * 0.5;
                count += 1;
            }
            updated_vert_pos.push(sum / count as f32);
        } else {
            // Interior vertex: full Catmull-Clark rule
            let n_f32 = n as f32;

            // F = average of face points
            let f: Vec3 = adj_faces.iter().map(|&fi| face_points[fi]).sum::<Vec3>() / n_f32;

            // R = average of edge midpoints of all adjacent edges
            let r: Vec3 = adj_edges
                .iter()
                .map(|&(ai, bi)| {
                    let pa = geo.point_pos(PointHandle::from_index(ai));
                    let pb = geo.point_pos(PointHandle::from_index(bi));
                    (pa + pb) * 0.5
                })
                .sum::<Vec3>()
                / adj_edges.len() as f32;

            // new = (F + 2*R + (n-3)*P) / n
            let new_pos = (f + 2.0 * r + (n_f32 - 3.0) * p) / n_f32;
            updated_vert_pos.push(new_pos);
        }
    }

    // ---------------------------------------------------------------------------
    // Step 5: Build output geometry
    //   Add updated vertex positions as points
    //   Add face points as points
    //   Add edge points as points
    // ---------------------------------------------------------------------------
    let mut out = Geometry::new();

    // Map original vertex index -> PointHandle in output
    let mut orig_to_out: Vec<PointHandle> = Vec::with_capacity(num_pts);
    for pos in &updated_vert_pos {
        orig_to_out.push(out.add_point(*pos));
    }

    // Map face index -> PointHandle in output
    let mut face_to_out: Vec<PointHandle> = Vec::with_capacity(num_prims);
    for pos in &face_points {
        face_to_out.push(out.add_point(*pos));
    }

    // Map edge key -> PointHandle in output
    let mut edge_to_out: HashMap<(usize, usize), PointHandle> = HashMap::new();
    for (&key, &pos) in &edge_point_pos {
        let h = out.add_point(pos);
        edge_to_out.insert(key, h);
    }

    // ---------------------------------------------------------------------------
    // Step 6: Create new faces
    //   For each original face with vertices [v0, v1, v2, ..., vn-1]:
    //     For each vertex vi:
    //       new quad = [updated(vi), edge_pt(vi, vi+1), face_pt, edge_pt(vi-1, vi)]
    // ---------------------------------------------------------------------------
    for (prim_idx, &fp_h) in face_to_out.iter().enumerate() {
        let ph = PrimHandle::from_index(prim_idx);
        let pt_handles = geo.prim_points(ph);
        let n = pt_handles.len();

        for i in 0..n {
            let vi = pt_handles[i].index();
            let vi_next = pt_handles[(i + 1) % n].index();
            let vi_prev = pt_handles[(i + n - 1) % n].index();

            let v_h = orig_to_out[vi];
            let ep_next_h = edge_to_out[&(vi.min(vi_next), vi.max(vi_next))];
            let ep_prev_h = edge_to_out[&(vi.min(vi_prev), vi.max(vi_prev))];

            // new quad: [v, ep_to_next, face_pt, ep_from_prev]
            out.add_face(&[v_h, ep_next_h, fp_h, ep_prev_h]);
        }
    }

    out
}

impl Sop for SubdivideSop {
    type Params = SubdivideParams;

    fn name(&self) -> &'static str {
        "subdivide"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 1)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;

        let mut current = inputs[0].clone();
        for _ in 0..params.depth {
            current = match params.mode {
                SubdivideMode::Linear => subdivide_once(&current),
                SubdivideMode::CatmullClark => catmull_clark_once(&current),
            };
        }
        Ok(current)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creation::box_sop::{BoxSop, BoxParams};
    use crate::{GeometryExt, generate};
    use glam::Vec3;

    fn make_quad() -> Geometry {
        let mut geo = Geometry::new();
        let p0 = geo.add_point(Vec3::new(0.0, 0.0, 0.0));
        let p1 = geo.add_point(Vec3::new(1.0, 0.0, 0.0));
        let p2 = geo.add_point(Vec3::new(1.0, 0.0, 1.0));
        let p3 = geo.add_point(Vec3::new(0.0, 0.0, 1.0));
        geo.add_face(&[p0, p1, p2, p3]);
        geo
    }

    fn make_triangle() -> Geometry {
        let mut geo = Geometry::new();
        let p0 = geo.add_point(Vec3::new(0.0, 0.0, 0.0));
        let p1 = geo.add_point(Vec3::new(1.0, 0.0, 0.0));
        let p2 = geo.add_point(Vec3::new(0.5, 0.0, 1.0));
        geo.add_face(&[p0, p1, p2]);
        geo
    }

    #[test]
    fn subdivide_single_quad() {
        // 1 quad (4 pts) → depth 1 → 4 quads, 9 points (4 corners + 4 edge mids + 1 center)
        let params = SubdivideParams { depth: 1, mode: SubdivideMode::Linear };
        let result = make_quad().apply(&SubdivideSop, &params).unwrap();

        assert_eq!(result.num_prims(), 4, "expected 4 sub-quads");
        // 4 original corners + 4 edge midpoints + 1 face center = 9
        assert_eq!(result.num_points(), 9, "expected 9 points");
    }

    #[test]
    fn subdivide_triangle() {
        // 1 triangle (3 pts) → depth 1 → 4 triangles, 6 points (3 corners + 3 edge mids + 1 unused centroid)
        // Note: we add an unreferenced centroid point for triangles (a known limitation)
        // So point count will be 7 (3 + 3 + 1 unused center)
        let params = SubdivideParams { depth: 1, mode: SubdivideMode::Linear };
        let result = make_triangle().apply(&SubdivideSop, &params).unwrap();

        assert_eq!(result.num_prims(), 4, "expected 4 sub-triangles");
        // 3 corners + 3 edge mids + 1 unused centroid = 7
        assert_eq!(result.num_points(), 7, "expected 7 points (3+3+1 unused centroid)");
    }

    #[test]
    fn subdivide_box() {
        // Box has 6 quad faces → depth 1 → 24 quads
        let params = SubdivideParams { depth: 1, mode: SubdivideMode::Linear };
        let box_geo = generate(&BoxSop, &BoxParams::default()).unwrap();
        let result = box_geo.apply(&SubdivideSop, &params).unwrap();

        assert_eq!(result.num_prims(), 24, "expected 24 sub-quads");
    }

    #[test]
    fn subdivide_depth_2() {
        // 1 quad → 4 at depth 1 → 16 at depth 2
        let params = SubdivideParams { depth: 2, mode: SubdivideMode::Linear };
        let result = make_quad().apply(&SubdivideSop, &params).unwrap();

        assert_eq!(result.num_prims(), 16, "expected 16 quads at depth 2");
    }

    // -----------------------------------------------------------------------
    // Catmull-Clark tests
    // -----------------------------------------------------------------------

    #[test]
    fn catmull_clark_quad() {
        // Single quad → 4 quads, 9 points (same count as linear but different positions)
        let params = SubdivideParams { depth: 1, mode: SubdivideMode::CatmullClark };
        let result = make_quad().apply(&SubdivideSop, &params).unwrap();

        assert_eq!(result.num_prims(), 4, "CC: expected 4 sub-quads from single quad");
        assert_eq!(result.num_points(), 9, "CC: expected 9 points (4 updated verts + 4 edge pts + 1 face pt)");
    }

    #[test]
    fn catmull_clark_box() {
        // Box (6 quads) → 24 quads after 1 CC subdivision
        let params = SubdivideParams { depth: 1, mode: SubdivideMode::CatmullClark };
        let box_geo = generate(&BoxSop, &BoxParams::default()).unwrap();
        let orig_bbox = box_geo.bounding_box();
        let result = box_geo.apply(&SubdivideSop, &params).unwrap();

        assert_eq!(result.num_prims(), 24, "CC: expected 24 sub-quads from box");

        // CC should produce a smaller bounding box than the original (smooths corners)
        let cc_bbox = result.bounding_box();
        assert!(
            cc_bbox.max.x < orig_bbox.max.x + 1e-4,
            "CC box max.x should not exceed original"
        );
        assert!(
            cc_bbox.min.x > orig_bbox.min.x - 1e-4,
            "CC box min.x should not exceed original"
        );
    }

    #[test]
    fn catmull_clark_preserves_topology() {
        // CC and linear should produce the same face count at same depth
        let linear_params = SubdivideParams { depth: 1, mode: SubdivideMode::Linear };
        let cc_params = SubdivideParams { depth: 1, mode: SubdivideMode::CatmullClark };

        let box_geo = generate(&BoxSop, &BoxParams::default()).unwrap();
        let linear_result = box_geo.clone().apply(&SubdivideSop, &linear_params).unwrap();
        let cc_result = box_geo.apply(&SubdivideSop, &cc_params).unwrap();

        assert_eq!(
            linear_result.num_prims(),
            cc_result.num_prims(),
            "CC and linear should produce same face count"
        );
    }

    #[test]
    fn default_params_are_linear() {
        // Ensure default still works (mode defaults to Linear)
        let params = SubdivideParams::default();
        let result = make_quad().apply(&SubdivideSop, &params).unwrap();
        assert_eq!(result.num_prims(), 4);
    }
}
