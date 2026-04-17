use glam::Vec3;
use serde::{Deserialize, Serialize};

use procgeo_core::{Geometry, PointHandle};

use crate::{Sop, SopError};

/// Houdini-style Add SOP: append explicit points, then connect them into
/// closed polygons or open polylines.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AddParams {
    /// New points to append, expressed as `[x, y, z]`.
    pub points: Vec<[f32; 3]>,
    /// Closed polygons expressed as point-index lists.
    pub polygons: Vec<Vec<usize>>,
    /// Open polylines expressed as point-index lists.
    pub polylines: Vec<Vec<usize>>,
}

pub struct AddSop;

fn strip_redundant_polygon_closure(indices: &[usize]) -> &[usize] {
    if indices.len() >= 2 && indices.first() == indices.last() {
        &indices[..indices.len() - 1]
    } else {
        indices
    }
}

fn validate_indices(
    kind: &str,
    indices: &[usize],
    min_points: usize,
    total_points: usize,
) -> Result<(), SopError> {
    if indices.len() < min_points {
        return Err(SopError::InvalidParam(format!(
            "{kind} requires at least {min_points} points, got {}",
            indices.len()
        )));
    }

    if let Some(&bad) = indices.iter().find(|&&idx| idx >= total_points) {
        return Err(SopError::InvalidParam(format!(
            "{kind} references point index {bad}, but only {total_points} points exist after append"
        )));
    }

    Ok(())
}

fn point_refs(indices: &[usize]) -> Vec<PointHandle> {
    indices
        .iter()
        .map(|&idx| PointHandle::from_index(idx))
        .collect()
}

impl Sop for AddSop {
    type Params = AddParams;

