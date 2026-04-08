// Patch edge quantization: determine the integer number of quad subdivisions
// for each patch side.
//
// This implements a simplified version of the ILP (Integer Linear Programming)
// approach from QuadWild. The original uses Gurobi for optimization; here we
// use a least-squares heuristic with parity constraints.
//
// Constraints:
// - Opposite sides of a 4-sided patch must have equal subdivision count
// - Side lengths should be proportional to their geometric length / target edge size
// - Minimum 1 subdivision per side

use procgeo_core::{Geometry, PointHandle};

use super::patches::{Patch, PatchDecomposition};

/// Quantization result: integer subdivisions per patch side.
#[derive(Clone, Debug)]
pub struct QuantizedPatches {
    /// For each patch, the number of subdivisions on each side.
    pub subdivisions: Vec<Vec<u32>>,
    /// Target edge length used for quantization.
    pub target_edge: f32,
}

/// Quantize patch sides to integer subdivision counts.
pub fn quantize_patches(
    geo: &Geometry,
    decomp: &PatchDecomposition,
    target_edge: f32,
    _alpha: f32,
) -> QuantizedPatches {
    let mut subdivisions = Vec::with_capacity(decomp.patches.len());

    for patch in &decomp.patches {
        let side_subdivs = quantize_single_patch(geo, patch, target_edge);
        subdivisions.push(side_subdivs);
    }

    // Apply parity constraints across shared patch edges
    enforce_parity_constraints(&decomp.patches, &mut subdivisions);

    QuantizedPatches {
        subdivisions,
        target_edge,
    }
}

/// Quantize a single patch's sides.
fn quantize_single_patch(geo: &Geometry, patch: &Patch, target_edge: f32) -> Vec<u32> {
    if patch.sides.is_empty() {
        return vec![1];
    }

    let mut result = Vec::with_capacity(patch.sides.len());

    for side in &patch.sides {
        let length = compute_side_length(geo, side);
        let subdivs = if target_edge > 0.0 {
            (length / target_edge).round().max(1.0) as u32
        } else {
            1
        };
        result.push(subdivs);
    }

    // For 4-sided patches, enforce opposite sides equal
    if result.len() == 4 {
        // Average opposite sides
        let avg_02 = ((result[0] + result[2]) as f32 / 2.0).round().max(1.0) as u32;
        let avg_13 = ((result[1] + result[3]) as f32 / 2.0).round().max(1.0) as u32;
        result[0] = avg_02;
        result[2] = avg_02;
        result[1] = avg_13;
        result[3] = avg_13;
    }

    result
}

/// Compute the geometric length of a patch side (polyline of boundary vertices).
fn compute_side_length(geo: &Geometry, side: &[usize]) -> f32 {
    if side.len() < 2 {
        return 0.0;
    }
    let mut length = 0.0f32;
    for i in 0..side.len() - 1 {
        let p0 = geo.point_pos(PointHandle::from_index(side[i]));
        let p1 = geo.point_pos(PointHandle::from_index(side[i + 1]));
        length += (p1 - p0).length();
    }
    length
}

/// Enforce parity constraints: shared edges between patches should have
/// the same subdivision count on both sides.
fn enforce_parity_constraints(patches: &[Patch], subdivisions: &mut [Vec<u32>]) {
    // Build a map of shared boundaries between patches
    // Two patches share a boundary if they have common boundary vertices
    for pi in 0..patches.len() {
        for pj in (pi + 1)..patches.len() {
            // Find shared side pairs
            for (si, side_i) in patches[pi].sides.iter().enumerate() {
                for (sj, side_j) in patches[pj].sides.iter().enumerate() {
                    if sides_share_edge(side_i, side_j) {
                        // These sides should have the same subdivision count
                        if si < subdivisions[pi].len() && sj < subdivisions[pj].len() {
                            let avg = (subdivisions[pi][si] + subdivisions[pj][sj] + 1) / 2;
                            let avg = avg.max(1);
                            subdivisions[pi][si] = avg;
                            subdivisions[pj][sj] = avg;
                        }
                    }
                }
            }
        }
    }
}

/// Check if two sides share common vertices (indicating they are the same edge).
fn sides_share_edge(side_a: &[usize], side_b: &[usize]) -> bool {
    if side_a.len() < 2 || side_b.len() < 2 {
        return false;
    }

    // Check if significant overlap in vertex sets
    let mut shared = 0;
    for &va in side_a {
        if side_b.contains(&va) {
            shared += 1;
        }
    }

    // At least 2 shared vertices means they share an edge segment
    shared >= 2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantize_respects_minimum() {
        let mut geo = procgeo_core::Geometry::new();
        let _p0 = geo.add_point(glam::Vec3::new(0.0, 0.0, 0.0));
        let _p1 = geo.add_point(glam::Vec3::new(0.1, 0.0, 0.0)); // Very short edge

        let patch = Patch {
            faces: vec![0],
            boundary_edges: vec![],
            boundary_verts: vec![0, 1],
            corners: vec![0, 1],
            sides: vec![vec![0, 1]],
            num_sides: 1,
        };

        let subdivs = quantize_single_patch(&geo, &patch, 10.0); // Very large target
        assert!(subdivs[0] >= 1, "minimum 1 subdivision");
    }

    #[test]
    fn quad_patch_opposite_sides_equal() {
        let mut geo = procgeo_core::Geometry::new();
        geo.add_point(glam::Vec3::new(0.0, 0.0, 0.0));
        geo.add_point(glam::Vec3::new(3.0, 0.0, 0.0));
        geo.add_point(glam::Vec3::new(3.0, 2.0, 0.0));
        geo.add_point(glam::Vec3::new(0.0, 2.0, 0.0));

        let patch = Patch {
            faces: vec![],
            boundary_edges: vec![],
            boundary_verts: vec![0, 1, 2, 3],
            corners: vec![0, 1, 2, 3],
            sides: vec![
                vec![0, 1], // length 3
                vec![1, 2], // length 2
                vec![2, 3], // length 3
                vec![3, 0], // length 2
            ],
            num_sides: 4,
        };

        let subdivs = quantize_single_patch(&geo, &patch, 1.0);
        assert_eq!(subdivs.len(), 4);
        assert_eq!(subdivs[0], subdivs[2], "opposite sides should match");
        assert_eq!(subdivs[1], subdivs[3], "opposite sides should match");
    }
}
