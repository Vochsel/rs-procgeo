use glam::Vec3;
use serde::{Deserialize, Serialize};

use procgeo_core::attribute::{AttribClass, AttribDefault, TypeQualifier};
use procgeo_core::Geometry;

use crate::{Sop, SopError};

/// Kernel function controlling the density falloff from center to edge.
#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub enum MetaballKernel {
    /// Blinn: exp(-a * r^2). Stable, always puts a sphere at center.
    Blinn,
    /// Wyvill: (1 - r^2)^3. Smooth, C2 continuous, compact support.
    #[default]
    Wyvill,
    /// Hart: 1 - 6t^5 + 15t^4 - 10t^3 (smooth-step polynomial).
    Hart,
}

/// Definition of a single metaball in the field.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetaballDef {
    pub center: Vec3,
    pub radius: f32,
    pub weight: f32,
}

impl Default for MetaballDef {
    fn default() -> Self {
        MetaballDef {
            center: Vec3::ZERO,
            radius: 1.0,
            weight: 1.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetaballParams {
    /// Metaball definitions. If empty, creates a single default metaball.
    pub balls: Vec<MetaballDef>,
    /// Density threshold for surface extraction.
    pub threshold: f32,
    /// Kernel function for density falloff.
    pub kernel: MetaballKernel,
    /// Resolution of the marching cubes grid (cells per axis in the largest dimension).
    pub resolution: u32,
    /// Padding around bounding box as a fraction of its size.
    pub padding: f32,
}

impl Default for MetaballParams {
    fn default() -> Self {
        MetaballParams {
            balls: vec![MetaballDef::default()],
            threshold: 0.5,
            kernel: MetaballKernel::default(),
            resolution: 32,
            padding: 0.2,
        }
    }
}

pub struct MetaballSop;

/// Evaluate kernel density at squared distance `r2` with given radius.
/// Returns density contribution in [0, 1] range within the radius, 0 outside.
fn eval_kernel(kernel: &MetaballKernel, r2: f32, radius: f32) -> f32 {
    let r2_norm = r2 / (radius * radius);
    if r2_norm >= 1.0 {
        return 0.0;
    }
    match kernel {
        MetaballKernel::Blinn => {
            // Gaussian-like: exp(-4 * r2_norm) — clamped at radius
            (-4.0 * r2_norm).exp()
        }
        MetaballKernel::Wyvill => {
            // (1 - r2_norm)^3
            let t = 1.0 - r2_norm;
            t * t * t
        }
        MetaballKernel::Hart => {
            // 1 - 6t^5 + 15t^4 - 10t^3 where t = r2_norm
            let t = r2_norm;
            let t3 = t * t * t;
            let t4 = t3 * t;
            let t5 = t4 * t;
            1.0 - 6.0 * t5 + 15.0 * t4 - 10.0 * t3
        }
    }
}

/// Evaluate the total density field at a point.
fn eval_field(pos: Vec3, balls: &[MetaballDef], kernel: &MetaballKernel) -> f32 {
    let mut density = 0.0f32;
    for ball in balls {
        let r2 = (pos - ball.center).length_squared();
        density += ball.weight * eval_kernel(kernel, r2, ball.radius);
    }
    density
}

/// Marching cubes edge table: for each of the 256 cube configurations, which edges are intersected.
/// Edge indices: 0-11 mapping to specific cube edges.
/// Using a compact representation: each entry is a 12-bit mask.
///
/// Rather than embedding the full 256-entry table, we implement a simplified
/// marching cubes that processes each cube by checking all 12 edges and forming
/// triangles from the intersection vertices.

impl Sop for MetaballSop {
    type Params = MetaballParams;

    fn name(&self) -> &'static str {
        "metaball"
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

        let balls = if params.balls.is_empty() {
            &[MetaballDef::default()][..]
        } else {
            &params.balls
        };

        // Compute bounding box of all metaballs
        let mut bb_min = Vec3::splat(f32::MAX);
        let mut bb_max = Vec3::splat(f32::MIN);
        for ball in balls {
            let r = Vec3::splat(ball.radius);
            bb_min = bb_min.min(ball.center - r);
            bb_max = bb_max.max(ball.center + r);
        }

        // Add padding
        let size = bb_max - bb_min;
        let pad = size * params.padding;
        bb_min -= pad;
        bb_max += pad;

        let grid_size = bb_max - bb_min;
        let max_dim = grid_size.x.max(grid_size.y).max(grid_size.z);
        let cell_size = max_dim / params.resolution as f32;

        let nx = ((grid_size.x / cell_size).ceil() as usize).max(2);
        let ny = ((grid_size.y / cell_size).ceil() as usize).max(2);
        let nz = ((grid_size.z / cell_size).ceil() as usize).max(2);

        // Sample the density field at grid vertices
        let gx = nx + 1;
        let gy = ny + 1;
        let gz = nz + 1;
        let mut field = vec![0.0f32; gx * gy * gz];
        for iz in 0..gz {
            for iy in 0..gy {
                for ix in 0..gx {
                    let pos = bb_min
                        + Vec3::new(
                            ix as f32 * cell_size,
                            iy as f32 * cell_size,
                            iz as f32 * cell_size,
                        );
                    field[iz * gy * gx + iy * gx + ix] = eval_field(pos, balls, &params.kernel);
                }
            }
        }

        let threshold = params.threshold;

        // Marching cubes: process each cell
        let mut geo = Geometry::new();

        // Cache for edge-interpolated vertices to avoid duplicates.
        // Key: (edge_type, lower_grid_index) → PointHandle index
        // Edge types: 0=X-edge, 1=Y-edge, 2=Z-edge
        use std::collections::HashMap;
        let mut edge_vertices: HashMap<(u8, usize, usize, usize), procgeo_core::PointHandle> =
            HashMap::new();

        let idx = |ix: usize, iy: usize, iz: usize| -> usize { iz * gy * gx + iy * gx + ix };

        let interp = |v0: Vec3, d0: f32, v1: Vec3, d1: f32| -> Vec3 {
            if (d1 - d0).abs() < 1e-10 {
                return (v0 + v1) * 0.5;
            }
            let t = (threshold - d0) / (d1 - d0);
            v0 + t * (v1 - v0)
        };

        let pos_at = |ix: usize, iy: usize, iz: usize| -> Vec3 {
            bb_min
                + Vec3::new(
                    ix as f32 * cell_size,
                    iy as f32 * cell_size,
                    iz as f32 * cell_size,
                )
        };

        // For each cell, identify sign changes on the 12 edges and generate triangles
        // using the classic marching cubes algorithm with lookup tables.
        //
        // Cube vertex numbering:
        //   4-----5
        //  /|    /|
        // 7-----6 |
        // | 0---|-1
        // |/    |/
        // 3-----2
        //
        // v0=(0,0,0) v1=(1,0,0) v2=(1,0,1) v3=(0,0,1)
        // v4=(0,1,0) v5=(1,1,0) v6=(1,1,1) v7=(0,1,1)

        // Edge table and triangle table (compressed marching cubes)
        let edge_table: [u16; 256] = EDGE_TABLE;
        let tri_table: [[i8; 16]; 256] = TRI_TABLE;

        // Cube corner offsets: [dx, dy, dz]
        let corner_offsets: [(usize, usize, usize); 8] = [
            (0, 0, 0),
            (1, 0, 0),
            (1, 0, 1),
            (0, 0, 1),
            (0, 1, 0),
            (1, 1, 0),
            (1, 1, 1),
            (0, 1, 1),
        ];

        // Edge definitions: (corner_a, corner_b, edge_type, grid_offset_x, grid_offset_y, grid_offset_z)
        // edge_type: 0=X, 1=Y, 2=Z
        let edge_defs: [(usize, usize, u8, usize, usize, usize); 12] = [
            (0, 1, 0, 0, 0, 0), // edge 0: v0-v1, X-edge at (ix, iy, iz)
            (1, 2, 2, 1, 0, 0), // edge 1: v1-v2, Z-edge at (ix+1, iy, iz)
            (3, 2, 0, 0, 0, 1), // edge 2: v3-v2, X-edge at (ix, iy, iz+1)
            (0, 3, 2, 0, 0, 0), // edge 3: v0-v3, Z-edge at (ix, iy, iz)
            (4, 5, 0, 0, 1, 0), // edge 4: v4-v5, X-edge at (ix, iy+1, iz)
            (5, 6, 2, 1, 1, 0), // edge 5: v5-v6, Z-edge at (ix+1, iy+1, iz)
            (7, 6, 0, 0, 1, 1), // edge 6: v7-v6, X-edge at (ix, iy+1, iz+1)
            (4, 7, 2, 0, 1, 0), // edge 7: v4-v7, Z-edge at (ix, iy+1, iz)
            (0, 4, 1, 0, 0, 0), // edge 8: v0-v4, Y-edge at (ix, iy, iz)
            (1, 5, 1, 1, 0, 0), // edge 9: v1-v5, Y-edge at (ix+1, iy, iz)
            (2, 6, 1, 1, 0, 1), // edge 10: v2-v6, Y-edge at (ix+1, iy, iz+1)
            (3, 7, 1, 0, 0, 1), // edge 11: v3-v7, Y-edge at (ix, iy, iz+1)
        ];

        for iz in 0..nz {
            for iy in 0..ny {
                for ix in 0..nx {
                    // Determine cube index from corner signs
                    let mut cube_idx = 0u8;
                    let mut corners = [(0usize, 0usize, 0usize, 0.0f32); 8];
                    for (i, &(dx, dy, dz)) in corner_offsets.iter().enumerate() {
                        let cx = ix + dx;
                        let cy = iy + dy;
                        let cz = iz + dz;
                        let d = field[idx(cx, cy, cz)];
                        corners[i] = (cx, cy, cz, d);
                        if d >= threshold {
                            cube_idx |= 1 << i;
                        }
                    }

                    if edge_table[cube_idx as usize] == 0 {
                        continue;
                    }

                    // For each edge that has an intersection, compute or reuse the vertex
                    let mut edge_verts = [None; 12];
                    let edges_mask = edge_table[cube_idx as usize];

                    for edge_idx in 0..12u8 {
                        if edges_mask & (1 << edge_idx) == 0 {
                            continue;
                        }

                        let (ca, cb, etype, edx, edy, edz) = edge_defs[edge_idx as usize];
                        let key = (etype, ix + edx, iy + edy, iz + edz);

                        let handle = *edge_vertices.entry(key).or_insert_with(|| {
                            let (cax, cay, caz, da) = corners[ca];
                            let (cbx, cby, cbz, db) = corners[cb];
                            let pa = pos_at(cax, cay, caz);
                            let pb = pos_at(cbx, cby, cbz);
                            let p = interp(pa, da, pb, db);
                            geo.add_point(p)
                        });
                        edge_verts[edge_idx as usize] = Some(handle);
                    }

                    // Generate triangles from the triangle table
                    let tri_row = &tri_table[cube_idx as usize];
                    let mut i = 0;
                    while i < 16 && tri_row[i] >= 0 {
                        let v0 = edge_verts[tri_row[i] as usize].unwrap();
                        let v1 = edge_verts[tri_row[i + 1] as usize].unwrap();
                        let v2 = edge_verts[tri_row[i + 2] as usize].unwrap();
                        geo.add_face(&[v0, v1, v2]);
                        i += 3;
                    }
                }
            }
        }

        // Add "density" point attribute with field value at each vertex position
        if geo.num_points() > 0 {
            geo.add_attrib(
                AttribClass::Point,
                "density",
                AttribDefault::Float(0.0),
                TypeQualifier::None,
            )
            .map_err(SopError::Core)?;
            let handle = geo
                .find_attrib::<f32>(AttribClass::Point, "density")
                .map_err(SopError::Core)?;
            for i in 0..geo.num_points() {
                let pos = geo.point_pos(procgeo_core::handle::PointHandle::from_index(i));
                let d = eval_field(pos, balls, &params.kernel);
                geo.set_attrib(&handle, i, d).map_err(SopError::Core)?;
            }
        }

        Ok(geo)
    }
}

// ===========================================================================
// Marching Cubes Lookup Tables
// ===========================================================================

/// Edge table: for each of the 256 cube configurations, a 12-bit mask
/// indicating which edges contain an intersection vertex.
#[rustfmt::skip]
const EDGE_TABLE: [u16; 256] = [
    0x000, 0x109, 0x203, 0x30a, 0x406, 0x50f, 0x605, 0x70c,
    0x80c, 0x905, 0xa0f, 0xb06, 0xc0a, 0xd03, 0xe09, 0xf00,
    0x190, 0x099, 0x393, 0x29a, 0x596, 0x49f, 0x795, 0x69c,
    0x99c, 0x895, 0xb9f, 0xa96, 0xd9a, 0xc93, 0xf99, 0xe90,
    0x230, 0x339, 0x033, 0x13a, 0x636, 0x73f, 0x435, 0x53c,
    0xa3c, 0xb35, 0x83f, 0x936, 0xe3a, 0xf33, 0xc39, 0xd30,
    0x3a0, 0x2a9, 0x1a3, 0x0aa, 0x7a6, 0x6af, 0x5a5, 0x4ac,
    0xbac, 0xaa5, 0x9af, 0x8a6, 0xfaa, 0xea3, 0xda9, 0xca0,
    0x460, 0x569, 0x663, 0x76a, 0x066, 0x16f, 0x265, 0x36c,
    0xc6c, 0xd65, 0xe6f, 0xf66, 0x86a, 0x963, 0xa69, 0xb60,
    0x5f0, 0x4f9, 0x7f3, 0x6fa, 0x1f6, 0x0ff, 0x3f5, 0x2fc,
    0xdfc, 0xcf5, 0xfff, 0xef6, 0x9fa, 0x8f3, 0xbf9, 0xaf0,
    0x650, 0x759, 0x453, 0x55a, 0x256, 0x35f, 0x055, 0x15c,
    0xe5c, 0xf55, 0xc5f, 0xd56, 0xa5a, 0xb53, 0x859, 0x950,
    0x7c0, 0x6c9, 0x5c3, 0x4ca, 0x3c6, 0x2cf, 0x1c5, 0x0cc,
    0xfcc, 0xec5, 0xdcf, 0xcc6, 0xbca, 0xac3, 0x9c9, 0x8c0,
    0x8c0, 0x9c9, 0xac3, 0xbca, 0xcc6, 0xdcf, 0xec5, 0xfcc,
    0x0cc, 0x1c5, 0x2cf, 0x3c6, 0x4ca, 0x5c3, 0x6c9, 0x7c0,
    0x950, 0x859, 0xb53, 0xa5a, 0xd56, 0xc5f, 0xf55, 0xe5c,
    0x15c, 0x055, 0x35f, 0x256, 0x55a, 0x453, 0x759, 0x650,
    0xaf0, 0xbf9, 0x8f3, 0x9fa, 0xef6, 0xfff, 0xcf5, 0xdfc,
    0x2fc, 0x3f5, 0x0ff, 0x1f6, 0x6fa, 0x7f3, 0x4f9, 0x5f0,
    0xb60, 0xa69, 0x963, 0x86a, 0xf66, 0xe6f, 0xd65, 0xc6c,
    0x36c, 0x265, 0x16f, 0x066, 0x76a, 0x663, 0x569, 0x460,
    0xca0, 0xda9, 0xea3, 0xfaa, 0x8a6, 0x9af, 0xaa5, 0xbac,
    0x4ac, 0x5a5, 0x6af, 0x7a6, 0x0aa, 0x1a3, 0x2a9, 0x3a0,
    0xd30, 0xc39, 0xf33, 0xe3a, 0x936, 0x83f, 0xb35, 0xa3c,
    0x53c, 0x435, 0x73f, 0x636, 0x13a, 0x033, 0x339, 0x230,
    0xe90, 0xf99, 0xc93, 0xd9a, 0xa96, 0xb9f, 0x895, 0x99c,
    0x69c, 0x795, 0x49f, 0x596, 0x29a, 0x393, 0x099, 0x190,
    0xf00, 0xe09, 0xd03, 0xc0a, 0xb06, 0xa0f, 0x905, 0x80c,
    0x70c, 0x605, 0x50f, 0x406, 0x30a, 0x203, 0x109, 0x000,
];

/// Triangle table: for each of the 256 cube configurations, up to 5 triangles
/// specified as edge indices. -1 terminates the list.
#[rustfmt::skip]
const TRI_TABLE: [[i8; 16]; 256] = [
    [-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 0, 8, 3,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 0, 1, 9,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 1, 8, 3, 9, 8, 1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 1, 2,10,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 0, 8, 3, 1, 2,10,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 9, 2,10, 0, 2, 9,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 2, 8, 3, 2,10, 8,10, 9, 8,-1,-1,-1,-1,-1,-1,-1],
    [ 3,11, 2,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 0,11, 2, 8,11, 0,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 1, 9, 0, 2, 3,11,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 1,11, 2, 1, 9,11, 9, 8,11,-1,-1,-1,-1,-1,-1,-1],
    [ 3,10, 1,11,10, 3,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 0,10, 1, 0, 8,10, 8,11,10,-1,-1,-1,-1,-1,-1,-1],
    [ 3, 9, 0, 3,11, 9,11,10, 9,-1,-1,-1,-1,-1,-1,-1],
    [ 9, 8,10,10, 8,11,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 4, 7, 8,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 4, 3, 0, 7, 3, 4,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 0, 1, 9, 8, 4, 7,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 4, 1, 9, 4, 7, 1, 7, 3, 1,-1,-1,-1,-1,-1,-1,-1],
    [ 1, 2,10, 8, 4, 7,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 3, 4, 7, 3, 0, 4, 1, 2,10,-1,-1,-1,-1,-1,-1,-1],
    [ 9, 2,10, 9, 0, 2, 8, 4, 7,-1,-1,-1,-1,-1,-1,-1],
    [ 2,10, 9, 2, 9, 7, 2, 7, 3, 7, 9, 4,-1,-1,-1,-1],
    [ 8, 4, 7, 3,11, 2,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [11, 4, 7,11, 2, 4, 2, 0, 4,-1,-1,-1,-1,-1,-1,-1],
    [ 9, 0, 1, 8, 4, 7, 2, 3,11,-1,-1,-1,-1,-1,-1,-1],
    [ 4, 7,11, 9, 4,11, 9,11, 2, 9, 2, 1,-1,-1,-1,-1],
    [ 3,10, 1, 3,11,10, 7, 8, 4,-1,-1,-1,-1,-1,-1,-1],
    [ 1,11,10, 1, 4,11, 1, 0, 4, 7,11, 4,-1,-1,-1,-1],
    [ 4, 7, 8, 9, 0,11, 9,11,10,11, 0, 3,-1,-1,-1,-1],
    [ 4, 7,11, 4,11, 9, 9,11,10,-1,-1,-1,-1,-1,-1,-1],
    [ 9, 5, 4,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 9, 5, 4, 0, 8, 3,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 0, 5, 4, 1, 5, 0,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 8, 5, 4, 8, 3, 5, 3, 1, 5,-1,-1,-1,-1,-1,-1,-1],
    [ 1, 2,10, 9, 5, 4,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 3, 0, 8, 1, 2,10, 4, 9, 5,-1,-1,-1,-1,-1,-1,-1],
    [ 5, 2,10, 5, 4, 2, 4, 0, 2,-1,-1,-1,-1,-1,-1,-1],
    [ 2,10, 5, 3, 2, 5, 3, 5, 4, 3, 4, 8,-1,-1,-1,-1],
    [ 9, 5, 4, 2, 3,11,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 0,11, 2, 0, 8,11, 4, 9, 5,-1,-1,-1,-1,-1,-1,-1],
    [ 0, 5, 4, 0, 1, 5, 2, 3,11,-1,-1,-1,-1,-1,-1,-1],
    [ 2, 1, 5, 2, 5, 8, 2, 8,11, 4, 8, 5,-1,-1,-1,-1],
    [10, 3,11,10, 1, 3, 9, 5, 4,-1,-1,-1,-1,-1,-1,-1],
    [ 4, 9, 5, 0, 8, 1, 8,10, 1, 8,11,10,-1,-1,-1,-1],
    [ 5, 4, 0, 5, 0,11, 5,11,10,11, 0, 3,-1,-1,-1,-1],
    [ 5, 4, 8, 5, 8,10,10, 8,11,-1,-1,-1,-1,-1,-1,-1],
    [ 9, 7, 8, 5, 7, 9,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 9, 3, 0, 9, 5, 3, 5, 7, 3,-1,-1,-1,-1,-1,-1,-1],
    [ 0, 7, 8, 0, 1, 7, 1, 5, 7,-1,-1,-1,-1,-1,-1,-1],
    [ 1, 5, 3, 3, 5, 7,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 9, 7, 8, 9, 5, 7,10, 1, 2,-1,-1,-1,-1,-1,-1,-1],
    [10, 1, 2, 9, 5, 0, 5, 3, 0, 5, 7, 3,-1,-1,-1,-1],
    [ 8, 0, 2, 8, 2, 5, 8, 5, 7,10, 5, 2,-1,-1,-1,-1],
    [ 2,10, 5, 2, 5, 3, 3, 5, 7,-1,-1,-1,-1,-1,-1,-1],
    [ 7, 9, 5, 7, 8, 9, 3,11, 2,-1,-1,-1,-1,-1,-1,-1],
    [ 9, 5, 7, 9, 7, 2, 9, 2, 0, 2, 7,11,-1,-1,-1,-1],
    [ 2, 3,11, 0, 1, 8, 1, 7, 8, 1, 5, 7,-1,-1,-1,-1],
    [11, 2, 1,11, 1, 7, 7, 1, 5,-1,-1,-1,-1,-1,-1,-1],
    [ 9, 5, 8, 8, 5, 7,10, 1, 3,10, 3,11,-1,-1,-1,-1],
    [ 5, 7, 0, 5, 0, 9, 7,11, 0, 1, 0,10,11,10, 0,-1],
    [11,10, 0,11, 0, 3,10, 5, 0, 8, 0, 7, 5, 7, 0,-1],
    [11,10, 5, 7,11, 5,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [10, 6, 5,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 0, 8, 3, 5,10, 6,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 9, 0, 1, 5,10, 6,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 1, 8, 3, 1, 9, 8, 5,10, 6,-1,-1,-1,-1,-1,-1,-1],
    [ 1, 6, 5, 2, 6, 1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 1, 6, 5, 1, 2, 6, 3, 0, 8,-1,-1,-1,-1,-1,-1,-1],
    [ 9, 6, 5, 9, 0, 6, 0, 2, 6,-1,-1,-1,-1,-1,-1,-1],
    [ 5, 9, 8, 5, 8, 2, 5, 2, 6, 3, 2, 8,-1,-1,-1,-1],
    [ 2, 3,11,10, 6, 5,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [11, 0, 8,11, 2, 0,10, 6, 5,-1,-1,-1,-1,-1,-1,-1],
    [ 0, 1, 9, 2, 3,11, 5,10, 6,-1,-1,-1,-1,-1,-1,-1],
    [ 5,10, 6, 1, 9, 2, 9,11, 2, 9, 8,11,-1,-1,-1,-1],
    [ 6, 3,11, 6, 5, 3, 5, 1, 3,-1,-1,-1,-1,-1,-1,-1],
    [ 0, 8,11, 0,11, 5, 0, 5, 1, 5,11, 6,-1,-1,-1,-1],
    [ 3,11, 6, 0, 3, 6, 0, 6, 5, 0, 5, 9,-1,-1,-1,-1],
    [ 6, 5, 9, 6, 9,11,11, 9, 8,-1,-1,-1,-1,-1,-1,-1],
    [ 5,10, 6, 4, 7, 8,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 4, 3, 0, 4, 7, 3, 6, 5,10,-1,-1,-1,-1,-1,-1,-1],
    [ 1, 9, 0, 5,10, 6, 8, 4, 7,-1,-1,-1,-1,-1,-1,-1],
    [10, 6, 5, 1, 9, 7, 1, 7, 3, 7, 9, 4,-1,-1,-1,-1],
    [ 6, 1, 2, 6, 5, 1, 4, 7, 8,-1,-1,-1,-1,-1,-1,-1],
    [ 1, 2, 5, 5, 2, 6, 3, 0, 4, 3, 4, 7,-1,-1,-1,-1],
    [ 8, 4, 7, 9, 0, 5, 0, 6, 5, 0, 2, 6,-1,-1,-1,-1],
    [ 7, 3, 9, 7, 9, 4, 3, 2, 9, 5, 9, 6, 2, 6, 9,-1],
    [ 3,11, 2, 7, 8, 4,10, 6, 5,-1,-1,-1,-1,-1,-1,-1],
    [ 5,10, 6, 4, 7, 2, 4, 2, 0, 2, 7,11,-1,-1,-1,-1],
    [ 0, 1, 9, 4, 7, 8, 2, 3,11, 5,10, 6,-1,-1,-1,-1],
    [ 9, 2, 1, 9,11, 2, 9, 4,11, 7,11, 4, 5,10, 6,-1],
    [ 8, 4, 7, 3,11, 5, 3, 5, 1, 5,11, 6,-1,-1,-1,-1],
    [ 5, 1,11, 5,11, 6, 1, 0,11, 7,11, 4, 0, 4,11,-1],
    [ 0, 5, 9, 0, 6, 5, 0, 3, 6,11, 6, 3, 8, 4, 7,-1],
    [ 6, 5, 9, 6, 9,11, 4, 7, 9, 7,11, 9,-1,-1,-1,-1],
    [10, 4, 9, 6, 4,10,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 4,10, 6, 4, 9,10, 0, 8, 3,-1,-1,-1,-1,-1,-1,-1],
    [10, 0, 1,10, 6, 0, 6, 4, 0,-1,-1,-1,-1,-1,-1,-1],
    [ 8, 3, 1, 8, 1, 6, 8, 6, 4, 6, 1,10,-1,-1,-1,-1],
    [ 1, 4, 9, 1, 2, 4, 2, 6, 4,-1,-1,-1,-1,-1,-1,-1],
    [ 3, 0, 8, 1, 2, 9, 2, 4, 9, 2, 6, 4,-1,-1,-1,-1],
    [ 0, 2, 4, 4, 2, 6,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 8, 3, 2, 8, 2, 4, 4, 2, 6,-1,-1,-1,-1,-1,-1,-1],
    [10, 4, 9,10, 6, 4,11, 2, 3,-1,-1,-1,-1,-1,-1,-1],
    [ 0, 8, 2, 2, 8,11, 4, 9,10, 4,10, 6,-1,-1,-1,-1],
    [ 3,11, 2, 0, 1, 6, 0, 6, 4, 6, 1,10,-1,-1,-1,-1],
    [ 6, 4, 1, 6, 1,10, 4, 8, 1, 2, 1,11, 8,11, 1,-1],
    [ 9, 6, 4, 9, 3, 6, 9, 1, 3,11, 6, 3,-1,-1,-1,-1],
    [ 8,11, 1, 8, 1, 0,11, 6, 1, 9, 1, 4, 6, 4, 1,-1],
    [ 3,11, 6, 3, 6, 0, 0, 6, 4,-1,-1,-1,-1,-1,-1,-1],
    [ 6, 4, 8,11, 6, 8,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 7,10, 6, 7, 8,10, 8, 9,10,-1,-1,-1,-1,-1,-1,-1],
    [ 0, 7, 3, 0,10, 7, 0, 9,10, 6, 7,10,-1,-1,-1,-1],
    [10, 6, 7, 1,10, 7, 1, 7, 8, 1, 8, 0,-1,-1,-1,-1],
    [10, 6, 7,10, 7, 1, 1, 7, 3,-1,-1,-1,-1,-1,-1,-1],
    [ 1, 2, 6, 1, 6, 8, 1, 8, 9, 8, 6, 7,-1,-1,-1,-1],
    [ 2, 6, 9, 2, 9, 1, 6, 7, 9, 0, 9, 3, 7, 3, 9,-1],
    [ 7, 8, 0, 7, 0, 6, 6, 0, 2,-1,-1,-1,-1,-1,-1,-1],
    [ 7, 3, 2, 6, 7, 2,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 2, 3,11,10, 6, 8,10, 8, 9, 8, 6, 7,-1,-1,-1,-1],
    [ 2, 0, 7, 2, 7,11, 0, 9, 7, 6, 7,10, 9,10, 7,-1],
    [ 1, 8, 0, 1, 7, 8, 1,10, 7, 6, 7,10, 2, 3,11,-1],
    [11, 2, 1,11, 1, 7,10, 6, 1, 6, 7, 1,-1,-1,-1,-1],
    [ 8, 9, 6, 8, 6, 7, 9, 1, 6,11, 6, 3, 1, 3, 6,-1],
    [ 0, 9, 1,11, 6, 7,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 7, 8, 0, 7, 0, 6, 3,11, 0,11, 6, 0,-1,-1,-1,-1],
    [ 7,11, 6,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 7, 6,11,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 3, 0, 8,11, 7, 6,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 0, 1, 9,11, 7, 6,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 8, 1, 9, 8, 3, 1,11, 7, 6,-1,-1,-1,-1,-1,-1,-1],
    [10, 1, 2, 6,11, 7,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 1, 2,10, 3, 0, 8, 6,11, 7,-1,-1,-1,-1,-1,-1,-1],
    [ 2, 9, 0, 2,10, 9, 6,11, 7,-1,-1,-1,-1,-1,-1,-1],
    [ 6,11, 7, 2,10, 3,10, 8, 3,10, 9, 8,-1,-1,-1,-1],
    [ 7, 2, 3, 6, 2, 7,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 7, 0, 8, 7, 6, 0, 6, 2, 0,-1,-1,-1,-1,-1,-1,-1],
    [ 2, 7, 6, 2, 3, 7, 0, 1, 9,-1,-1,-1,-1,-1,-1,-1],
    [ 1, 6, 2, 1, 8, 6, 1, 9, 8, 8, 7, 6,-1,-1,-1,-1],
    [10, 7, 6,10, 1, 7, 1, 3, 7,-1,-1,-1,-1,-1,-1,-1],
    [10, 7, 6, 1, 7,10, 1, 8, 7, 1, 0, 8,-1,-1,-1,-1],
    [ 0, 3, 7, 0, 7,10, 0,10, 9, 6,10, 7,-1,-1,-1,-1],
    [ 7, 6,10, 7,10, 8, 8,10, 9,-1,-1,-1,-1,-1,-1,-1],
    [ 6, 8, 4,11, 8, 6,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 3, 6,11, 3, 0, 6, 0, 4, 6,-1,-1,-1,-1,-1,-1,-1],
    [ 8, 6,11, 8, 4, 6, 9, 0, 1,-1,-1,-1,-1,-1,-1,-1],
    [ 9, 4, 6, 9, 6, 3, 9, 3, 1,11, 3, 6,-1,-1,-1,-1],
    [ 6, 8, 4, 6,11, 8, 2,10, 1,-1,-1,-1,-1,-1,-1,-1],
    [ 1, 2,10, 3, 0,11, 0, 6,11, 0, 4, 6,-1,-1,-1,-1],
    [ 4,11, 8, 4, 6,11, 0, 2, 9, 2,10, 9,-1,-1,-1,-1],
    [10, 9, 3,10, 3, 2, 9, 4, 3,11, 3, 6, 4, 6, 3,-1],
    [ 8, 2, 3, 8, 4, 2, 4, 6, 2,-1,-1,-1,-1,-1,-1,-1],
    [ 0, 4, 2, 4, 6, 2,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 1, 9, 0, 2, 3, 4, 2, 4, 6, 4, 3, 8,-1,-1,-1,-1],
    [ 1, 9, 4, 1, 4, 2, 2, 4, 6,-1,-1,-1,-1,-1,-1,-1],
    [ 8, 1, 3, 8, 6, 1, 8, 4, 6, 6,10, 1,-1,-1,-1,-1],
    [10, 1, 0,10, 0, 6, 6, 0, 4,-1,-1,-1,-1,-1,-1,-1],
    [ 4, 6, 3, 4, 3, 8, 6,10, 3, 0, 3, 9,10, 9, 3,-1],
    [10, 9, 4, 6,10, 4,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 4, 9, 5, 7, 6,11,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 0, 8, 3, 4, 9, 5,11, 7, 6,-1,-1,-1,-1,-1,-1,-1],
    [ 5, 0, 1, 5, 4, 0, 7, 6,11,-1,-1,-1,-1,-1,-1,-1],
    [11, 7, 6, 8, 3, 4, 3, 5, 4, 3, 1, 5,-1,-1,-1,-1],
    [ 9, 5, 4,10, 1, 2, 7, 6,11,-1,-1,-1,-1,-1,-1,-1],
    [ 6,11, 7, 1, 2,10, 0, 8, 3, 4, 9, 5,-1,-1,-1,-1],
    [ 7, 6,11, 5, 4,10, 4, 2,10, 4, 0, 2,-1,-1,-1,-1],
    [ 3, 4, 8, 3, 5, 4, 3, 2, 5,10, 5, 2,11, 7, 6,-1],
    [ 7, 2, 3, 7, 6, 2, 5, 4, 9,-1,-1,-1,-1,-1,-1,-1],
    [ 9, 5, 4, 0, 8, 6, 0, 6, 2, 6, 8, 7,-1,-1,-1,-1],
    [ 3, 6, 2, 3, 7, 6, 1, 5, 0, 5, 4, 0,-1,-1,-1,-1],
    [ 6, 2, 8, 6, 8, 7, 2, 1, 8, 4, 8, 5, 1, 5, 8,-1],
    [ 9, 5, 4,10, 1, 6, 1, 7, 6, 1, 3, 7,-1,-1,-1,-1],
    [ 1, 6,10, 1, 7, 6, 1, 0, 7, 8, 7, 0, 9, 5, 4,-1],
    [ 4, 0,10, 4,10, 5, 0, 3,10, 6,10, 7, 3, 7,10,-1],
    [ 7, 6,10, 7,10, 8, 5, 4,10, 4, 8,10,-1,-1,-1,-1],
    [ 6, 9, 5, 6,11, 9,11, 8, 9,-1,-1,-1,-1,-1,-1,-1],
    [ 3, 6,11, 0, 6, 3, 0, 5, 6, 0, 9, 5,-1,-1,-1,-1],
    [ 0,11, 8, 0, 5,11, 0, 1, 5, 5, 6,11,-1,-1,-1,-1],
    [ 6,11, 3, 6, 3, 5, 5, 3, 1,-1,-1,-1,-1,-1,-1,-1],
    [ 1, 2,10, 9, 5,11, 9,11, 8,11, 5, 6,-1,-1,-1,-1],
    [ 0,11, 3, 0, 6,11, 0, 9, 6, 5, 6, 9, 1, 2,10,-1],
    [11, 8, 5,11, 5, 6, 8, 0, 5,10, 5, 2, 0, 2, 5,-1],
    [ 6,11, 3, 6, 3, 5, 2,10, 3,10, 5, 3,-1,-1,-1,-1],
    [ 5, 8, 9, 5, 2, 8, 5, 6, 2, 3, 8, 2,-1,-1,-1,-1],
    [ 9, 5, 6, 9, 6, 0, 0, 6, 2,-1,-1,-1,-1,-1,-1,-1],
    [ 1, 5, 8, 1, 8, 0, 5, 6, 8, 3, 8, 2, 6, 2, 8,-1],
    [ 1, 5, 6, 2, 1, 6,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 1, 3, 6, 1, 6,10, 3, 8, 6, 5, 6, 9, 8, 9, 6,-1],
    [10, 1, 0,10, 0, 6, 9, 5, 0, 5, 6, 0,-1,-1,-1,-1],
    [ 0, 3, 8, 5, 6,10,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [10, 5, 6,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [11, 5,10, 7, 5,11,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [11, 5,10,11, 7, 5, 8, 3, 0,-1,-1,-1,-1,-1,-1,-1],
    [ 5,11, 7, 5,10,11, 1, 9, 0,-1,-1,-1,-1,-1,-1,-1],
    [10, 7, 5,10,11, 7, 9, 8, 1, 8, 3, 1,-1,-1,-1,-1],
    [11, 1, 2,11, 7, 1, 7, 5, 1,-1,-1,-1,-1,-1,-1,-1],
    [ 0, 8, 3, 1, 2, 7, 1, 7, 5, 7, 2,11,-1,-1,-1,-1],
    [ 9, 7, 5, 9, 2, 7, 9, 0, 2, 2,11, 7,-1,-1,-1,-1],
    [ 7, 5, 2, 7, 2,11, 5, 9, 2, 3, 2, 8, 9, 8, 2,-1],
    [ 2, 5,10, 2, 3, 5, 3, 7, 5,-1,-1,-1,-1,-1,-1,-1],
    [ 8, 2, 0, 8, 5, 2, 8, 7, 5,10, 2, 5,-1,-1,-1,-1],
    [ 9, 0, 1, 5,10, 3, 5, 3, 7, 3,10, 2,-1,-1,-1,-1],
    [ 9, 8, 2, 9, 2, 1, 8, 7, 2,10, 2, 5, 7, 5, 2,-1],
    [ 1, 3, 5, 3, 7, 5,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 0, 8, 7, 0, 7, 1, 1, 7, 5,-1,-1,-1,-1,-1,-1,-1],
    [ 9, 0, 3, 9, 3, 5, 5, 3, 7,-1,-1,-1,-1,-1,-1,-1],
    [ 9, 8, 7, 5, 9, 7,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 5, 8, 4, 5,10, 8,10,11, 8,-1,-1,-1,-1,-1,-1,-1],
    [ 5, 0, 4, 5,11, 0, 5,10,11,11, 3, 0,-1,-1,-1,-1],
    [ 0, 1, 9, 8, 4,10, 8,10,11,10, 4, 5,-1,-1,-1,-1],
    [10,11, 4,10, 4, 5,11, 3, 4, 9, 4, 1, 3, 1, 4,-1],
    [ 2, 5, 1, 2, 8, 5, 2,11, 8, 4, 5, 8,-1,-1,-1,-1],
    [ 0, 4,11, 0,11, 3, 4, 5,11, 2,11, 1, 5, 1,11,-1],
    [ 0, 2, 5, 0, 5, 9, 2,11, 5, 4, 5, 8,11, 8, 5,-1],
    [ 9, 4, 5, 2,11, 3,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 2, 5,10, 3, 5, 2, 3, 4, 5, 3, 8, 4,-1,-1,-1,-1],
    [ 5,10, 2, 5, 2, 4, 4, 2, 0,-1,-1,-1,-1,-1,-1,-1],
    [ 3,10, 2, 3, 5,10, 3, 8, 5, 4, 5, 8, 0, 1, 9,-1],
    [ 5,10, 2, 5, 2, 4, 1, 9, 2, 9, 4, 2,-1,-1,-1,-1],
    [ 8, 4, 5, 8, 5, 3, 3, 5, 1,-1,-1,-1,-1,-1,-1,-1],
    [ 0, 4, 5, 1, 0, 5,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 8, 4, 5, 8, 5, 3, 9, 0, 5, 0, 3, 5,-1,-1,-1,-1],
    [ 9, 4, 5,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 4,11, 7, 4, 9,11, 9,10,11,-1,-1,-1,-1,-1,-1,-1],
    [ 0, 8, 3, 4, 9, 7, 9,11, 7, 9,10,11,-1,-1,-1,-1],
    [ 1,10,11, 1,11, 4, 1, 4, 0, 7, 4,11,-1,-1,-1,-1],
    [ 3, 1, 4, 3, 4, 8, 1,10, 4, 7, 4,11,10,11, 4,-1],
    [ 4,11, 7, 9,11, 4, 9, 2,11, 9, 1, 2,-1,-1,-1,-1],
    [ 9, 7, 4, 9,11, 7, 9, 1,11, 2,11, 1, 0, 8, 3,-1],
    [11, 7, 4,11, 4, 2, 2, 4, 0,-1,-1,-1,-1,-1,-1,-1],
    [11, 7, 4,11, 4, 2, 8, 3, 4, 3, 2, 4,-1,-1,-1,-1],
    [ 2, 9,10, 2, 7, 9, 2, 3, 7, 7, 4, 9,-1,-1,-1,-1],
    [ 9,10, 7, 9, 7, 4,10, 2, 7, 8, 7, 0, 2, 0, 7,-1],
    [ 3, 7,10, 3,10, 2, 7, 4,10, 1,10, 0, 4, 0,10,-1],
    [ 1,10, 2, 8, 7, 4,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 4, 9, 1, 4, 1, 7, 7, 1, 3,-1,-1,-1,-1,-1,-1,-1],
    [ 4, 9, 1, 4, 1, 7, 0, 8, 1, 8, 7, 1,-1,-1,-1,-1],
    [ 4, 0, 3, 7, 4, 3,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 4, 8, 7,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 9,10, 8,10,11, 8,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 3, 0, 9, 3, 9,11,11, 9,10,-1,-1,-1,-1,-1,-1,-1],
    [ 0, 1,10, 0,10, 8, 8,10,11,-1,-1,-1,-1,-1,-1,-1],
    [ 3, 1,10,11, 3,10,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 1, 2,11, 1,11, 9, 9,11, 8,-1,-1,-1,-1,-1,-1,-1],
    [ 3, 0, 9, 3, 9,11, 1, 2, 9, 2,11, 9,-1,-1,-1,-1],
    [ 0, 2,11, 8, 0,11,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 3, 2,11,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 2, 3, 8, 2, 8,10,10, 8, 9,-1,-1,-1,-1,-1,-1,-1],
    [ 9,10, 2, 0, 9, 2,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 2, 3, 8, 2, 8,10, 0, 1, 8, 1,10, 8,-1,-1,-1,-1],
    [ 1,10, 2,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 1, 3, 8, 9, 1, 8,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 0, 9, 1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [ 0, 3, 8,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
    [-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metaball_single_default() {
        let sop = MetaballSop;
        let params = MetaballParams::default();
        let geo = crate::generate(&sop, &params).unwrap();

        // Should produce a roughly spherical mesh
        assert!(geo.num_points() > 0, "should produce points");
        assert!(geo.num_prims() > 0, "should produce faces");

        // Bounding box should be roughly centered at origin, radius ~1
        let bb = geo.bounding_box();
        assert!(bb.max.x > 0.0 && bb.min.x < 0.0);
        assert!(bb.max.y > 0.0 && bb.min.y < 0.0);
        assert!(bb.max.z > 0.0 && bb.min.z < 0.0);
    }

    #[test]
    fn metaball_two_blending() {
        let sop = MetaballSop;
        let params = MetaballParams {
            balls: vec![
                MetaballDef {
                    center: Vec3::new(-0.5, 0.0, 0.0),
                    radius: 1.0,
                    weight: 1.0,
                },
                MetaballDef {
                    center: Vec3::new(0.5, 0.0, 0.0),
                    radius: 1.0,
                    weight: 1.0,
                },
            ],
            resolution: 24,
            ..Default::default()
        };
        let geo = crate::generate(&sop, &params).unwrap();

        // Two overlapping metaballs should produce a merged blobby shape
        assert!(geo.num_points() > 0);
        assert!(geo.num_prims() > 0);
        let bb = geo.bounding_box();
        // Should extend in both X directions beyond the centers (-0.5, 0.5)
        assert!(bb.max.x > 0.5, "max.x={} should exceed right center", bb.max.x);
        assert!(bb.min.x < -0.5, "min.x={} should exceed left center", bb.min.x);
    }

    #[test]
    fn metaball_negative_weight() {
        let sop = MetaballSop;
        let params = MetaballParams {
            balls: vec![
                MetaballDef {
                    center: Vec3::ZERO,
                    radius: 2.0,
                    weight: 1.0,
                },
                MetaballDef {
                    center: Vec3::new(1.0, 0.0, 0.0),
                    radius: 1.0,
                    weight: -0.5,
                },
            ],
            resolution: 24,
            ..Default::default()
        };
        let geo = crate::generate(&sop, &params).unwrap();

        // Negative weight should carve into the surface
        assert!(geo.num_points() > 0);
    }

    #[test]
    fn metaball_kernels() {
        // All kernel types should produce valid geometry
        for kernel in [MetaballKernel::Blinn, MetaballKernel::Wyvill, MetaballKernel::Hart] {
            let sop = MetaballSop;
            let params = MetaballParams {
                kernel,
                resolution: 16,
                ..Default::default()
            };
            let geo = crate::generate(&sop, &params).unwrap();
            assert!(geo.num_points() > 0, "kernel should produce geometry");
            assert!(geo.num_prims() > 0, "kernel should produce faces");
        }
    }

    #[test]
    fn metaball_resolution_affects_detail() {
        let sop = MetaballSop;
        let lo = crate::generate(
            &sop,
            &MetaballParams {
                resolution: 8,
                ..Default::default()
            },
        )
        .unwrap();
        let hi = crate::generate(
            &sop,
            &MetaballParams {
                resolution: 32,
                ..Default::default()
            },
        )
        .unwrap();

        // Higher resolution should produce more geometry
        assert!(hi.num_points() > lo.num_points());
        assert!(hi.num_prims() > lo.num_prims());
    }

    #[test]
    fn metaball_invalid_resolution() {
        let sop = MetaballSop;
        let params = MetaballParams {
            resolution: 1,
            ..Default::default()
        };
        assert!(crate::generate(&sop, &params).is_err());
    }

    #[test]
    fn metaball_has_density_attrib() {
        let sop = MetaballSop;
        let params = MetaballParams::default();
        let geo = crate::generate(&sop, &params).unwrap();

        let handle = geo
            .find_attrib::<f32>(AttribClass::Point, "density")
            .expect("should have density attribute");
        // All points should be near the threshold value (0.5 default)
        let threshold = 0.5f32;
        for i in 0..geo.num_points() {
            let d = geo.get_attrib(&handle, i).unwrap();
            // Density at the surface should be close to threshold
            assert!(
                (d - threshold).abs() < 0.15,
                "density={d} should be near threshold {threshold}"
            );
        }
    }
}
