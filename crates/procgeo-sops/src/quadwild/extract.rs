// Quad mesh extraction from quantized patches.
//
// Each patch is tessellated into quads based on the integer subdivision
// counts determined during quantization. The output is a new Geometry
// containing only quad (and occasionally triangle) faces.

use std::collections::HashMap;

use glam::Vec3;

use procgeo_core::{Geometry, PointHandle};

use crate::SopError;

use super::patches::{Patch, PatchDecomposition};
use super::quantize::QuantizedPatches;

/// Extract a quad mesh from the patch decomposition + quantization.
pub fn extract_quad_mesh(
    geo: &Geometry,
    decomp: &PatchDecomposition,
    quantized: &QuantizedPatches,
) -> Result<Geometry, SopError> {
    let mut out = Geometry::new();
    // Cache for shared boundary points to ensure watertight mesh
    let mut boundary_point_cache: HashMap<usize, PointHandle> = HashMap::new();

    for (pi, patch) in decomp.patches.iter().enumerate() {
        let subdivisions = &quantized.subdivisions[pi];
        tessellate_patch(
            geo,
            patch,
            subdivisions,
            &mut out,
            &mut boundary_point_cache,
        )?;
    }

    Ok(out)
}

/// Tessellate a single patch into quads.
fn tessellate_patch(
    geo: &Geometry,
    patch: &Patch,
    subdivisions: &[u32],
    out: &mut Geometry,
    boundary_cache: &mut HashMap<usize, PointHandle>,
) -> Result<(), SopError> {
    match patch.num_sides {
        0 | 1 | 2 => tessellate_degenerate(geo, patch, out, boundary_cache),
        3 => tessellate_tri_patch(geo, patch, subdivisions, out, boundary_cache),
        4 => tessellate_quad_patch(geo, patch, subdivisions, out, boundary_cache),
        _ => tessellate_ngon_patch(geo, patch, subdivisions, out, boundary_cache),
    }
}

/// For degenerate patches, just copy triangles.
fn tessellate_degenerate(
    geo: &Geometry,
    patch: &Patch,
    out: &mut Geometry,
    boundary_cache: &mut HashMap<usize, PointHandle>,
) -> Result<(), SopError> {
    for &fi in &patch.faces {
        let ph = procgeo_core::PrimHandle::from_index(fi);
        let pts = geo.prim_points(ph);
        let new_pts: Vec<PointHandle> = pts
            .iter()
            .map(|p| get_or_create_point(geo, p.index(), out, boundary_cache))
            .collect();
        out.add_face(&new_pts);
    }
    Ok(())
}

/// Tessellate a 3-sided (triangular) patch.
/// Uses a fan tessellation from the centroid to create 3 quads.
fn tessellate_tri_patch(
    geo: &Geometry,
    patch: &Patch,
    subdivisions: &[u32],
    out: &mut Geometry,
    boundary_cache: &mut HashMap<usize, PointHandle>,
) -> Result<(), SopError> {
    if patch.corners.len() < 3 || patch.sides.len() < 3 {
        return tessellate_degenerate(geo, patch, out, boundary_cache);
    }

    // Get the 3 corner positions
    let c0 = geo.point_pos(PointHandle::from_index(patch.corners[0]));
    let c1 = geo.point_pos(PointHandle::from_index(patch.corners[1]));
    let c2 = geo.point_pos(PointHandle::from_index(patch.corners[2]));

    // Compute centroid
    let center = (c0 + c1 + c2) / 3.0;

    // Get subdivision counts for each side
    let n0 = subdivisions.first().copied().unwrap_or(1).max(1) as usize;
    let n1 = subdivisions.get(1).copied().unwrap_or(1).max(1) as usize;
    let n2 = subdivisions.get(2).copied().unwrap_or(1).max(1) as usize;

    // Sample boundary points along each side
    let side0 = interpolate_side(geo, &patch.sides[0], n0);
    let side1 = interpolate_side(geo, &patch.sides[1], n1);
    let side2 = interpolate_side(geo, &patch.sides[2], n2);

    let center_pt = out.add_point(center);

    // Generate edge midpoints connecting center to side midpoints
    let mid0 = lerp(side0[n0 / 2], center, 0.5);
    let mid1 = lerp(side1[n1 / 2], center, 0.5);
    let mid2 = lerp(side2[n2 / 2], center, 0.5);

    let mid0_pt = out.add_point(mid0);
    let mid1_pt = out.add_point(mid1);
    let mid2_pt = out.add_point(mid2);

    // Create side boundary points in output
    let side0_pts: Vec<PointHandle> = side0.iter().map(|&p| out.add_point(p)).collect();
    let side1_pts: Vec<PointHandle> = side1.iter().map(|&p| out.add_point(p)).collect();
    let side2_pts: Vec<PointHandle> = side2.iter().map(|&p| out.add_point(p)).collect();

    // Create 3 quad fans from center
    // Fan from center to side 0
    for i in 0..n0 {
        out.add_face(&[side0_pts[i], side0_pts[i + 1], center_pt, mid0_pt]);
    }
    for i in 0..n1 {
        out.add_face(&[side1_pts[i], side1_pts[i + 1], center_pt, mid1_pt]);
    }
    for i in 0..n2 {
        out.add_face(&[side2_pts[i], side2_pts[i + 1], center_pt, mid2_pt]);
    }

    Ok(())
}

