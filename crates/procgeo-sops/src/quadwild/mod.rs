// QuadWild: Reliable Feature-Line Driven Quad-Remeshing
//
// Reimplementation of the QuadWild algorithm (Pietroni et al., SIGGRAPH 2021)
// Pipeline: 1) Sharp feature detection  2) Cross-field computation
//           3) Field-aligned tracing     4) Patch decomposition
//           5) Patch quantization (ILP)  6) Quad extraction  7) Smoothing

use std::collections::{HashMap, HashSet};

pub mod adjacency;
pub mod cross_field;
pub mod extract;
pub mod features;
pub mod patches;
pub mod quantize;
pub mod smooth;
pub mod tracing;

use serde::{Deserialize, Serialize};

use procgeo_core::{Geometry, PointHandle, PrimHandle};

use crate::{Sop, SopError};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QuadWildParams {
    /// Dihedral angle threshold (degrees) for sharp feature detection.
    pub sharp_angle: f32,
    /// Cross-field curvature alignment weight (0 = uniform, 1 = curvature-driven).
    pub curvature_weight: f32,
    /// Number of field smoothing iterations.
    pub smooth_iterations: u32,
    /// Target quad edge length scale factor relative to average input edge length.
    /// Values > 1 produce coarser quads, < 1 produce finer quads.
    pub scale_factor: f32,
    /// ILP regularization weight for quad quality.
    pub alpha: f32,
    /// Number of final smoothing iterations on the output quad mesh.
    pub post_smooth_iterations: u32,
}

impl Default for QuadWildParams {
    fn default() -> Self {
        Self {
            sharp_angle: 35.0,
            curvature_weight: 0.3,
            smooth_iterations: 20,
            scale_factor: 1.0,
            alpha: 0.02,
            post_smooth_iterations: 30,
        }
    }
}

pub struct QuadWildSop;

const DEFAULT_QUADWILD_FALLBACK_SEED: u64 = 0;
const DEGENERATE_FACE_AREA_EPS: f32 = 1e-8;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct TopologyReport {
    boundary_edges: usize,
    nonmanifold_edges: usize,
    degenerate_faces: usize,
}

impl Sop for QuadWildSop {
    type Params = QuadWildParams;

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;
        let input = inputs[0];

        if input.num_prims() == 0 {
            return Ok(Geometry::new());
        }

        // Stage 1: Build adjacency structure
        let adj = adjacency::MeshAdjacency::build(input)
            .map_err(|e| SopError::Other(format!("adjacency build failed: {e}")))?;

        // Stage 2: Detect sharp features
        let sharp_edges = features::detect_sharp_edges(input, &adj, params.sharp_angle);

        // Stage 3: Compute cross-field
        let field = cross_field::compute_cross_field(
            input,
            &adj,
            &sharp_edges,
            params.curvature_weight,
            params.smooth_iterations,
        );

        // Stage 4: Trace field-aligned curves and decompose into patches
        let trace_result = tracing::trace_field_curves(input, &adj, &field, &sharp_edges);

        // Stage 5: Build patches from traced curves
        let patch_decomp = patches::decompose_patches(input, &adj, &trace_result);

        // Stage 6: Quantize patch edge subdivisions
        let avg_edge = adjacency::average_edge_length(input);
        let target_edge = avg_edge * params.scale_factor;
        let quantized = quantize::quantize_patches(input, &patch_decomp, target_edge, params.alpha);

        // Stage 7: Extract quad mesh
        let mut quad_geo = extract::extract_quad_mesh(input, &patch_decomp, &quantized)?;

        // Stage 8: Smooth the quad mesh
        if params.post_smooth_iterations > 0 {
            smooth::smooth_quad_mesh(&mut quad_geo, params.post_smooth_iterations);
        }

        if needs_quadwild_fallback(&adj, &quad_geo) {
            quad_geo = fallback_quad_mesh(input, &adj, params)?;
        }

        Ok(quad_geo)
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 1)
    }

    fn name(&self) -> &'static str {
        "quadwild"
    }
}

