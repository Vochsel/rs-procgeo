use std::collections::HashMap;

use glam::Vec3;
use serde::{Deserialize, Serialize};

use procgeo_core::Geometry;

use crate::{Sop, SopError};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TeapotParams {
    pub size: Vec3,
    pub center: Vec3,
    pub resolution: u32,
}

impl Default for TeapotParams {
    fn default() -> Self {
        Self {
            size: Vec3::ONE,
            center: Vec3::ZERO,
            resolution: 6,
        }
    }
}

pub struct TeapotSop;

#[derive(Default)]
struct MeshBuilder {
    vertices: Vec<Vec3>,
    faces: Vec<Vec<usize>>,
    vertex_map: HashMap<(u32, u32, u32), usize>,
}

impl MeshBuilder {
    fn add_vertex(&mut self, pos: Vec3) -> usize {
        let key = (
            canonical_bits(pos.x),
            canonical_bits(pos.y),
            canonical_bits(pos.z),
        );
        if let Some(&idx) = self.vertex_map.get(&key) {
            return idx;
        }

        let idx = self.vertices.len();
        self.vertices.push(pos);
        self.vertex_map.insert(key, idx);
        idx
    }

    fn add_face(&mut self, indices: &[usize]) {
        let mut face = Vec::with_capacity(indices.len());
        for &idx in indices {
            if !face.contains(&idx) {
                face.push(idx);
            }
        }
        if face.len() >= 3 {
            self.faces.push(face);
        }
    }
}

fn canonical_bits(value: f32) -> u32 {
    let normalized = if value == 0.0 { 0.0 } else { value };
    normalized.to_bits()
}

fn bernstein(t: f32) -> [f32; 4] {
    let omt = 1.0 - t;
    [
        omt * omt * omt,
        3.0 * t * omt * omt,
        3.0 * t * t * omt,
        t * t * t,
    ]
}

fn source_point(index: usize, reflect_x: bool, reflect_source_y: bool) -> Vec3 {
    let [mut x, mut y, z] = TEAPOT_CP[index];
    if reflect_x {
        x = -x;
    }
    if reflect_source_y {
        y = -y;
    }

    // The source dataset is Z-up. Rotate it into procgeo's Y-up convention
    // without changing handedness so face winding stays consistent.
    Vec3::new(x, z, -y)
}

fn control_patch(patch: [usize; 16], reflect_x: bool, reflect_source_y: bool) -> [[Vec3; 4]; 4] {
    let mut control = [[Vec3::ZERO; 4]; 4];
    for row in 0..4 {
        for col in 0..4 {
            control[row][col] = source_point(patch[row * 4 + col], reflect_x, reflect_source_y);
        }
    }
    control
}

fn eval_patch(control: &[[Vec3; 4]; 4], u: f32, v: f32) -> Vec3 {
    let bu = bernstein(u);
    let bv = bernstein(v);
    let mut pos = Vec3::ZERO;

    for row in 0..4 {
        for col in 0..4 {
            pos += control[row][col] * bu[row] * bv[col];
        }
    }

    pos
}

fn tessellate_patch(
    builder: &mut MeshBuilder,
    patch: [usize; 16],
    reflect_x: bool,
    reflect_source_y: bool,
    resolution: usize,
) {
    let control = control_patch(patch, reflect_x, reflect_source_y);
    let mirrored = (reflect_x as u8 + reflect_source_y as u8) % 2 == 1;
    let mut grid = vec![vec![0usize; resolution + 1]; resolution + 1];

    for u_idx in 0..=resolution {
        let u = u_idx as f32 / resolution as f32;
        for v_idx in 0..=resolution {
            let v = v_idx as f32 / resolution as f32;
            let pos = eval_patch(&control, u, v);
            grid[u_idx][v_idx] = builder.add_vertex(pos);
        }
    }

    for u_idx in 0..resolution {
        for v_idx in 0..resolution {
            let a = grid[u_idx][v_idx];
            let b = grid[u_idx][v_idx + 1];
            let c = grid[u_idx + 1][v_idx + 1];
            let d = grid[u_idx + 1][v_idx];
            if mirrored {
                builder.add_face(&[d, c, b, a]);
            } else {
                builder.add_face(&[a, b, c, d]);
            }
        }
    }
}

