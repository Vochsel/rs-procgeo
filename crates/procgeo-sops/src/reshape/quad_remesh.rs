use serde::{Deserialize, Serialize};

use procgeo_core::{Geometry, PointHandle, PrimHandle};

use crate::{Sop, SopError};

/// Target mode for quad remeshing.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
pub enum QuadRemeshTarget {
    /// Target a specific number of output quad faces.
    #[default]
    FaceCount,
    /// Target a specific number of output vertices.
    VertexCount,
    /// Target a specific edge length in world units.
    EdgeLength,
}

/// Optimization mode for orientation and position field smoothing.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
pub enum QuadRemeshMode {
    /// Intrinsic smoothing (default).
    #[default]
    Intrinsic,
    /// Extrinsic smoothing.
    Extrinsic,
}

/// Parameters for the QuadRemesh SOP.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QuadRemeshParams {
    /// How to interpret the target value.
    pub target_mode: QuadRemeshTarget,
    /// Target count when `target_mode` is `FaceCount` or `VertexCount`.
    pub target_count: u32,
    /// Target edge length when `target_mode` is `EdgeLength`.
    pub target_edge_length: f64,
    /// Optional seed for deterministic results.
    pub seed: Option<u64>,
    /// Optimization mode.
    pub mode: QuadRemeshMode,
}

impl Default for QuadRemeshParams {
    fn default() -> Self {
        Self {
            target_mode: QuadRemeshTarget::FaceCount,
            target_count: 1000,
            target_edge_length: 0.1,
            seed: None,
            mode: QuadRemeshMode::Intrinsic,
        }
    }
}

pub struct QuadRemeshSop;

/// Convert procgeo Geometry into a `quadrs::Mesh`.
fn geometry_to_quadrs_mesh(geo: &Geometry) -> quadrs::Mesh {
    let num_pts = geo.num_points();
    let mut vertices = Vec::with_capacity(num_pts);
    for i in 0..num_pts {
        let pos = geo.point_pos(PointHandle::from_index(i));
        vertices.push(quadrs::Vec3::new(pos.x as f64, pos.y as f64, pos.z as f64));
    }

    let num_prims = geo.num_prims();
    let mut faces = Vec::with_capacity(num_prims);
    for i in 0..num_prims {
        let ph = PrimHandle::from_index(i);
        let pts = geo.prim_points(ph);
        faces.push(pts.iter().map(|p| p.index()).collect());
    }

    quadrs::Mesh { vertices, faces }
}

/// Convert a `quadrs::Mesh` back into procgeo Geometry.
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

impl Sop for QuadRemeshSop {
    type Params = QuadRemeshParams;

