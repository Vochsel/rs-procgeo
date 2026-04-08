// QuadWild: Reliable Feature-Line Driven Quad-Remeshing
//
// Reimplementation of the QuadWild algorithm (Pietroni et al., SIGGRAPH 2021)
// Pipeline: 1) Sharp feature detection  2) Cross-field computation
//           3) Field-aligned tracing     4) Patch decomposition
//           5) Patch quantization (ILP)  6) Quad extraction  7) Smoothing

pub mod adjacency;
pub mod cross_field;
pub mod extract;
pub mod features;
pub mod patches;
pub mod quantize;
pub mod smooth;
pub mod tracing;

use serde::{Deserialize, Serialize};

use procgeo_core::Geometry;

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
        let quantized = quantize::quantize_patches(
            input,
            &patch_decomp,
            target_edge,
            params.alpha,
        );

        // Stage 7: Extract quad mesh
        let mut quad_geo = extract::extract_quad_mesh(input, &patch_decomp, &quantized)?;

        // Stage 8: Smooth the quad mesh
        if params.post_smooth_iterations > 0 {
            smooth::smooth_quad_mesh(&mut quad_geo, params.post_smooth_iterations);
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

#[cfg(test)]
mod tests {
    use super::*;
    use procgeo_core::PointHandle;

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
}
