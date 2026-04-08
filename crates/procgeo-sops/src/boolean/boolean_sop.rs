// BooleanSop — orchestrator for CSG boolean operations.
//
// Implements Union, Intersect, Subtract, Shatter, Seam, Detect, Resolve, and
// Custom operations by composing the BVH, intersection, splitting,
// classification, and de-triangulation subsystems.

use std::collections::{HashMap, HashSet};

use glam::Vec3;
use serde::{Deserialize, Serialize};

use procgeo_core::{Geometry, PointHandle, PrimHandle};

use crate::{Sop, SopError};

use super::bvh::{Triangle, TriangleBvh};
use super::classification::is_inside_mesh;
use super::detriangulate::{DetriMode, Polygon, detriangulate};
use super::intersection::{TriTriResult, tri_tri_intersection};
use super::splitting::{CutEdge, TriFragment, split_triangle};

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Boolean operation mode.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub enum BooleanOp {
    #[default]
    Union,
    Intersect,
    Subtract,
    Shatter,
    Seam,
    Detect,
    Resolve,
    Custom,
}

/// How to treat input geometry.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub enum BooleanTreatAs {
    #[default]
    Solid,
    Surface,
}

/// De-triangulation mode for the output.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub enum Detriangulate {
    #[default]
    All,
    OnlyUnchanged,
    None,
}

/// Custom match mode for the Custom operation.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub enum CustomMatch {
    A,
    B,
    #[default]
    Both,
    ExactlyOne,
}

// ---------------------------------------------------------------------------
// Params
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BooleanParams {
    pub group_a: Option<String>,
    pub group_b: Option<String>,
    pub treat_a_as: BooleanTreatAs,
    pub treat_b_as: BooleanTreatAs,
    pub resolve_self_a: bool,
    pub resolve_self_b: bool,
    pub operation: BooleanOp,
    pub detriangulate: Detriangulate,
    pub assume_seam_flat: bool,
    pub unique_seam_points: bool,
    pub collapse_tiny_edges: bool,
    pub edge_length_threshold: f32,
    pub a_depth_range: [i32; 2],
    pub b_depth_range: [i32; 2],
    pub custom_match: CustomMatch,
    pub merge_adjacent: bool,
    pub generate_aa_seams: bool,
    pub generate_bb_seams: bool,
    pub generate_ab_seams: bool,
    pub a_inside_b_group: Option<String>,
    pub a_outside_b_group: Option<String>,
    pub b_inside_a_group: Option<String>,
    pub b_outside_a_group: Option<String>,
    pub aa_seam_edge_group: Option<String>,
    pub bb_seam_edge_group: Option<String>,
    pub ab_seam_edge_group: Option<String>,
}

impl Default for BooleanParams {
    fn default() -> Self {
        BooleanParams {
            group_a: None,
            group_b: None,
            treat_a_as: BooleanTreatAs::Solid,
            treat_b_as: BooleanTreatAs::Solid,
            resolve_self_a: false,
            resolve_self_b: false,
            operation: BooleanOp::Union,
            detriangulate: Detriangulate::All,
            assume_seam_flat: true,
            unique_seam_points: false,
            collapse_tiny_edges: true,
            edge_length_threshold: 1e-5,
            a_depth_range: [1, 9999],
            b_depth_range: [1, 9999],
            custom_match: CustomMatch::Both,
            merge_adjacent: true,
            generate_aa_seams: false,
            generate_bb_seams: false,
            generate_ab_seams: true,
            a_inside_b_group: None,
            a_outside_b_group: None,
            b_inside_a_group: None,
            b_outside_a_group: None,
            aa_seam_edge_group: None,
            bb_seam_edge_group: None,
            ab_seam_edge_group: None,
        }
    }
}

// ---------------------------------------------------------------------------
// BooleanSop
// ---------------------------------------------------------------------------

pub struct BooleanSop;

impl Sop for BooleanSop {
    type Params = BooleanParams;