    fn name(&self) -> &'static str {
        "quadremesh"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 1)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;

        let input_geo = inputs[0];
        if input_geo.num_points() == 0 || input_geo.num_prims() == 0 {
            return Err(SopError::Other("quad remesh requires non-empty geometry".into()));
        }

        let mesh = geometry_to_quadrs_mesh(input_geo);

        let target = match params.target_mode {
            QuadRemeshTarget::FaceCount => {
                quadrs::RemeshTarget::FaceCount(params.target_count as usize)
            }
            QuadRemeshTarget::VertexCount => {
                quadrs::RemeshTarget::VertexCount(params.target_count as usize)
            }
            QuadRemeshTarget::EdgeLength => {
                quadrs::RemeshTarget::EdgeLength(params.target_edge_length)
            }
        };

        let mut options = quadrs::RemeshOptions::new(target);
        options.seed = params.seed;
        options.mode = match params.mode {
            QuadRemeshMode::Intrinsic => quadrs::RemeshMode::Intrinsic,
            QuadRemeshMode::Extrinsic => quadrs::RemeshMode::Extrinsic,
        };

        let result = quadrs::remesh(&mesh, &options).map_err(|e| {
            SopError::Other(format!("quad remesh failed: {e}"))
        })?;

        Ok(quadrs_mesh_to_geometry(&result.mesh))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creation::box_sop::{BoxSop, BoxParams};
    use crate::creation::grid::{GridSop, GridParams};
    use crate::creation::sphere::{SphereSop, SphereParams};
    use crate::reshape::subdivide::{SubdivideSop, SubdivideParams};
    use crate::{GeometryExt, generate};

    #[test]
    fn quad_remesh_basic() {
        // Subdivide a box to get enough geometry, then remesh to quads
        let geo = generate(&BoxSop, &BoxParams::default())
            .unwrap()
            .apply(
                &SubdivideSop,
                &SubdivideParams {
                    depth: 2,
                    ..Default::default()
                },
            )
            .unwrap();

        let params = QuadRemeshParams {
            target_mode: QuadRemeshTarget::FaceCount,
            target_count: 20,
            seed: Some(42),
            ..Default::default()
        };

        let result = geo.apply(&QuadRemeshSop, &params).unwrap();
        assert!(result.num_points() > 0, "output should have points");
        assert!(result.num_prims() > 0, "output should have faces");
    }

    #[test]
    fn quad_remesh_produces_quads() {
        // Use a sphere with enough faces to remesh
        let geo = generate(
            &SphereSop,
            &SphereParams {
                rows: 16,
                cols: 32,
                ..Default::default()
            },
        )
        .unwrap();

        let params = QuadRemeshParams {
            target_mode: QuadRemeshTarget::FaceCount,
            target_count: 50,
            seed: Some(123),
            ..Default::default()
        };

        let result = geo.apply(&QuadRemeshSop, &params).unwrap();

        // Count quads vs non-quads
        let mut quad_count = 0;
        let mut total = 0;
        for i in 0..result.num_prims() {
            let pts = result.prim_points(PrimHandle::from_index(i));
            total += 1;
            if pts.len() == 4 {
                quad_count += 1;
            }
        }

        // The output should be predominantly quads
        let ratio = quad_count as f64 / total as f64;
        assert!(
            ratio > 0.8,
            "expected >80% quads, got {:.1}% ({quad_count}/{total})",
            ratio * 100.0
        );
    }

    #[test]
    fn quad_remesh_vertex_count_target() {
        let geo = generate(
            &SphereSop,
            &SphereParams {
                rows: 16,
                cols: 32,
                ..Default::default()
            },
        )
        .unwrap();

        let params = QuadRemeshParams {
            target_mode: QuadRemeshTarget::VertexCount,
            target_count: 100,
            seed: Some(7),
            ..Default::default()
        };

        let result = geo.apply(&QuadRemeshSop, &params).unwrap();
        assert!(result.num_points() > 0);
        assert!(result.num_prims() > 0);
    }

    #[test]
    fn quad_remesh_edge_length_target() {
        let geo = generate(
            &SphereSop,
            &SphereParams {
                rows: 16,
                cols: 32,
                ..Default::default()
            },
        )
        .unwrap();

        let params = QuadRemeshParams {
            target_mode: QuadRemeshTarget::EdgeLength,
            target_edge_length: 0.5,
            seed: Some(99),
            ..Default::default()
        };

        let result = geo.apply(&QuadRemeshSop, &params).unwrap();
        assert!(result.num_points() > 0);
        assert!(result.num_prims() > 0);
    }

    #[test]
    fn quad_remesh_with_seed_produces_valid_output() {
        let geo = generate(
            &SphereSop,
            &SphereParams {
                rows: 12,
                cols: 24,
                ..Default::default()
            },
        )
        .unwrap();

        let params = QuadRemeshParams {
            target_mode: QuadRemeshTarget::FaceCount,
            target_count: 30,
            seed: Some(42),
            ..Default::default()
        };

        let result = geo.apply(&QuadRemeshSop, &params).unwrap();
        assert!(result.num_points() > 0, "seeded remesh should produce points");
        assert!(result.num_prims() > 0, "seeded remesh should produce faces");
    }

    #[test]
    fn quad_remesh_empty_geometry_errors() {
        let geo = Geometry::new();
        let params = QuadRemeshParams::default();
        let result = QuadRemeshSop.execute(&[&geo], &params);
        assert!(result.is_err());
    }

    #[test]
    fn quad_remesh_grid() {
        let geo = generate(
            &GridSop,
            &GridParams {
                rows: 20,
                cols: 20,
                ..Default::default()
            },
        )
        .unwrap();

        let params = QuadRemeshParams {
            target_mode: QuadRemeshTarget::FaceCount,
            target_count: 40,
            seed: Some(1),
            ..Default::default()
        };

        let result = geo.apply(&QuadRemeshSop, &params).unwrap();
        assert!(result.num_points() > 0);
        assert!(result.num_prims() > 0);
    }

    #[test]
    fn quad_remesh_extrinsic_mode() {
        let geo = generate(
            &SphereSop,
            &SphereParams {
                rows: 12,
                cols: 24,
                ..Default::default()
            },
        )
        .unwrap();

        let params = QuadRemeshParams {
            target_mode: QuadRemeshTarget::FaceCount,
            target_count: 30,
            seed: Some(55),
            mode: QuadRemeshMode::Extrinsic,
            ..Default::default()
        };

        let result = geo.apply(&QuadRemeshSop, &params).unwrap();
        assert!(result.num_points() > 0);
        assert!(result.num_prims() > 0);
    }
}