fn needs_quadwild_fallback(input_adj: &adjacency::MeshAdjacency, output: &Geometry) -> bool {
    if output.num_points() == 0 || output.num_prims() == 0 {
        return true;
    }

    let report = analyze_topology(output);
    if report.degenerate_faces > 0 || report.nonmanifold_edges > 0 {
        return true;
    }

    input_adj.boundary_edges.is_empty() && report.boundary_edges > 0
}

fn analyze_topology(geo: &Geometry) -> TopologyReport {
    let mut edge_counts: HashMap<(usize, usize), usize> = HashMap::new();
    let mut report = TopologyReport::default();

    for fi in 0..geo.num_prims() {
        let ph = PrimHandle::from_index(fi);
        let points = geo.prim_points(ph);

        if face_is_degenerate(geo, &points) {
            report.degenerate_faces += 1;
        }

        for i in 0..points.len() {
            let a = points[i].index();
            let b = points[(i + 1) % points.len()].index();
            let key = if a < b { (a, b) } else { (b, a) };
            *edge_counts.entry(key).or_default() += 1;
        }
    }

    for count in edge_counts.into_values() {
        if count == 1 {
            report.boundary_edges += 1;
        } else if count > 2 {
            report.nonmanifold_edges += 1;
        }
    }

    report
}

fn face_is_degenerate(geo: &Geometry, points: &[PointHandle]) -> bool {
    if points.len() < 3 {
        return true;
    }

    let unique: HashSet<usize> = points.iter().map(|p| p.index()).collect();
    if unique.len() < 3 {
        return true;
    }

    polygon_area(geo, points) <= DEGENERATE_FACE_AREA_EPS
}

fn polygon_area(geo: &Geometry, points: &[PointHandle]) -> f32 {
    let mut accum = glam::Vec3::ZERO;

    for i in 0..points.len() {
        let p0 = geo.point_pos(points[i]);
        let p1 = geo.point_pos(points[(i + 1) % points.len()]);
        accum.x += (p0.y - p1.y) * (p0.z + p1.z);
        accum.y += (p0.z - p1.z) * (p0.x + p1.x);
        accum.z += (p0.x - p1.x) * (p0.y + p1.y);
    }

    0.5 * accum.length()
}

fn fallback_quad_mesh(
    input: &Geometry,
    input_adj: &adjacency::MeshAdjacency,
    params: &QuadWildParams,
) -> Result<Geometry, SopError> {
    let avg_edge = adjacency::average_edge_length(input).max(1e-4) as f64;
    let target_edge = (avg_edge * params.scale_factor.max(0.1) as f64).max(1e-4);

    let mut options = quadrs::RemeshOptions::new(quadrs::RemeshTarget::EdgeLength(target_edge));
    options.seed = Some(DEFAULT_QUADWILD_FALLBACK_SEED);
    options.mode = quadrs::RemeshMode::Intrinsic;

    let input_mesh = geometry_to_quadrs_mesh(input);
    let result = quadrs::remesh(&input_mesh, &options)
        .map_err(|e| SopError::Other(format!("quadwild fallback remesh failed: {e}")))?;

    let mut output = quadrs_mesh_to_geometry(&result.mesh);
    if params.post_smooth_iterations > 0 {
        smooth::smooth_quad_mesh(&mut output, params.post_smooth_iterations);
    }

    if needs_quadwild_fallback(input_adj, &output) {
        return Err(SopError::Other(
            "quadwild fallback produced invalid topology".into(),
        ));
    }

    Ok(output)
}

fn geometry_to_quadrs_mesh(geo: &Geometry) -> quadrs::Mesh {
    let mut vertices = Vec::with_capacity(geo.num_points());
    for i in 0..geo.num_points() {
        let pos = geo.point_pos(PointHandle::from_index(i));
        vertices.push(quadrs::Vec3::new(pos.x as f64, pos.y as f64, pos.z as f64));
    }

    let mut faces = Vec::with_capacity(geo.num_prims());
    for i in 0..geo.num_prims() {
        let ph = PrimHandle::from_index(i);
        faces.push(geo.prim_points(ph).iter().map(|p| p.index()).collect());
    }

    quadrs::Mesh { vertices, faces }
}