    fn name(&self) -> &'static str {
        "boolean"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 2)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;

        let geo_a = inputs[0];
        let geo_b = if inputs.len() > 1 {
            Some(inputs[1])
        } else {
            None
        };

        // -----------------------------------------------------------------
        // 1. Triangulate both input meshes
        // -----------------------------------------------------------------
        let tris_a = triangulate_geometry(geo_a, 0);
        let tris_b = geo_b
            .map(|g| triangulate_geometry(g, 1))
            .unwrap_or_default();

        // -----------------------------------------------------------------
        // 2. Early exit for trivial cases
        // -----------------------------------------------------------------
        if tris_a.is_empty() && tris_b.is_empty() {
            return Ok(Geometry::new());
        }

        if geo_b.is_none() || tris_b.is_empty() {
            return match params.operation {
                BooleanOp::Union | BooleanOp::Subtract | BooleanOp::Resolve => {
                    Ok(clone_geometry(geo_a))
                }
                BooleanOp::Intersect => Ok(Geometry::new()),
                BooleanOp::Seam | BooleanOp::Detect => Ok(clone_geometry(geo_a)),
                BooleanOp::Shatter => Ok(clone_geometry(geo_a)),
                BooleanOp::Custom => Ok(clone_geometry(geo_a)),
            };
        }

        if tris_a.is_empty() {
            return match params.operation {
                BooleanOp::Union => Ok(clone_geometry(geo_b.unwrap())),
                BooleanOp::Intersect | BooleanOp::Subtract => Ok(Geometry::new()),
                _ => Ok(Geometry::new()),
            };
        }

        // -----------------------------------------------------------------
        // 3. Special modes: Seam and Detect
        // -----------------------------------------------------------------
        if params.operation == BooleanOp::Seam {
            return self.execute_seam(&tris_a, &tris_b);
        }

        if params.operation == BooleanOp::Detect {
            return self.execute_detect(geo_a, &tris_a, &tris_b);
        }

        // -----------------------------------------------------------------
        // 4. Build BVHs for both meshes
        // -----------------------------------------------------------------
        let bvh_a = TriangleBvh::build(&tris_a);
        let bvh_b = TriangleBvh::build(&tris_b);

        // -----------------------------------------------------------------
        // 5. Find overlapping pairs via BVH tree-vs-tree
        // -----------------------------------------------------------------
        let pairs = bvh_a.find_overlapping_pairs(&bvh_b);

        // -----------------------------------------------------------------
        // 6. Compute intersections and collect cut edges per triangle
        // -----------------------------------------------------------------
        // Maps: (mesh_id, source_prim_index) -> Vec<CutEdge>
        let mut cuts_a: HashMap<usize, Vec<CutEdge>> = HashMap::new();
        let mut cuts_b: HashMap<usize, Vec<CutEdge>> = HashMap::new();

        // Build lookup from prim_index -> triangle for both meshes.
        let tri_map_a = build_tri_map(&tris_a);
        let tri_map_b = build_tri_map(&tris_b);

        for &(prim_idx_a, prim_idx_b) in &pairs {
            // There may be multiple triangles from the same source prim (fan
            // triangulation of quads/n-gons). We need to intersect each
            // sub-triangle pair.
            let sub_a = tri_map_a.get(&prim_idx_a);
            let sub_b = tri_map_b.get(&prim_idx_b);

            if let (Some(subs_a), Some(subs_b)) = (sub_a, sub_b) {
                for ta in subs_a {
                    for tb in subs_b {
                        let result = tri_tri_intersection(ta.v0, ta.v1, ta.v2, tb.v0, tb.v1, tb.v2);
                        match result {
                            TriTriResult::Segment { start, end } => {
                                let cut = CutEdge { start, end };
                                cuts_a.entry(prim_idx_a).or_default().push(cut.clone());
                                cuts_b.entry(prim_idx_b).or_default().push(cut);
                            }
                            TriTriResult::Coplanar { points } => {
                                // For coplanar, create cut edges from
                                // consecutive intersection polygon points.
                                if points.len() >= 2 {
                                    for i in 0..points.len() {
                                        let j = (i + 1) % points.len();
                                        let cut = CutEdge {
                                            start: points[i],
                                            end: points[j],
                                        };
                                        cuts_a.entry(prim_idx_a).or_default().push(cut.clone());
                                        cuts_b.entry(prim_idx_b).or_default().push(cut);
                                    }
                                }
                            }
                            TriTriResult::None => {}
                        }
                    }
                }
            }
        }

        // -----------------------------------------------------------------
        // 7. Split triangles
        // -----------------------------------------------------------------
        let mut all_fragments: Vec<TriFragment> = Vec::new();

        for tri in &tris_a {
            let cuts = cuts_a.get(&tri.index).map(|v| v.as_slice()).unwrap_or(&[]);
            let frags = split_triangle(tri.v0, tri.v1, tri.v2, cuts, tri.index, 0);
            all_fragments.extend(frags);
        }

        for tri in &tris_b {
            let cuts = cuts_b.get(&tri.index).map(|v| v.as_slice()).unwrap_or(&[]);
            let frags = split_triangle(tri.v0, tri.v1, tri.v2, cuts, tri.index, 1);
            all_fragments.extend(frags);
        }

        // -----------------------------------------------------------------
        // 8. Classify and select fragments
        // -----------------------------------------------------------------
        let selected = self.classify_and_select(&all_fragments, &tris_a, &tris_b, params);

        // -----------------------------------------------------------------
        // 9. De-triangulate
        // -----------------------------------------------------------------
        let detri_mode = match params.detriangulate {
            Detriangulate::All => DetriMode::All,
            Detriangulate::OnlyUnchanged => DetriMode::OnlyUnchanged,
            Detriangulate::None => DetriMode::None,
        };

        let flat_threshold = if params.assume_seam_flat { 1e-3 } else { 1e-5 };
        let polygons = detriangulate(&selected, detri_mode, flat_threshold);

        // -----------------------------------------------------------------
        // 10. Build output geometry
        // -----------------------------------------------------------------
        let output = build_output_geometry(
            &polygons,
            params.collapse_tiny_edges,
            params.edge_length_threshold,
        );

        Ok(output)
    }
}