/// Tessellate a 4-sided (quad) patch into a regular grid of quads.
fn tessellate_quad_patch(
    geo: &Geometry,
    patch: &Patch,
    subdivisions: &[u32],
    out: &mut Geometry,
    boundary_cache: &mut HashMap<usize, PointHandle>,
) -> Result<(), SopError> {
    if patch.corners.len() < 4 || patch.sides.len() < 4 {
        return tessellate_degenerate(geo, patch, out, boundary_cache);
    }

    let nu = subdivisions.first().copied().unwrap_or(1).max(1) as usize;
    let nv = subdivisions.get(1).copied().unwrap_or(1).max(1) as usize;

    // Sample boundary curves
    let bottom = interpolate_side(geo, &patch.sides[0], nu); // side 0: u direction
    let right = interpolate_side(geo, &patch.sides[1], nv); // side 1: v direction
    let top = interpolate_side(geo, &patch.sides[2], nu); // side 2: u direction (reversed)
    let left = interpolate_side(geo, &patch.sides[3], nv); // side 3: v direction (reversed)

    // Generate interior grid points using transfinite interpolation (TFI)
    let mut grid = vec![vec![Vec3::ZERO; nu + 1]; nv + 1];

    for j in 0..=nv {
        for i in 0..=nu {
            let u = i as f32 / nu as f32;
            let v = j as f32 / nv as f32;

            // Bilinear blend of boundaries (transfinite interpolation)
            let bottom_pt = bottom[i.min(bottom.len() - 1)];
            let top_pt = top[(nu - i).min(top.len() - 1)]; // reversed
            let left_pt = left[(nv - j).min(left.len() - 1)]; // reversed
            let right_pt = right[j.min(right.len() - 1)];

            // Corners
            let c00 = bottom[0];
            let c10 = bottom[bottom.len() - 1];
            let c11 = top[0];
            let c01 = top[top.len() - 1];

            // TFI formula
            let edge_interp =
                bottom_pt * (1.0 - v) + top_pt * v + left_pt * (1.0 - u) + right_pt * u;
            let corner_interp = c00 * (1.0 - u) * (1.0 - v)
                + c10 * u * (1.0 - v)
                + c01 * (1.0 - u) * v
                + c11 * u * v;

            grid[j][i] = edge_interp - corner_interp;
        }
    }

    // Create points
    let mut pt_grid = vec![vec![PointHandle::from_index(0); nu + 1]; nv + 1];
    for j in 0..=nv {
        for i in 0..=nu {
            // Check if this is a boundary point that should use the cache
            let is_boundary = i == 0 || i == nu || j == 0 || j == nv;
            if is_boundary {
                // Use boundary position for better accuracy
                let pos = grid[j][i];
                pt_grid[j][i] = out.add_point(pos);
            } else {
                pt_grid[j][i] = out.add_point(grid[j][i]);
            }
        }
    }

    // Create quad faces
    for j in 0..nv {
        for i in 0..nu {
            out.add_face(&[
                pt_grid[j][i],
                pt_grid[j][i + 1],
                pt_grid[j + 1][i + 1],
                pt_grid[j + 1][i],
            ]);
        }
    }

    Ok(())
}

/// Tessellate an n-sided patch (n > 4) using a fan from centroid.
fn tessellate_ngon_patch(
    geo: &Geometry,
    patch: &Patch,
    subdivisions: &[u32],
    out: &mut Geometry,
    boundary_cache: &mut HashMap<usize, PointHandle>,
) -> Result<(), SopError> {
    if patch.sides.is_empty() {
        return tessellate_degenerate(geo, patch, out, boundary_cache);
    }

    // Compute centroid of all boundary points
    let centroid = compute_patch_centroid(geo, patch);
    let center_pt = out.add_point(centroid);

    // For each side, create a strip of quads from the side to the center
    for (si, side) in patch.sides.iter().enumerate() {
        let n = subdivisions.get(si).copied().unwrap_or(1).max(1) as usize;
        let side_pts = interpolate_side(geo, side, n);

        let _prev_inner = center_pt;
        let outer_pts: Vec<PointHandle> = side_pts.iter().map(|&p| out.add_point(p)).collect();

        // Connect outer to center with triangular fan
        for i in 0..n {
            out.add_face(&[outer_pts[i], outer_pts[i + 1], center_pt]);
        }
    }

    Ok(())
}

