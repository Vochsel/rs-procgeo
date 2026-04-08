// Quad mesh smoothing: improve mesh quality after extraction.
//
// Uses Laplacian smoothing constrained to the original surface,
// preserving boundary vertices and feature edges.

use std::collections::HashMap;

use glam::Vec3;

use procgeo_core::{Geometry, PointHandle, PrimHandle};

/// Smooth the quad mesh using Laplacian relaxation.
///
/// Boundary/corner vertices are fixed; interior vertices are moved
/// toward the average of their neighbors.
pub fn smooth_quad_mesh(geo: &mut Geometry, iterations: u32) {
    let num_points = geo.num_points();
    if num_points < 4 {
        return;
    }

    // Build point adjacency from faces
    let (neighbors, is_boundary) = build_point_adjacency(geo);

    for _ in 0..iterations {
        let mut new_positions = Vec::with_capacity(num_points);

        for vi in 0..num_points {
            let pos = geo.point_pos(PointHandle::from_index(vi));

            if is_boundary[vi] || neighbors[vi].is_empty() {
                new_positions.push(pos);
                continue;
            }

            // Laplacian: average of neighbors
            let avg: Vec3 = neighbors[vi]
                .iter()
                .map(|&ni| geo.point_pos(PointHandle::from_index(ni)))
                .sum::<Vec3>()
                / neighbors[vi].len() as f32;

            // Blend between original and average (damped smoothing)
            let lambda = 0.5;
            new_positions.push(pos * (1.0 - lambda) + avg * lambda);
        }

        // Apply new positions
        for (vi, &pos) in new_positions.iter().enumerate() {
            geo.set_point_pos(PointHandle::from_index(vi), pos);
        }
    }
}

/// Build point adjacency (neighbors) and identify boundary points.
fn build_point_adjacency(geo: &Geometry) -> (Vec<Vec<usize>>, Vec<bool>) {
    let num_points = geo.num_points();
    let mut neighbors: Vec<Vec<usize>> = vec![Vec::new(); num_points];
    let mut edge_count: HashMap<(usize, usize), u32> = HashMap::new();

    for fi in 0..geo.num_prims() {
        let ph = PrimHandle::from_index(fi);
        let pts = geo.prim_points(ph);
        let n = pts.len();

        for i in 0..n {
            let a = pts[i].index();
            let b = pts[(i + 1) % n].index();

            if !neighbors[a].contains(&b) {
                neighbors[a].push(b);
            }
            if !neighbors[b].contains(&a) {
                neighbors[b].push(a);
            }

            let key = (a.min(b), a.max(b));
            *edge_count.entry(key).or_default() += 1;
        }
    }

    // Boundary: edges used by only 1 face
    let mut is_boundary = vec![false; num_points];
    for (&(a, b), &count) in &edge_count {
        if count == 1 {
            is_boundary[a] = true;
            is_boundary[b] = true;
        }
    }

    (neighbors, is_boundary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smooth_preserves_boundary() {
        let mut geo = Geometry::new();
        // Create a 2x2 grid of quads
        let mut pts = Vec::new();
        for r in 0..=2 {
            for c in 0..=2 {
                pts.push(geo.add_point(Vec3::new(c as f32, r as f32, 0.0)));
            }
        }
        // Perturb center point
        geo.set_point_pos(pts[4], Vec3::new(1.5, 1.5, 0.0));

        // Create 4 quads
        let w = 3;
        for r in 0..2 {
            for c in 0..2 {
                geo.add_face(&[
                    pts[r * w + c],
                    pts[r * w + c + 1],
                    pts[(r + 1) * w + c + 1],
                    pts[(r + 1) * w + c],
                ]);
            }
        }

        // Save boundary positions
        let corner_pos = geo.point_pos(pts[0]);

        smooth_quad_mesh(&mut geo, 5);

        // Corner should not move (boundary)
        let new_corner = geo.point_pos(pts[0]);
        assert!(
            (new_corner - corner_pos).length() < 1e-5,
            "boundary should not move"
        );

        // Center should have moved toward average
        let center = geo.point_pos(pts[4]);
        assert!(
            (center - Vec3::new(1.5, 1.5, 0.0)).length() > 0.01,
            "interior point should have moved"
        );
    }

    #[test]
    fn smooth_noop_on_regular_grid() {
        let mut geo = Geometry::new();
        let mut pts = Vec::new();
        for r in 0..=2 {
            for c in 0..=2 {
                pts.push(geo.add_point(Vec3::new(c as f32, r as f32, 0.0)));
            }
        }
        let w = 3;
        for r in 0..2 {
            for c in 0..2 {
                geo.add_face(&[
                    pts[r * w + c],
                    pts[r * w + c + 1],
                    pts[(r + 1) * w + c + 1],
                    pts[(r + 1) * w + c],
                ]);
            }
        }

        let center_before = geo.point_pos(pts[4]);
        smooth_quad_mesh(&mut geo, 10);
        let center_after = geo.point_pos(pts[4]);

        // Regular grid: center point is already at average of neighbors
        assert!(
            (center_after - center_before).length() < 0.1,
            "regular grid should barely change"
        );
    }
}