// ---------------------------------------------------------------------------
// BooleanSop implementation helpers
// ---------------------------------------------------------------------------

impl BooleanSop {
    /// Seam mode: compute intersection segments and output them as polylines.
    fn execute_seam(&self, tris_a: &[Triangle], tris_b: &[Triangle]) -> Result<Geometry, SopError> {
        let bvh_a = TriangleBvh::build(tris_a);
        let bvh_b = TriangleBvh::build(tris_b);
        let pairs = bvh_a.find_overlapping_pairs(&bvh_b);

        let tri_map_a = build_tri_map(tris_a);
        let tri_map_b = build_tri_map(tris_b);

        let mut output = Geometry::new();

        for &(prim_idx_a, prim_idx_b) in &pairs {
            let sub_a = tri_map_a.get(&prim_idx_a);
            let sub_b = tri_map_b.get(&prim_idx_b);

            if let (Some(subs_a), Some(subs_b)) = (sub_a, sub_b) {
                for ta in subs_a {
                    for tb in subs_b {
                        let result = tri_tri_intersection(ta.v0, ta.v1, ta.v2, tb.v0, tb.v1, tb.v2);
                        if let TriTriResult::Segment { start, end } = result {
                            if (start - end).length() > 1e-8 {
                                let p0 = output.add_point(start);
                                let p1 = output.add_point(end);
                                output.add_polyline(&[p0, p1]);
                            }
                        }
                    }
                }
            }
        }

        Ok(output)
    }

    /// Detect mode: pass through geo_a, marking intersecting prims.
    fn execute_detect(
        &self,
        geo_a: &Geometry,
        tris_a: &[Triangle],
        tris_b: &[Triangle],
    ) -> Result<Geometry, SopError> {
        let bvh_a = TriangleBvh::build(tris_a);
        let bvh_b = TriangleBvh::build(tris_b);
        let pairs = bvh_a.find_overlapping_pairs(&bvh_b);

        let tri_map_a = build_tri_map(tris_a);
        let tri_map_b = build_tri_map(tris_b);

        // Collect prim indices from A that actually intersect with B.
        let mut intersecting_prims: HashSet<usize> = HashSet::new();

        for &(prim_idx_a, prim_idx_b) in &pairs {
            if intersecting_prims.contains(&prim_idx_a) {
                continue; // Already marked.
            }
            let sub_a = tri_map_a.get(&prim_idx_a);
            let sub_b = tri_map_b.get(&prim_idx_b);

            if let (Some(subs_a), Some(subs_b)) = (sub_a, sub_b) {
                'outer: for ta in subs_a {
                    for tb in subs_b {
                        let result = tri_tri_intersection(ta.v0, ta.v1, ta.v2, tb.v0, tb.v1, tb.v2);
                        if !matches!(result, TriTriResult::None) {
                            intersecting_prims.insert(prim_idx_a);
                            break 'outer;
                        }
                    }
                }
            }
        }