fn build_teapot_mesh(resolution: usize) -> MeshBuilder {
    let mut builder = MeshBuilder::default();

    for &patch in &TEAPOT_PATCHES[..6] {
        tessellate_patch(&mut builder, patch, false, false, resolution);
        tessellate_patch(&mut builder, patch, false, true, resolution);
        tessellate_patch(&mut builder, patch, true, false, resolution);
        tessellate_patch(&mut builder, patch, true, true, resolution);
    }

    for &patch in &TEAPOT_PATCHES[6..] {
        tessellate_patch(&mut builder, patch, false, false, resolution);
        tessellate_patch(&mut builder, patch, false, true, resolution);
    }

    builder
}

impl Sop for TeapotSop {
    type Params = TeapotParams;

    fn name(&self) -> &'static str {
        "teapot"
    }

    fn input_count(&self) -> (usize, usize) {
        (0, 0)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;

        if params.resolution < 2 {
            return Err(SopError::InvalidParam(format!(
                "resolution must be >= 2, got {}",
                params.resolution
            )));
        }
        if params.size.x <= 0.0 || params.size.y <= 0.0 || params.size.z <= 0.0 {
            return Err(SopError::InvalidParam(
                "size components must be > 0".to_string(),
            ));
        }

        let mesh = build_teapot_mesh(params.resolution as usize);

        let mut bb_min = Vec3::splat(f32::INFINITY);
        let mut bb_max = Vec3::splat(f32::NEG_INFINITY);
        for &pos in &mesh.vertices {
            bb_min = bb_min.min(pos);
            bb_max = bb_max.max(pos);
        }

        let raw_center = (bb_min + bb_max) * 0.5;
        let raw_size = (bb_max - bb_min).max(Vec3::splat(1.0e-6));
        let scale = params.size / raw_size.y.max(1.0e-6);

        let mut geo = Geometry::with_capacity(mesh.vertices.len(), mesh.faces.len());
        let handles: Vec<_> = mesh
            .vertices
            .iter()
            .map(|&pos| geo.add_point((pos - raw_center) * scale + params.center))
            .collect();

        for face in mesh.faces {
            let point_handles: Vec<_> = face.into_iter().map(|idx| handles[idx]).collect();
            geo.add_face(&point_handles);
        }

        Ok(geo)
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use procgeo_core::PrimHandle;

    use super::*;
    use crate::generate;

    fn prim_normal(geo: &Geometry, prim_idx: usize) -> Vec3 {
        let points = geo.prim_points(PrimHandle::from_index(prim_idx));
        let mut normal = Vec3::ZERO;
        for i in 0..points.len() {
            let a = geo.point_pos(points[i]);
            let b = geo.point_pos(points[(i + 1) % points.len()]);
            normal.x += (a.y - b.y) * (a.z + b.z);
            normal.y += (a.z - b.z) * (a.x + b.x);
            normal.z += (a.x - b.x) * (a.y + b.y);
        }
        normal
    }

    #[test]
    fn teapot_default() {
        let geo = generate(&TeapotSop, &TeapotParams::default()).unwrap();

        assert!(geo.num_points() > 700);
        assert!(geo.num_prims() > 1_000);

        let bb = geo.bounding_box();
        assert_relative_eq!((bb.min.x + bb.max.x) * 0.5, 0.0, epsilon = 1e-5);
        assert_relative_eq!((bb.min.y + bb.max.y) * 0.5, 0.0, epsilon = 1e-5);
        assert_relative_eq!((bb.min.z + bb.max.z) * 0.5, 0.0, epsilon = 1e-5);
        assert_relative_eq!(bb.max.y - bb.min.y, 1.0, epsilon = 1e-5);
        assert!(bb.max.x - bb.min.x > 1.0);
        assert!(bb.max.z - bb.min.z > 1.0);
    }

    #[test]
    fn teapot_custom_size_and_center() {
        let default = generate(
            &TeapotSop,
            &TeapotParams {
                resolution: 4,
                ..Default::default()
            },
        )
        .unwrap();
        let geo = generate(
            &TeapotSop,
            &TeapotParams {
                size: Vec3::new(2.0, 4.0, 3.0),
                center: Vec3::new(1.0, 2.0, -1.0),
                resolution: 4,
            },
        )
        .unwrap();

        let bb = geo.bounding_box();
        let default_bb = default.bounding_box();
        assert_relative_eq!((bb.min.x + bb.max.x) * 0.5, 1.0, epsilon = 1e-5);
        assert_relative_eq!((bb.min.y + bb.max.y) * 0.5, 2.0, epsilon = 1e-5);
        assert_relative_eq!((bb.min.z + bb.max.z) * 0.5, -1.0, epsilon = 1e-5);
        assert_relative_eq!(
            bb.max.x - bb.min.x,
            (default_bb.max.x - default_bb.min.x) * 2.0,
            epsilon = 1e-5
        );
        assert_relative_eq!(
            bb.max.y - bb.min.y,
            (default_bb.max.y - default_bb.min.y) * 4.0,
            epsilon = 1e-5
        );
        assert_relative_eq!(
            bb.max.z - bb.min.z,
            (default_bb.max.z - default_bb.min.z) * 3.0,
            epsilon = 1e-5
        );
    }

    #[test]
    fn teapot_resolution_increases_detail() {
        let low = generate(
            &TeapotSop,
            &TeapotParams {
                resolution: 3,
                ..Default::default()
            },
        )
        .unwrap();
        let high = generate(
            &TeapotSop,
            &TeapotParams {
                resolution: 7,
                ..Default::default()
            },
        )
        .unwrap();

        assert!(high.num_points() > low.num_points());
        assert!(high.num_prims() > low.num_prims());
    }

    #[test]
    fn teapot_top_faces_point_up() {
        let geo = generate(
            &TeapotSop,
            &TeapotParams {
                resolution: 4,
                ..Default::default()
            },
        )
        .unwrap();

        let mut top_prim = 0usize;
        let mut top_y = f32::NEG_INFINITY;
        for prim_idx in 0..geo.num_prims() {
            let points = geo.prim_points(PrimHandle::from_index(prim_idx));
            let center_y =
                points.iter().map(|&p| geo.point_pos(p).y).sum::<f32>() / points.len() as f32;
            if center_y > top_y {
                top_y = center_y;
                top_prim = prim_idx;
            }
        }

        let normal = prim_normal(&geo, top_prim).normalize_or_zero();
        assert!(normal.y > 0.0, "top face should point upward");
    }

    #[test]
    fn teapot_rejects_invalid_params() {
        assert!(
            generate(
                &TeapotSop,
                &TeapotParams {
                    resolution: 1,
                    ..Default::default()
                }
            )
            .is_err()
        );
        assert!(
            generate(
                &TeapotSop,
                &TeapotParams {
                    size: Vec3::new(1.0, 0.0, 1.0),
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
}

// Adapted from freeglut_teapot_data.h, which is distributed under a permissive
// MIT-style notice and includes the original SGI teapot permission notice.
// Source: https://sources.debian.org/src/fltk1.1/1.1.10-23/src/freeglut_teapot_data.h
const TEAPOT_PATCHES: [[usize; 16]; 10] = [
    [102, 103, 104, 105, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    [
        12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27,
    ],
    [
        24, 25, 26, 27, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40,
    ],
    [
        96, 96, 96, 96, 97, 98, 99, 100, 101, 101, 101, 101, 0, 1, 2, 3,
    ],
    [
        0, 1, 2, 3, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117,
    ],
    [
        118, 118, 118, 118, 124, 122, 119, 121, 123, 126, 125, 120, 40, 39, 38, 37,
    ],
    [
        41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56,
    ],
    [
        53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 28, 65, 66, 67,
    ],
    [
        68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83,
    ],
    [
        80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95,
    ],
];

const TEAPOT_CP: [[f32; 3]; 127] = [
    [0.2_f32, 0_f32, 2.7_f32],
    [0.2_f32, -0.112_f32, 2.7_f32],
    [0.112_f32, -0.2_f32, 2.7_f32],
    [0_f32, -0.2_f32, 2.7_f32],
    [1.3375_f32, 0_f32, 2.53125_f32],
    [1.3375_f32, -0.749_f32, 2.53125_f32],
    [0.749_f32, -1.3375_f32, 2.53125_f32],
    [0_f32, -1.3375_f32, 2.53125_f32],
    [1.4375_f32, 0_f32, 2.53125_f32],
    [1.4375_f32, -0.805_f32, 2.53125_f32],
    [0.805_f32, -1.4375_f32, 2.53125_f32],
    [0_f32, -1.4375_f32, 2.53125_f32],
    [1.5_f32, 0_f32, 2.4_f32],
    [1.5_f32, -0.84_f32, 2.4_f32],
    [0.84_f32, -1.5_f32, 2.4_f32],
    [0_f32, -1.5_f32, 2.4_f32],
    [1.75_f32, 0_f32, 1.875_f32],
    [1.75_f32, -0.98_f32, 1.875_f32],
    [0.98_f32, -1.75_f32, 1.875_f32],
    [0_f32, -1.75_f32, 1.875_f32],
    [2_f32, 0_f32, 1.35_f32],
    [2_f32, -1.12_f32, 1.35_f32],
    [1.12_f32, -2_f32, 1.35_f32],
    [0_f32, -2_f32, 1.35_f32],
    [2_f32, 0_f32, 0.9_f32],
    [2_f32, -1.12_f32, 0.9_f32],
    [1.12_f32, -2_f32, 0.9_f32],
    [0_f32, -2_f32, 0.9_f32],
    [-2_f32, 0_f32, 0.9_f32],
    [2_f32, 0_f32, 0.45_f32],
    [2_f32, -1.12_f32, 0.45_f32],
    [1.12_f32, -2_f32, 0.45_f32],
    [0_f32, -2_f32, 0.45_f32],
    [1.5_f32, 0_f32, 0.225_f32],
    [1.5_f32, -0.84_f32, 0.225_f32],
    [0.84_f32, -1.5_f32, 0.225_f32],
    [0_f32, -1.5_f32, 0.225_f32],
    [1.5_f32, 0_f32, 0.15_f32],
    [1.5_f32, -0.84_f32, 0.15_f32],
    [0.84_f32, -1.5_f32, 0.15_f32],
    [0_f32, -1.5_f32, 0.15_f32],
    [-1.6_f32, 0_f32, 2.025_f32],
    [-1.6_f32, -0.3_f32, 2.025_f32],
    [-1.5_f32, -0.3_f32, 2.25_f32],
    [-1.5_f32, 0_f32, 2.25_f32],
    [-2.3_f32, 0_f32, 2.025_f32],
    [-2.3_f32, -0.3_f32, 2.025_f32],
    [-2.5_f32, -0.3_f32, 2.25_f32],
    [-2.5_f32, 0_f32, 2.25_f32],
    [-2.7_f32, 0_f32, 2.025_f32],
    [-2.7_f32, -0.3_f32, 2.025_f32],
    [-3_f32, -0.3_f32, 2.25_f32],
    [-3_f32, 0_f32, 2.25_f32],
    [-2.7_f32, 0_f32, 1.8_f32],
    [-2.7_f32, -0.3_f32, 1.8_f32],
    [-3_f32, -0.3_f32, 1.8_f32],
    [-3_f32, 0_f32, 1.8_f32],
    [-2.7_f32, 0_f32, 1.575_f32],
    [-2.7_f32, -0.3_f32, 1.575_f32],
    [-3_f32, -0.3_f32, 1.35_f32],
    [-3_f32, 0_f32, 1.35_f32],
    [-2.5_f32, 0_f32, 1.125_f32],
    [-2.5_f32, -0.3_f32, 1.125_f32],
    [-2.65_f32, -0.3_f32, 0.9375_f32],
    [-2.65_f32, 0_f32, 0.9375_f32],
    [-2_f32, -0.3_f32, 0.9_f32],
    [-1.9_f32, -0.3_f32, 0.6_f32],
    [-1.9_f32, 0_f32, 0.6_f32],
    [1.7_f32, 0_f32, 1.425_f32],
    [1.7_f32, -0.66_f32, 1.425_f32],
    [1.7_f32, -0.66_f32, 0.6_f32],
    [1.7_f32, 0_f32, 0.6_f32],
    [2.6_f32, 0_f32, 1.425_f32],
    [2.6_f32, -0.66_f32, 1.425_f32],
    [3.1_f32, -0.66_f32, 0.825_f32],
    [3.1_f32, 0_f32, 0.825_f32],
    [2.3_f32, 0_f32, 2.1_f32],
    [2.3_f32, -0.25_f32, 2.1_f32],
    [2.4_f32, -0.25_f32, 2.025_f32],
    [2.4_f32, 0_f32, 2.025_f32],
    [2.7_f32, 0_f32, 2.4_f32],
    [2.7_f32, -0.25_f32, 2.4_f32],
    [3.3_f32, -0.25_f32, 2.4_f32],
    [3.3_f32, 0_f32, 2.4_f32],
    [2.8_f32, 0_f32, 2.475_f32],
    [2.8_f32, -0.25_f32, 2.475_f32],
    [3.525_f32, -0.25_f32, 2.49375_f32],
    [3.525_f32, 0_f32, 2.49375_f32],
    [2.9_f32, 0_f32, 2.475_f32],
    [2.9_f32, -0.15_f32, 2.475_f32],
    [3.45_f32, -0.15_f32, 2.5125_f32],
    [3.45_f32, 0_f32, 2.5125_f32],
    [2.8_f32, 0_f32, 2.4_f32],
    [2.8_f32, -0.15_f32, 2.4_f32],
    [3.2_f32, -0.15_f32, 2.4_f32],
    [3.2_f32, 0_f32, 2.4_f32],
    [0_f32, 0_f32, 3.15_f32],
    [0.8_f32, 0_f32, 3.15_f32],
    [0.8_f32, -0.45_f32, 3.15_f32],
    [0.45_f32, -0.8_f32, 3.15_f32],
    [0_f32, -0.8_f32, 3.15_f32],
    [0_f32, 0_f32, 2.85_f32],
    [1.4_f32, 0_f32, 2.4_f32],
    [1.4_f32, -0.784_f32, 2.4_f32],
    [0.784_f32, -1.4_f32, 2.4_f32],
    [0_f32, -1.4_f32, 2.4_f32],
    [0.4_f32, 0_f32, 2.55_f32],
    [0.4_f32, -0.224_f32, 2.55_f32],
    [0.224_f32, -0.4_f32, 2.55_f32],
    [0_f32, -0.4_f32, 2.55_f32],
    [1.3_f32, 0_f32, 2.55_f32],
    [1.3_f32, -0.728_f32, 2.55_f32],
    [0.728_f32, -1.3_f32, 2.55_f32],
    [0_f32, -1.3_f32, 2.55_f32],
    [1.3_f32, 0_f32, 2.4_f32],
    [1.3_f32, -0.728_f32, 2.4_f32],
    [0.728_f32, -1.3_f32, 2.4_f32],
    [0_f32, -1.3_f32, 2.4_f32],
    [0_f32, 0_f32, 0_f32],
    [1.425_f32, -0.798_f32, 0_f32],
    [1.5_f32, 0_f32, 0.075_f32],
    [1.425_f32, 0_f32, 0_f32],
    [0.798_f32, -1.425_f32, 0_f32],
    [0_f32, -1.5_f32, 0.075_f32],
    [0_f32, -1.425_f32, 0_f32],
    [1.5_f32, -0.84_f32, 0.075_f32],
    [0.84_f32, -1.5_f32, 0.075_f32],
];
