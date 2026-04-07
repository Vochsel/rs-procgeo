use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};

use glam::Vec3;

use procgeo_core::{Geometry, PointHandle, PrimHandle};

use crate::{Sop, SopError};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub enum SortEntity {
    #[default]
    Points,
    Primitives,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub enum SortMode {
    #[default]
    ByAxis,
    Reverse,
    Random,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub enum SortAxis {
    X,
    #[default]
    Y,
    Z,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SortParams {
    pub entity: SortEntity,
    pub mode: SortMode,
    pub axis: SortAxis,
    pub seed: u64,
}

pub struct SortSop;

fn axis_value(pos: Vec3, axis: SortAxis) -> f32 {
    match axis {
        SortAxis::X => pos.x,
        SortAxis::Y => pos.y,
        SortAxis::Z => pos.z,
    }
}

impl Sop for SortSop {
    type Params = SortParams;

    fn name(&self) -> &'static str {
        "sort"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 1)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;
        let geo = inputs[0];

        match params.entity {
            SortEntity::Points => sort_points(geo, params),
            SortEntity::Primitives => sort_prims(geo, params),
        }
    }
}

fn sort_points(geo: &Geometry, params: &SortParams) -> Result<Geometry, SopError> {
    let num_points = geo.num_points();

    // Build sorted order of point indices
    let mut order: Vec<usize> = (0..num_points).collect();

    match params.mode {
        SortMode::ByAxis => {
            order.sort_by(|&a, &b| {
                let pa = geo.point_pos(PointHandle::from_index(a));
                let pb = geo.point_pos(PointHandle::from_index(b));
                let va = axis_value(pa, params.axis);
                let vb = axis_value(pb, params.axis);
                va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        SortMode::Reverse => {
            order.reverse();
        }
        SortMode::Random => {
            let mut rng = StdRng::seed_from_u64(params.seed);
            order.shuffle(&mut rng);
        }
    }

    // Build point remap: old_index -> new_index
    let mut old_to_new = vec![0usize; num_points];
    for (new_idx, &old_idx) in order.iter().enumerate() {
        old_to_new[old_idx] = new_idx;
    }

    // Build new geometry
    let mut out = Geometry::new();

    // Add points in sorted order
    for &old_idx in &order {
        let pos = geo.point_pos(PointHandle::from_index(old_idx));
        out.add_point(pos);
    }

    // Add prims with remapped point indices
    for prim_idx in 0..geo.num_prims() {
        let ph = PrimHandle::from_index(prim_idx);
        let old_pts = geo.prim_points(ph);
        let new_pts: Vec<PointHandle> = old_pts
            .iter()
            .map(|&h| PointHandle::from_index(old_to_new[h.index()]))
            .collect();

        let prim = geo.prim(ph);
        match prim {
            procgeo_core::Primitive::Polygon(poly) => match poly.poly_type {
                procgeo_core::PolyType::Closed => {
                    out.add_face(&new_pts);
                }
                procgeo_core::PolyType::Open => {
                    out.add_polyline(&new_pts);
                }
            },
        }
    }

    Ok(out)
}

fn sort_prims(geo: &Geometry, params: &SortParams) -> Result<Geometry, SopError> {
    let num_prims = geo.num_prims();

    // Compute centroid for each prim
    let centroids: Vec<Vec3> = (0..num_prims)
        .map(|prim_idx| {
            let ph = PrimHandle::from_index(prim_idx);
            let pts = geo.prim_points(ph);
            if pts.is_empty() {
                Vec3::ZERO
            } else {
                pts.iter().map(|&h| geo.point_pos(h)).sum::<Vec3>() / pts.len() as f32
            }
        })
        .collect();

    // Build sorted order of prim indices
    let mut order: Vec<usize> = (0..num_prims).collect();

    match params.mode {
        SortMode::ByAxis => {
            order.sort_by(|&a, &b| {
                let va = axis_value(centroids[a], params.axis);
                let vb = axis_value(centroids[b], params.axis);
                va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        SortMode::Reverse => {
            order.reverse();
        }
        SortMode::Random => {
            let mut rng = StdRng::seed_from_u64(params.seed);
            order.shuffle(&mut rng);
        }
    }

    // Build new geometry: copy all points as-is
    let mut out = Geometry::new();
    for i in 0..geo.num_points() {
        let pos = geo.point_pos(PointHandle::from_index(i));
        out.add_point(pos);
    }

    // Add prims in sorted order
    for &old_prim_idx in &order {
        let ph = PrimHandle::from_index(old_prim_idx);
        let old_pts = geo.prim_points(ph);
        // Points stay at same indices, no remap needed
        let prim = geo.prim(ph);
        match prim {
            procgeo_core::Primitive::Polygon(poly) => match poly.poly_type {
                procgeo_core::PolyType::Closed => {
                    out.add_face(&old_pts);
                }
                procgeo_core::PolyType::Open => {
                    out.add_polyline(&old_pts);
                }
            },
        }
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creation::box_sop::{BoxSop, BoxParams};
    use crate::{GeometryExt, generate};

    fn make_box() -> Geometry {
        generate(&BoxSop, &BoxParams::default()).unwrap()
    }

    #[test]
    fn sort_points_by_x() {
        // Create 3 points in order (1,0,0), (0,0,0), (2,0,0), sort by X → positions should be 0,1,2
        let mut geo = Geometry::new();
        geo.add_point(Vec3::new(1.0, 0.0, 0.0));
        geo.add_point(Vec3::new(0.0, 0.0, 0.0));
        geo.add_point(Vec3::new(2.0, 0.0, 0.0));

        let params = SortParams {
            entity: SortEntity::Points,
            mode: SortMode::ByAxis,
            axis: SortAxis::X,
            seed: 0,
        };
        let result = geo.apply(&SortSop, &params).unwrap();

        assert_eq!(result.num_points(), 3);
        // Sorted by X: 0.0, 1.0, 2.0
        let p0 = result.point_pos(PointHandle::from_index(0));
        let p1 = result.point_pos(PointHandle::from_index(1));
        let p2 = result.point_pos(PointHandle::from_index(2));
        assert!((p0.x - 0.0).abs() < 1e-6, "expected x=0.0, got {}", p0.x);
        assert!((p1.x - 1.0).abs() < 1e-6, "expected x=1.0, got {}", p1.x);
        assert!((p2.x - 2.0).abs() < 1e-6, "expected x=2.0, got {}", p2.x);
    }

    #[test]
    fn sort_points_reverse() {
        let mut geo = Geometry::new();
        geo.add_point(Vec3::new(0.0, 0.0, 0.0));
        geo.add_point(Vec3::new(1.0, 0.0, 0.0));
        geo.add_point(Vec3::new(2.0, 0.0, 0.0));

        let params = SortParams {
            entity: SortEntity::Points,
            mode: SortMode::Reverse,
            axis: SortAxis::X,
            seed: 0,
        };
        let result = geo.apply(&SortSop, &params).unwrap();

        assert_eq!(result.num_points(), 3);
        // Reversed: 2.0, 1.0, 0.0
        let p0 = result.point_pos(PointHandle::from_index(0));
        let p2 = result.point_pos(PointHandle::from_index(2));
        assert!((p0.x - 2.0).abs() < 1e-6, "expected x=2.0, got {}", p0.x);
        assert!((p2.x - 0.0).abs() < 1e-6, "expected x=0.0, got {}", p2.x);
    }

    #[test]
    fn sort_prims_by_axis() {
        // Use a box and sort prims by their centroid Y
        let box_geo = make_box();
        let num_prims = box_geo.num_prims();
        assert_eq!(num_prims, 6);

        let params = SortParams {
            entity: SortEntity::Primitives,
            mode: SortMode::ByAxis,
            axis: SortAxis::Y,
            seed: 0,
        };
        let result = box_geo.apply(&SortSop, &params).unwrap();

        assert_eq!(result.num_prims(), num_prims);
        assert_eq!(result.num_points(), 8);

        // Verify prims are sorted by centroid Y
        let mut prev_y = f32::NEG_INFINITY;
        for prim_idx in 0..result.num_prims() {
            let ph = PrimHandle::from_index(prim_idx);
            let pts = result.prim_points(ph);
            let centroid_y = pts.iter().map(|&h| result.point_pos(h).y).sum::<f32>() / pts.len() as f32;
            assert!(centroid_y >= prev_y - 1e-6, "prims not sorted by Y");
            prev_y = centroid_y;
        }
    }
}