    fn name(&self) -> &'static str {
        "add"
    }

    fn input_count(&self) -> (usize, usize) {
        (0, 1)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;

        let mut out = inputs.first().map(|geo| (*geo).clone()).unwrap_or_else(|| {
            Geometry::with_capacity(
                params.points.len(),
                params.polygons.len() + params.polylines.len(),
            )
        });

        for point in &params.points {
            out.add_point(Vec3::new(point[0], point[1], point[2]));
        }

        let total_points = out.num_points();

        for polygon in &params.polygons {
            let polygon = strip_redundant_polygon_closure(polygon);
            validate_indices("polygon", polygon, 3, total_points)?;
            let handles = point_refs(polygon);
            out.add_face(&handles);
        }

        for polyline in &params.polylines {
            validate_indices("polyline", polyline, 2, total_points)?;
            let handles = point_refs(polyline);
            out.add_polyline(&handles);
        }

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reshape::poly_extrude::{PolyExtrudeParams, PolyExtrudeSop};
    use crate::reshape::poly_wire::{PolyWireParams, PolyWireSop};
    use crate::{GeometryExt, generate};
    use approx::assert_relative_eq;
    use procgeo_core::{PolyType, PrimHandle, Primitive};

    #[test]
    fn add_default_is_empty() {
        let geo = generate(&AddSop, &AddParams::default()).unwrap();

        assert_eq!(geo.num_points(), 0);
        assert_eq!(geo.num_prims(), 0);
        assert_eq!(geo.num_vertices(), 0);
    }

    #[test]
    fn add_builds_polygon_and_polyline() {
        let geo = generate(
            &AddSop,
            &AddParams {
                points: vec![
                    [0.0, 0.0, 0.0],
                    [1.0, 0.0, 0.0],
                    [1.0, 0.0, 1.0],
                    [0.0, 0.0, 1.0],
                    [0.0, 1.0, 0.0],
                    [0.5, 1.5, 0.0],
                    [1.0, 1.0, 0.0],
                ],
                polygons: vec![vec![0, 3, 2, 1]],
                polylines: vec![vec![4, 5, 6]],
            },
        )
        .unwrap();

        assert_eq!(geo.num_points(), 7);
        assert_eq!(geo.num_prims(), 2);

        match geo.prim(PrimHandle::from_index(0)) {
            Primitive::Polygon(poly) => assert_eq!(poly.poly_type, PolyType::Closed),
        }
        match geo.prim(PrimHandle::from_index(1)) {
            Primitive::Polygon(poly) => assert_eq!(poly.poly_type, PolyType::Open),
        }
    }

    #[test]
    fn add_allows_redundant_polygon_closure() {
        let geo = generate(
            &AddSop,
            &AddParams {
                points: vec![
                    [0.0, 0.0, 0.0],
                    [1.0, 0.0, 0.0],
                    [1.0, 0.0, 1.0],
                    [0.0, 0.0, 1.0],
                ],
                polygons: vec![vec![0, 3, 2, 1, 0]],
                polylines: Vec::new(),
            },
        )
        .unwrap();

        assert_eq!(geo.num_prims(), 1);
        assert_eq!(geo.prim_points(PrimHandle::from_index(0)).len(), 4);
    }

    #[test]
    fn add_appends_to_input_geometry() {
        let input = generate(
            &crate::creation::line::LineSop,
            &crate::creation::line::LineParams::default(),
        )
        .unwrap();

        let result = input
            .apply(
                &AddSop,
                &AddParams {
                    points: vec![[1.0, 0.0, 0.0], [1.0, 0.0, 1.0]],
                    polygons: vec![vec![0, 2, 3, 1]],
                    polylines: Vec::new(),
                },
            )
            .unwrap();

        assert_eq!(result.num_points(), 4);
        assert_eq!(result.num_prims(), 2);

        let bbox = result.bounding_box();
        assert_relative_eq!(bbox.max.x, 1.0, epsilon = 1e-5);
        assert_relative_eq!(bbox.max.z, 1.0, epsilon = 1e-5);
    }

    #[test]
    fn add_polygon_can_be_extruded() {
        let profile = generate(
            &AddSop,
            &AddParams {
                points: vec![
                    [-0.5, 0.0, -0.5],
                    [0.5, 0.0, -0.5],
                    [0.5, 0.0, 0.5],
                    [-0.5, 0.0, 0.5],
                ],
                polygons: vec![vec![0, 3, 2, 1]],
                polylines: Vec::new(),
            },
        )
        .unwrap();

        let extruded = profile
            .apply(
                &PolyExtrudeSop,
                &PolyExtrudeParams {
                    distance: 2.0,
                    ..Default::default()
                },
            )
            .unwrap();

        assert_eq!(extruded.num_prims(), 5);
        assert_eq!(extruded.num_points(), 8);

        for point_idx in 4..8 {
            let pos = extruded.point_pos(PointHandle::from_index(point_idx));
            assert_relative_eq!(pos.y, 2.0, epsilon = 1e-5);
        }
    }

    #[test]
    fn add_polyline_can_be_polywired() {
        let curve = generate(
            &AddSop,
            &AddParams {
                points: vec![
                    [0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0],
                    [1.0, 1.5, 0.0],
                    [1.0, 2.5, 0.5],
                ],
                polygons: Vec::new(),
                polylines: vec![vec![0, 1, 2, 3]],
            },
        )
        .unwrap();

        let wired = curve
            .apply(
                &PolyWireSop,
                &PolyWireParams {
                    radius: 0.2,
                    divisions: 6,
                },
            )
            .unwrap();

        assert_eq!(wired.num_points(), 4 * 6);
        assert_eq!(wired.num_prims(), 3 * 6);
    }

    #[test]
    fn add_rejects_out_of_bounds_indices() {
        let err = generate(
            &AddSop,
            &AddParams {
                points: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
                polygons: Vec::new(),
                polylines: vec![vec![0, 2]],
            },
        )
        .unwrap_err();

        assert!(err.to_string().contains("references point index 2"));
    }

    #[test]
    fn add_rejects_too_short_primitives() {
        let err = generate(
            &AddSop,
            &AddParams {
                points: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
                polygons: vec![vec![0, 1]],
                polylines: Vec::new(),
            },
        )
        .unwrap_err();

        assert!(
            err.to_string()
                .contains("polygon requires at least 3 points")
        );
    }
}