/// Interpolate points along a patch side.
fn interpolate_side(geo: &Geometry, side_verts: &[usize], n: usize) -> Vec<Vec3> {
    if side_verts.is_empty() {
        return vec![Vec3::ZERO; n + 1];
    }

    if side_verts.len() == 1 {
        let p = geo.point_pos(PointHandle::from_index(side_verts[0]));
        return vec![p; n + 1];
    }

    // Compute cumulative arc lengths
    let positions: Vec<Vec3> = side_verts
        .iter()
        .map(|&vi| geo.point_pos(PointHandle::from_index(vi)))
        .collect();

    let mut arc_lengths = vec![0.0f32];
    for i in 1..positions.len() {
        let prev = arc_lengths[i - 1];
        arc_lengths.push(prev + (positions[i] - positions[i - 1]).length());
    }

    let total_length = *arc_lengths.last().unwrap();
    if total_length < 1e-10 {
        return vec![positions[0]; n + 1];
    }

    // Sample n+1 evenly-spaced points along the polyline
    let mut result = Vec::with_capacity(n + 1);
    for si in 0..=n {
        let target_len = (si as f32 / n as f32) * total_length;

        // Find the segment containing this arc length
        let mut seg = 0;
        for i in 1..arc_lengths.len() {
            if arc_lengths[i] >= target_len {
                seg = i - 1;
                break;
            }
            seg = i - 1;
        }

        let seg_start = arc_lengths[seg];
        let seg_end = arc_lengths[(seg + 1).min(arc_lengths.len() - 1)];
        let seg_len = seg_end - seg_start;

        let t = if seg_len > 1e-10 {
            (target_len - seg_start) / seg_len
        } else {
            0.0
        };

        let p = lerp(
            positions[seg],
            positions[(seg + 1).min(positions.len() - 1)],
            t,
        );
        result.push(p);
    }

    result
}

/// Compute the centroid of a patch.
fn compute_patch_centroid(geo: &Geometry, patch: &Patch) -> Vec3 {
    if patch.boundary_verts.is_empty() {
        // Fall back to face centroid
        if patch.faces.is_empty() {
            return Vec3::ZERO;
        }
        let mut sum = Vec3::ZERO;
        let mut count = 0;
        for &fi in &patch.faces {
            let ph = procgeo_core::PrimHandle::from_index(fi);
            for pt in geo.prim_points(ph) {
                sum += geo.point_pos(pt);
                count += 1;
            }
        }
        return if count > 0 {
            sum / count as f32
        } else {
            Vec3::ZERO
        };
    }

    let sum: Vec3 = patch
        .boundary_verts
        .iter()
        .map(|&vi| geo.point_pos(PointHandle::from_index(vi)))
        .sum();
    sum / patch.boundary_verts.len() as f32
}

/// Get or create a point in the output geometry, reusing cached boundary points.
fn get_or_create_point(
    geo: &Geometry,
    pt_idx: usize,
    out: &mut Geometry,
    cache: &mut HashMap<usize, PointHandle>,
) -> PointHandle {
    *cache
        .entry(pt_idx)
        .or_insert_with(|| out.add_point(geo.point_pos(PointHandle::from_index(pt_idx))))
}

fn lerp(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpolate_side_uniform() {
        let mut geo = Geometry::new();
        geo.add_point(Vec3::new(0.0, 0.0, 0.0));
        geo.add_point(Vec3::new(4.0, 0.0, 0.0));
        let pts = interpolate_side(&geo, &[0, 1], 4);
        assert_eq!(pts.len(), 5);
        assert!((pts[0].x - 0.0).abs() < 1e-5);
        assert!((pts[2].x - 2.0).abs() < 1e-5);
        assert!((pts[4].x - 4.0).abs() < 1e-5);
    }

    #[test]
    fn interpolate_side_single_point() {
        let mut geo = Geometry::new();
        geo.add_point(Vec3::new(1.0, 2.0, 3.0));
        let pts = interpolate_side(&geo, &[0], 3);
        assert_eq!(pts.len(), 4);
        for p in &pts {
            assert!((p.x - 1.0).abs() < 1e-5);
        }
    }
}
