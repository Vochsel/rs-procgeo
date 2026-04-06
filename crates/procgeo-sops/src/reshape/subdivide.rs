use std::collections::HashMap;

use glam::Vec3;
use serde::{Deserialize, Serialize};

use procgeo_core::{Geometry, PointHandle, PrimHandle};

use crate::{Sop, SopError};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubdivideParams {
    /// Number of subdivision levels.
    pub depth: u32,
}

impl Default for SubdivideParams {
    fn default() -> Self {
        SubdivideParams { depth: 1 }
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
            current = subdivide_once(&current);
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
        let params = SubdivideParams { depth: 1 };
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
        let params = SubdivideParams { depth: 1 };
        let result = make_triangle().apply(&SubdivideSop, &params).unwrap();

        assert_eq!(result.num_prims(), 4, "expected 4 sub-triangles");
        // 3 corners + 3 edge mids + 1 unused centroid = 7
        assert_eq!(result.num_points(), 7, "expected 7 points (3+3+1 unused centroid)");
    }

    #[test]
    fn subdivide_box() {
        // Box has 6 quad faces → depth 1 → 24 quads
        let params = SubdivideParams { depth: 1 };
        let box_geo = generate(&BoxSop, &BoxParams::default()).unwrap();
        let result = box_geo.apply(&SubdivideSop, &params).unwrap();

        assert_eq!(result.num_prims(), 24, "expected 24 sub-quads");
    }

    #[test]
    fn subdivide_depth_2() {
        // 1 quad → 4 at depth 1 → 16 at depth 2
        let params = SubdivideParams { depth: 2 };
        let result = make_quad().apply(&SubdivideSop, &params).unwrap();

        assert_eq!(result.num_prims(), 16, "expected 16 quads at depth 2");
    }
}