        // Clone A and add prim group.
        let mut output = clone_geometry(geo_a);
        let num_prims = output.num_prims();
        output
            .groups_mut()
            .create_prim_group("axb_intersecting", num_prims);
        for &prim_idx in &intersecting_prims {
            if prim_idx < num_prims {
                output
                    .groups_mut()
                    .prim_group_mut("axb_intersecting")
                    .unwrap()
                    .add(prim_idx);
            }
        }

        Ok(output)
    }

    /// Classify each fragment's centroid as inside/outside the opposite mesh
    /// and select based on the operation.
    fn classify_and_select(
        &self,
        fragments: &[TriFragment],
        tris_a: &[Triangle],
        tris_b: &[Triangle],
        params: &BooleanParams,
    ) -> Vec<TriFragment> {
        let mut selected: Vec<TriFragment> = Vec::new();

        for frag in fragments {
            let centroid = frag.centroid();
            let keep = match frag.mesh_id {
                0 => {
                    // Fragment from mesh A — classify against B.
                    let inside_b = is_inside_mesh(centroid, tris_b);
                    match params.operation {
                        BooleanOp::Union => !inside_b,
                        BooleanOp::Intersect => inside_b,
                        BooleanOp::Subtract => !inside_b,
                        BooleanOp::Shatter | BooleanOp::Resolve => true,
                        BooleanOp::Custom => {
                            let depth = super::classification::classify_depth(centroid, tris_b);
                            let in_range = depth >= params.b_depth_range[0]
                                && depth <= params.b_depth_range[1];
                            match params.custom_match {
                                CustomMatch::A | CustomMatch::Both => in_range,
                                CustomMatch::B => false,
                                CustomMatch::ExactlyOne => in_range,
                            }
                        }
                        // Seam/Detect already handled above.
                        _ => true,
                    }
                }
                1 => {
                    // Fragment from mesh B — classify against A.
                    let inside_a = is_inside_mesh(centroid, tris_a);
                    match params.operation {
                        BooleanOp::Union => !inside_a,
                        BooleanOp::Intersect => inside_a,
                        BooleanOp::Subtract => inside_a,
                        BooleanOp::Shatter | BooleanOp::Resolve => true,
                        BooleanOp::Custom => {
                            let depth = super::classification::classify_depth(centroid, tris_a);
                            let in_range = depth >= params.a_depth_range[0]
                                && depth <= params.a_depth_range[1];
                            match params.custom_match {
                                CustomMatch::B | CustomMatch::Both => in_range,
                                CustomMatch::A => false,
                                CustomMatch::ExactlyOne => in_range,
                            }
                        }
                        _ => true,
                    }
                }
                _ => false,
            };

            if keep {
                let mut f = frag.clone();

                // For Subtract, flip B fragment winding to invert normals.
                if params.operation == BooleanOp::Subtract && f.mesh_id == 1 {
                    std::mem::swap(&mut f.v1, &mut f.v2);
                }

                selected.push(f);
            }
        }

        selected
    }
}

// ---------------------------------------------------------------------------
// Utility functions
// ---------------------------------------------------------------------------

/// Fan-triangulate all primitives in a geometry, returning triangles tagged
/// with their source primitive index and mesh id.
fn triangulate_geometry(geo: &Geometry, mesh_id: u8) -> Vec<Triangle> {
    let mut triangles = Vec::new();
    let _ = mesh_id; // mesh_id is encoded in the Triangle's index field for reference

    for prim_idx in 0..geo.num_prims() {
        let prim_handle = PrimHandle::from_index(prim_idx);
        let pts = geo.prim_points(prim_handle);

        if pts.len() < 3 {
            continue;
        }

        // Fan triangulation from the first vertex.
        let v0 = geo.point_pos(pts[0]);
        for i in 1..pts.len() - 1 {
            let v1 = geo.point_pos(pts[i]);
            let v2 = geo.point_pos(pts[i + 1]);
            triangles.push(Triangle {
                v0,
                v1,
                v2,
                index: prim_idx,
            });
        }
    }

    triangles
}