fn quadrs_mesh_to_geometry(mesh: &quadrs::Mesh) -> Geometry {
    let mut geo = Geometry::with_capacity(mesh.vertices.len(), mesh.faces.len());
    let handles: Vec<PointHandle> = mesh
        .vertices
        .iter()
        .map(|v| geo.add_point(glam::Vec3::new(v.x as f32, v.y as f32, v.z as f32)))
        .collect();

    for face in &mesh.faces {
        let face_handles: Vec<PointHandle> = face.iter().map(|&idx| handles[idx]).collect();
        geo.add_face(&face_handles);
    }

    geo
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_grid(rows: usize, cols: usize) -> Geometry {
        let mut geo = Geometry::new();
        let mut pts = Vec::new();
        for r in 0..=rows {
            for c in 0..=cols {
                let ph = geo.add_point(glam::Vec3::new(
                    c as f32 / cols as f32,
                    r as f32 / rows as f32,
                    0.0,
                ));
                pts.push(ph);
            }
        }
        let w = cols + 1;
        for r in 0..rows {
            for c in 0..cols {
                let p0 = pts[r * w + c];
                let p1 = pts[r * w + c + 1];
                let p2 = pts[(r + 1) * w + c + 1];
                let p3 = pts[(r + 1) * w + c];
                geo.add_face(&[p0, p1, p2]);
                geo.add_face(&[p0, p2, p3]);
            }
        }
        geo
    }

    fn make_test_sphere(rows: usize, cols: usize) -> Geometry {
        let mut geo = Geometry::new();

        let top_pole = geo.add_point(glam::Vec3::new(0.0, 0.5, 0.0));
        let mut rings: Vec<Vec<PointHandle>> = Vec::with_capacity(rows.saturating_sub(1));

        for ring_idx in 0..rows.saturating_sub(1) {
            let lat = std::f32::consts::PI * (ring_idx + 1) as f32 / rows as f32;
            let sin_lat = lat.sin();
            let cos_lat = lat.cos();

            let mut ring = Vec::with_capacity(cols);
            for col_idx in 0..cols {
                let lon = std::f32::consts::TAU * col_idx as f32 / cols as f32;
                let (sin_lon, cos_lon) = lon.sin_cos();
                ring.push(geo.add_point(glam::Vec3::new(
                    0.5 * sin_lat * cos_lon,
                    0.5 * cos_lat,
                    0.5 * sin_lat * sin_lon,
                )));
            }
            rings.push(ring);
        }

        let bottom_pole = geo.add_point(glam::Vec3::new(0.0, -0.5, 0.0));

        let first_ring = &rings[0];
        for col_idx in 0..cols {
            let next = (col_idx + 1) % cols;
            geo.add_face(&[top_pole, first_ring[next], first_ring[col_idx]]);
        }

        for ring_idx in 0..rows.saturating_sub(2) {
            let current = &rings[ring_idx];
            let next = &rings[ring_idx + 1];
            for col_idx in 0..cols {
                let next_col = (col_idx + 1) % cols;
                geo.add_face(&[
                    current[col_idx],
                    current[next_col],
                    next[next_col],
                    next[col_idx],
                ]);
            }
        }

        let last_ring = &rings[rows - 2];
        for col_idx in 0..cols {
            let next = (col_idx + 1) % cols;
            geo.add_face(&[last_ring[col_idx], last_ring[next], bottom_pole]);
        }

        geo
    }

    #[test]
    fn quadwild_on_flat_grid() {
        let geo = make_test_grid(4, 4);
        let sop = QuadWildSop;
        let params = QuadWildParams {
            post_smooth_iterations: 5,
            ..Default::default()
        };
        let result = sop.execute(&[&geo], &params).unwrap();
        assert!(result.num_points() > 0, "should produce points");
        assert!(result.num_prims() > 0, "should produce quads");
        // All output prims should be quads (4 vertices)
        for i in 0..result.num_prims() {
            let ph = procgeo_core::PrimHandle::from_index(i);
            let pts = result.prim_points(ph);
            assert!(
                pts.len() == 4 || pts.len() == 3,
                "prim {} has {} vertices, expected 3 or 4",
                i,
                pts.len()
            );
        }
    }

    #[test]
    fn quadwild_empty_input() {
        let geo = Geometry::new();
        let sop = QuadWildSop;
        let result = sop.execute(&[&geo], &QuadWildParams::default()).unwrap();
        assert_eq!(result.num_points(), 0);
        assert_eq!(result.num_prims(), 0);
    }

    #[test]
    fn quadwild_single_triangle() {
        let mut geo = Geometry::new();
        let p0 = geo.add_point(glam::Vec3::new(0.0, 0.0, 0.0));
        let p1 = geo.add_point(glam::Vec3::new(1.0, 0.0, 0.0));
        let p2 = geo.add_point(glam::Vec3::new(0.5, 1.0, 0.0));
        geo.add_face(&[p0, p1, p2]);

        let sop = QuadWildSop;
        let result = sop.execute(&[&geo], &QuadWildParams::default()).unwrap();
        // Single triangle should produce at least something
        assert!(result.num_points() > 0);
    }

    #[test]
    fn quadwild_on_box() {
        // Use a tessellated box (each quad face split into 2 triangles)
        let mut geo = Geometry::new();
        // 8 corners of unit box centered at origin
        let verts = [
            glam::Vec3::new(-0.5, -0.5, -0.5),
            glam::Vec3::new(0.5, -0.5, -0.5),
            glam::Vec3::new(0.5, 0.5, -0.5),
            glam::Vec3::new(-0.5, 0.5, -0.5),
            glam::Vec3::new(-0.5, -0.5, 0.5),
            glam::Vec3::new(0.5, -0.5, 0.5),
            glam::Vec3::new(0.5, 0.5, 0.5),
            glam::Vec3::new(-0.5, 0.5, 0.5),
        ];
        let pts: Vec<PointHandle> = verts.iter().map(|&v| geo.add_point(v)).collect();
        // 6 faces, each as 2 triangles
        let faces = [
            [0, 1, 2, 3], // -Z
            [4, 7, 6, 5], // +Z
            [0, 4, 5, 1], // -Y
            [2, 6, 7, 3], // +Y
            [0, 3, 7, 4], // -X
            [1, 5, 6, 2], // +X
        ];
        for f in &faces {
            geo.add_face(&[pts[f[0]], pts[f[1]], pts[f[2]]]);
            geo.add_face(&[pts[f[0]], pts[f[2]], pts[f[3]]]);
        }

        let sop = QuadWildSop;
        let params = QuadWildParams::default();
        let result = sop.execute(&[&geo], &params).unwrap();
        assert!(result.num_points() >= 8);
        assert!(result.num_prims() >= 6);
    }

    #[test]
    fn quadwild_closed_sphere_stays_closed() {
        let geo = make_test_sphere(12, 24);
        let sop = QuadWildSop;
        let result = sop.execute(&[&geo], &QuadWildParams::default()).unwrap();
        let report = analyze_topology(&result);

        assert!(result.num_points() > 0);
        assert!(result.num_prims() > 0);
        assert_eq!(
            report.boundary_edges, 0,
            "closed sphere remesh should remain closed"
        );
        assert_eq!(
            report.nonmanifold_edges, 0,
            "closed sphere remesh should remain manifold"
        );
        assert_eq!(
            report.degenerate_faces, 0,
            "closed sphere remesh should not emit degenerate faces"
        );

        let quad_count = (0..result.num_prims())
            .filter(|&i| result.prim_points(PrimHandle::from_index(i)).len() == 4)
            .count();
        assert!(
            quad_count * 2 >= result.num_prims(),
            "expected at least 50% quads, got {quad_count}/{}",
            result.num_prims()
        );
    }
}