/// Build a map from source prim index to all triangles with that index.
fn build_tri_map(tris: &[Triangle]) -> HashMap<usize, Vec<&Triangle>> {
    let mut map: HashMap<usize, Vec<&Triangle>> = HashMap::new();
    for tri in tris {
        map.entry(tri.index).or_default().push(tri);
    }
    map
}

/// Clone a Geometry by rebuilding its points and primitives.
fn clone_geometry(geo: &Geometry) -> Geometry {
    let mut out = Geometry::with_capacity(geo.num_points(), geo.num_prims());

    // Copy all points.
    let mut point_handles: Vec<PointHandle> = Vec::with_capacity(geo.num_points());
    for i in 0..geo.num_points() {
        let pos = geo.point_pos(PointHandle::from_index(i));
        point_handles.push(out.add_point(pos));
    }

    // Copy all primitives.
    for prim_idx in 0..geo.num_prims() {
        let prim_handle = PrimHandle::from_index(prim_idx);
        let old_pts = geo.prim_points(prim_handle);
        let new_pts: Vec<PointHandle> =
            old_pts.iter().map(|ph| point_handles[ph.index()]).collect();

        let prim = geo.prim(prim_handle);
        match prim {
            procgeo_core::primitive::Primitive::Polygon(poly) => match poly.poly_type {
                procgeo_core::primitive::PolyType::Closed => {
                    out.add_face(&new_pts);
                }
                procgeo_core::primitive::PolyType::Open => {
                    out.add_polyline(&new_pts);
                }
            },
        }
    }

    out
}

/// Build the final output Geometry from a set of polygons, welding coincident
/// points within `eps`.
fn build_output_geometry(
    polygons: &[Polygon],
    collapse_tiny: bool,
    edge_threshold: f32,
) -> Geometry {
    let eps = 1e-6_f32;
    let mut output = Geometry::new();

    // Spatial hash for point welding: quantised position -> PointHandle.
    let mut point_map: HashMap<(i64, i64, i64), PointHandle> = HashMap::new();

    let quantize = |v: Vec3| -> (i64, i64, i64) {
        let s = 1.0 / eps;
        (
            (v.x * s).round() as i64,
            (v.y * s).round() as i64,
            (v.z * s).round() as i64,
        )
    };

    let get_or_create_point = |output: &mut Geometry,
                               point_map: &mut HashMap<(i64, i64, i64), PointHandle>,
                               pos: Vec3|
     -> PointHandle {
        let key = quantize(pos);
        if let Some(&existing) = point_map.get(&key) {
            existing
        } else {
            let handle = output.add_point(pos);
            point_map.insert(key, handle);
            handle
        }
    };

    for poly in polygons {
        let handles: Vec<PointHandle> = poly
            .vertices
            .iter()
            .map(|&v| get_or_create_point(&mut output, &mut point_map, v))
            .collect();

        // Deduplicate consecutive identical handles.
        let mut deduped: Vec<PointHandle> = Vec::with_capacity(handles.len());
        for &h in &handles {
            if deduped.last() != Some(&h) {
                deduped.push(h);
            }
        }
        // Also check wrap-around.
        if deduped.len() > 1 && deduped.first() == deduped.last() {
            deduped.pop();
        }

        if deduped.len() < 3 {
            continue;
        }

        // Optionally collapse tiny edges.
        if collapse_tiny && edge_threshold > 0.0 {
            let mut collapsed: Vec<PointHandle> = Vec::with_capacity(deduped.len());
            for &h in &deduped {
                if let Some(&last) = collapsed.last() {
                    let p_last = output.point_pos(last);
                    let p_cur = output.point_pos(h);
                    if (p_cur - p_last).length() < edge_threshold {
                        continue; // Skip this point (edge too short).
                    }
                }
                collapsed.push(h);
            }
            if collapsed.len() >= 3 {
                output.add_face(&collapsed);
            }
        } else if deduped.len() >= 3 {
            output.add_face(&deduped);
        }
    }

    output
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creation::box_sop::{BoxParams, BoxSop};
    use crate::generate;

    /// Helper to create a box geometry.
    fn make_box(center: Vec3, size: Vec3) -> Geometry {
        let sop = BoxSop;
        let params = BoxParams { size, center };
        generate(&sop, &params).unwrap()
    }

    // ------------------------------------------------------------------
    // 1. Union of two overlapping boxes
    // ------------------------------------------------------------------

    #[test]
    fn union_of_two_boxes() {
        let a = make_box(Vec3::ZERO, Vec3::ONE);
        let b = make_box(Vec3::new(0.5, 0.0, 0.0), Vec3::ONE);

        let sop = BooleanSop;
        let params = BooleanParams {
            operation: BooleanOp::Union,
            ..Default::default()
        };

        let result = sop.execute(&[&a, &b], &params).unwrap();

        assert!(
            result.num_points() > 0,
            "Union should produce points, got {}",
            result.num_points()
        );
        assert!(
            result.num_prims() > 0,
            "Union should produce prims, got {}",
            result.num_prims()
        );

        println!(
            "Union: {} points, {} prims",
            result.num_points(),
            result.num_prims()
        );
    }

    // ------------------------------------------------------------------
    // 2. Intersect of two overlapping boxes
    // ------------------------------------------------------------------

    #[test]
    fn intersect_of_two_boxes() {
        let a = make_box(Vec3::ZERO, Vec3::ONE);
        let b = make_box(Vec3::new(0.5, 0.0, 0.0), Vec3::ONE);

        let sop = BooleanSop;
        let params = BooleanParams {
            operation: BooleanOp::Intersect,
            ..Default::default()
        };

        let result = sop.execute(&[&a, &b], &params).unwrap();

        assert!(
            result.num_points() > 0,
            "Intersect should produce points, got {}",
            result.num_points()
        );
        assert!(
            result.num_prims() > 0,
            "Intersect should produce prims, got {}",
            result.num_prims()
        );

        // The intersection should be smaller than either input.
        let bb = result.bounding_box();
        let a_bb = a.bounding_box();
        let extent_x = bb.max.x - bb.min.x;
        let a_extent_x = a_bb.max.x - a_bb.min.x;
        assert!(
            extent_x < a_extent_x + 0.01,
            "Intersection extent ({extent_x}) should be <= A extent ({a_extent_x})"
        );

        println!(
            "Intersect: {} points, {} prims",
            result.num_points(),
            result.num_prims()
        );
    }

    // ------------------------------------------------------------------
    // 3. Subtract boxes
    // ------------------------------------------------------------------

    #[test]
    fn subtract_boxes() {
        let a = make_box(Vec3::ZERO, Vec3::ONE);
        let b = make_box(Vec3::new(0.5, 0.0, 0.0), Vec3::ONE);

        let sop = BooleanSop;
        let params = BooleanParams {
            operation: BooleanOp::Subtract,
            ..Default::default()
        };

        let result = sop.execute(&[&a, &b], &params).unwrap();

        assert!(
            result.num_points() > 0,
            "Subtract should produce points, got {}",
            result.num_points()
        );
        assert!(
            result.num_prims() > 0,
            "Subtract should produce prims, got {}",
            result.num_prims()
        );

        println!(
            "Subtract: {} points, {} prims",
            result.num_points(),
            result.num_prims()
        );
    }

    // ------------------------------------------------------------------
    // 4. Non-intersecting union preserves both
    // ------------------------------------------------------------------

    #[test]
    fn non_intersecting_union() {
        let a = make_box(Vec3::ZERO, Vec3::ONE);
        let b = make_box(Vec3::new(10.0, 0.0, 0.0), Vec3::ONE);

        let sop = BooleanSop;
        let params = BooleanParams {
            operation: BooleanOp::Union,
            ..Default::default()
        };

        let result = sop.execute(&[&a, &b], &params).unwrap();

        // Both boxes should be preserved — at least 16 points (8+8) and 12 prims (6+6).
        assert!(
            result.num_points() >= 16,
            "Non-intersecting union should have >= 16 points, got {}",
            result.num_points()
        );
        assert!(
            result.num_prims() >= 12,
            "Non-intersecting union should have >= 12 prims, got {}",
            result.num_prims()
        );

        println!(
            "Non-intersecting union: {} points, {} prims",
            result.num_points(),
            result.num_prims()
        );
    }

    // ------------------------------------------------------------------
    // 5. Seam operation — outputs polylines
    // ------------------------------------------------------------------

    #[test]
    fn seam_operation() {
        let a = make_box(Vec3::ZERO, Vec3::ONE);
        let b = make_box(Vec3::new(0.5, 0.0, 0.0), Vec3::ONE);

        let sop = BooleanSop;
        let params = BooleanParams {
            operation: BooleanOp::Seam,
            ..Default::default()
        };

        let result = sop.execute(&[&a, &b], &params).unwrap();

        assert!(
            result.num_prims() > 0,
            "Seam should produce polylines, got {} prims",
            result.num_prims()
        );
        assert!(
            result.num_points() > 0,
            "Seam should produce points, got {}",
            result.num_points()
        );

        println!(
            "Seam: {} points, {} prims",
            result.num_points(),
            result.num_prims()
        );
    }

    // ------------------------------------------------------------------
    // 6. Requires at least one input
    // ------------------------------------------------------------------

    #[test]
    fn requires_at_least_one_input() {
        let sop = BooleanSop;
        let params = BooleanParams::default();

        let result = sop.execute(&[], &params);
        assert!(result.is_err(), "BooleanSop should error with zero inputs");
    }

    // ── Additional tests ─────────────────────────────────────────────────

    #[test]
    fn shatter_operation() {
        // Two overlapping boxes with Shatter should produce fragments
        // (all triangles from both meshes, split at intersections).
        let a = make_box(Vec3::ZERO, Vec3::ONE);
        let b = make_box(Vec3::new(0.5, 0.0, 0.0), Vec3::ONE);

        let sop = BooleanSop;
        let params = BooleanParams {
            operation: BooleanOp::Shatter,
            ..Default::default()
        };

        let result = sop.execute(&[&a, &b], &params).unwrap();

        // Shatter keeps all fragments (both inside and outside), so we should
        // get at least as many prims as the inputs combined.
        let input_prims = a.num_prims() + b.num_prims();
        assert!(
            result.num_prims() >= input_prims,
            "Shatter should produce at least {} prims (from both inputs), got {}",
            input_prims,
            result.num_prims()
        );
        assert!(result.num_points() > 0, "Shatter should produce points");

        println!(
            "Shatter: {} points, {} prims (inputs had {})",
            result.num_points(),
            result.num_prims(),
            input_prims,
        );
    }

    #[test]
    fn custom_depth_operation() {
        // Custom mode with match=A, keeping all A fragments at any depth.
        // This is effectively a Resolve on A: all A fragments are kept.
        let a = make_box(Vec3::ZERO, Vec3::ONE);
        let b = make_box(Vec3::new(0.5, 0.0, 0.0), Vec3::ONE);

        let sop = BooleanSop;

        // Custom mode keeping only A fragments at any depth
        let params_a_all = BooleanParams {
            operation: BooleanOp::Custom,
            b_depth_range: [0, 9999],
            a_depth_range: [0, 9999],
            custom_match: CustomMatch::A,
            ..Default::default()
        };

        let result_a_all = sop.execute(&[&a, &b], &params_a_all).unwrap();

        assert!(
            result_a_all.num_points() > 0,
            "Custom match=A with full depth range should produce geometry"
        );
        assert!(
            result_a_all.num_prims() > 0,
            "Custom match=A with full depth range should produce prims"
        );

        // Custom mode keeping A fragments outside B (depth [0, 0]).
        // This should produce fewer prims than the all-depth version
        // since some A fragments are inside B and get excluded.
        let params_a_outside_b = BooleanParams {
            operation: BooleanOp::Custom,
            b_depth_range: [0, 0],
            a_depth_range: [0, 9999],
            custom_match: CustomMatch::A,
            ..Default::default()
        };

        let result_a_outside_b = sop.execute(&[&a, &b], &params_a_outside_b).unwrap();

        // A fragments outside B should produce some geometry (the non-overlapping part of A)
        assert!(
            result_a_outside_b.num_points() > 0,
            "Custom A-outside-B should produce geometry"
        );

        // The all-depth result should have at least as many prims as the filtered one
        assert!(
            result_a_all.num_prims() >= result_a_outside_b.num_prims(),
            "all-depth ({}) should have >= prims than outside-only ({})",
            result_a_all.num_prims(),
            result_a_outside_b.num_prims()
        );

        println!(
            "Custom A-all: {} pts {} prims; A-outside-B: {} pts {} prims",
            result_a_all.num_points(),
            result_a_all.num_prims(),
            result_a_outside_b.num_points(),
            result_a_outside_b.num_prims(),
        );
    }

    #[test]
    fn detect_operation() {
        // Detect mode: output has same point/prim count as input A, with
        // an intersection prim group.
        let a = make_box(Vec3::ZERO, Vec3::ONE);
        let b = make_box(Vec3::new(0.5, 0.0, 0.0), Vec3::ONE);

        let sop = BooleanSop;
        let params = BooleanParams {
            operation: BooleanOp::Detect,
            ..Default::default()
        };

        let result = sop.execute(&[&a, &b], &params).unwrap();

        // Same topology as A
        assert_eq!(
            result.num_points(),
            a.num_points(),
            "Detect should preserve A's point count"
        );
        assert_eq!(
            result.num_prims(),
            a.num_prims(),
            "Detect should preserve A's prim count"
        );

        // Should have an intersection group
        let group = result.groups().prim_group("axb_intersecting");
        assert!(
            group.is_some(),
            "Detect should create 'axb_intersecting' prim group"
        );

        // At least some prims should be in the group (the boxes overlap)
        let grp = group.unwrap();
        let intersecting_count = (0..result.num_prims()).filter(|&i| grp.contains(i)).count();
        assert!(
            intersecting_count > 0,
            "overlapping boxes should have intersecting prims, got 0"
        );

        println!(
            "Detect: {} intersecting prims out of {}",
            intersecting_count,
            result.num_prims()
        );
    }

    #[test]
    fn concentric_boxes_intersect() {
        // One box fully inside another. Intersect should produce the inner box.
        let outer = make_box(Vec3::ZERO, Vec3::splat(2.0));
        let inner = make_box(Vec3::ZERO, Vec3::splat(0.5));

        let sop = BooleanSop;
        let params = BooleanParams {
            operation: BooleanOp::Intersect,
            ..Default::default()
        };

        let result = sop.execute(&[&outer, &inner], &params).unwrap();

        assert!(
            result.num_points() > 0,
            "Intersect of concentric boxes should produce geometry"
        );
        assert!(
            result.num_prims() > 0,
            "Intersect of concentric boxes should produce prims"
        );

        // The bounding box of the result should be approximately the inner box size
        let bb = result.bounding_box();
        let extent_x = bb.max.x - bb.min.x;
        assert!(
            extent_x < 1.0,
            "intersection extent ({extent_x}) should be near the inner box size (0.5)"
        );

        println!(
            "Concentric intersect: {} points, {} prims, extent_x={}",
            result.num_points(),
            result.num_prims(),
            extent_x,
        );
    }

    #[test]
    fn identical_boxes_union() {
        // Union of the same box twice should produce approximately the same geometry
        // as a single box (no duplicated interior faces).
        let a = make_box(Vec3::ZERO, Vec3::ONE);
        let b = make_box(Vec3::ZERO, Vec3::ONE);

        let sop = BooleanSop;
        let params = BooleanParams {
            operation: BooleanOp::Union,
            ..Default::default()
        };

        let result = sop.execute(&[&a, &b], &params).unwrap();

        // The result should have geometry (not empty)
        assert!(
            result.num_points() > 0,
            "Union of identical boxes should produce geometry"
        );

        // The bounding box should be the same as a single box
        let bb = result.bounding_box();
        let a_bb = a.bounding_box();
        let extent_x = bb.max.x - bb.min.x;
        let a_extent_x = a_bb.max.x - a_bb.min.x;
        assert!(
            (extent_x - a_extent_x).abs() < 0.1,
            "union of identical boxes should have same extent, got {} vs {}",
            extent_x,
            a_extent_x
        );

        println!(
            "Identical union: {} points, {} prims",
            result.num_points(),
            result.num_prims()
        );
    }
}
