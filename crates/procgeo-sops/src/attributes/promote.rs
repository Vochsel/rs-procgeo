use serde::{Deserialize, Serialize};

use procgeo_core::{AttribClass, AttribDefault, Geometry, PrimHandle, TypeQualifier};

use crate::{Sop, SopError};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub enum PromoteMethod {
    First,
    Last,
    Min,
    Max,
    #[default]
    Average,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttribPromoteParams {
    pub name: String,
    pub from_class: AttribClass,
    pub to_class: AttribClass,
    pub method: PromoteMethod,
    pub delete_original: bool,
}

impl Default for AttribPromoteParams {
    fn default() -> Self {
        AttribPromoteParams {
            name: "attrib".to_string(),
            from_class: AttribClass::Point,
            to_class: AttribClass::Primitive,
            method: PromoteMethod::Average,
            delete_original: true,
        }
    }
}

pub struct AttribPromoteSop;

/// Aggregate a slice of floats using the given method.
fn aggregate(values: &[f32], method: PromoteMethod) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    match method {
        PromoteMethod::First => values[0],
        PromoteMethod::Last => *values.last().unwrap(),
        PromoteMethod::Min => values.iter().cloned().fold(f32::INFINITY, f32::min),
        PromoteMethod::Max => values.iter().cloned().fold(f32::NEG_INFINITY, f32::max),
        PromoteMethod::Average => values.iter().sum::<f32>() / values.len() as f32,
    }
}

/// Aggregate a slice of Vector3 values per-component.
fn aggregate_vec3(values: &[[f32; 3]], method: PromoteMethod) -> [f32; 3] {
    if values.is_empty() {
        return [0.0; 3];
    }
    [
        aggregate(&values.iter().map(|v| v[0]).collect::<Vec<_>>(), method),
        aggregate(&values.iter().map(|v| v[1]).collect::<Vec<_>>(), method),
        aggregate(&values.iter().map(|v| v[2]).collect::<Vec<_>>(), method),
    ]
}

impl Sop for AttribPromoteSop {
    type Params = AttribPromoteParams;

    fn name(&self) -> &'static str {
        "attrib_promote"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 1)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;
        let mut geo = inputs[0].clone();

        // Try float first, then Vector3
        let is_vec3 = geo.find_attrib::<[f32; 3]>(params.from_class, &params.name).is_ok()
            && geo.find_attrib::<f32>(params.from_class, &params.name).is_err();

        if is_vec3 {
            promote_vec3(&mut geo, params)?;
        } else {
            promote_float(&mut geo, params)?;
        }

        Ok(geo)
    }
}

fn promote_float(geo: &mut Geometry, params: &AttribPromoteParams) -> Result<(), SopError> {
    let from_count = element_count(geo, params.from_class);
    let to_count = element_count(geo, params.to_class);

    let src_handle = geo.find_attrib::<f32>(params.from_class, &params.name)?;
    let src_values: Vec<f32> = (0..from_count)
        .map(|i| geo.get_attrib(&src_handle, i).unwrap_or(0.0))
        .collect();

    let dst_values: Vec<f32> = build_float_dst_values(
        geo,
        &src_values,
        params.from_class,
        params.to_class,
        to_count,
        params.method,
    )?;

    let _ = geo.add_attrib(
        params.to_class,
        &params.name,
        AttribDefault::Float(0.0),
        TypeQualifier::None,
    );
    let dst_handle = geo.find_attrib::<f32>(params.to_class, &params.name)?;
    for (i, &v) in dst_values.iter().enumerate() {
        geo.set_attrib(&dst_handle, i, v)?;
    }

    if params.delete_original {
        geo.attributes_mut().delete(params.from_class, &params.name);
    }

    Ok(())
}

fn promote_vec3(geo: &mut Geometry, params: &AttribPromoteParams) -> Result<(), SopError> {
    let from_count = element_count(geo, params.from_class);
    let to_count = element_count(geo, params.to_class);

    let src_handle = geo.find_attrib::<[f32; 3]>(params.from_class, &params.name)?;
    let src_values: Vec<[f32; 3]> = (0..from_count)
        .map(|i| geo.get_attrib(&src_handle, i).unwrap_or([0.0; 3]))
        .collect();

    let dst_values: Vec<[f32; 3]> = build_vec3_dst_values(
        geo,
        &src_values,
        params.from_class,
        params.to_class,
        to_count,
        params.method,
    )?;

    let _ = geo.add_attrib(
        params.to_class,
        &params.name,
        AttribDefault::Vector3([0.0; 3]),
        TypeQualifier::None,
    );
    let dst_handle = geo.find_attrib::<[f32; 3]>(params.to_class, &params.name)?;
    for (i, &v) in dst_values.iter().enumerate() {
        geo.set_attrib(&dst_handle, i, v)?;
    }

    if params.delete_original {
        geo.attributes_mut().delete(params.from_class, &params.name);
    }

    Ok(())
}

fn build_float_dst_values(
    geo: &Geometry,
    src_values: &[f32],
    from_class: AttribClass,
    to_class: AttribClass,
    to_count: usize,
    method: PromoteMethod,
) -> Result<Vec<f32>, SopError> {
    let result = match (from_class, to_class) {
        // Point -> Primitive
        (AttribClass::Point, AttribClass::Primitive) => {
            (0..to_count)
                .map(|prim_idx| {
                    let prim_handle = PrimHandle::from_index(prim_idx);
                    let pt_handles = geo.prim_points(prim_handle);
                    let vals: Vec<f32> = pt_handles
                        .iter()
                        .map(|ph| src_values.get(ph.index()).cloned().unwrap_or(0.0))
                        .collect();
                    aggregate(&vals, method)
                })
                .collect()
        }

        // Point -> Detail
        (AttribClass::Point, AttribClass::Detail) => {
            vec![aggregate(src_values, method)]
        }

        // Primitive -> Point
        (AttribClass::Primitive, AttribClass::Point) => {
            let num_pts = geo.num_points();
            let num_prims = geo.num_prims();
            let mut accum: Vec<Vec<f32>> = vec![Vec::new(); num_pts];
            for prim_idx in 0..num_prims {
                let prim_handle = PrimHandle::from_index(prim_idx);
                let prim_val = src_values.get(prim_idx).cloned().unwrap_or(0.0);
                for ph in geo.prim_points(prim_handle) {
                    accum[ph.index()].push(prim_val);
                }
            }
            accum.iter().map(|vals| aggregate(vals, method)).collect()
        }

        // Primitive -> Detail
        (AttribClass::Primitive, AttribClass::Detail) => {
            vec![aggregate(src_values, method)]
        }

        // Detail -> Point (broadcast)
        (AttribClass::Detail, AttribClass::Point) => {
            let detail_val = src_values.first().cloned().unwrap_or(0.0);
            vec![detail_val; to_count]
        }

        // Detail -> Primitive (broadcast)
        (AttribClass::Detail, AttribClass::Primitive) => {
            let detail_val = src_values.first().cloned().unwrap_or(0.0);
            vec![detail_val; to_count]
        }

        // Vertex -> Point
        (AttribClass::Vertex, AttribClass::Point) => {
            let num_pts = geo.num_points();
            let num_prims = geo.num_prims();
            let mut accum: Vec<Vec<f32>> = vec![Vec::new(); num_pts];
            for prim_idx in 0..num_prims {
                let prim_handle = PrimHandle::from_index(prim_idx);
                let verts = geo.prim_vertices(prim_handle);
                let pts = geo.prim_points(prim_handle);
                for (vert_handle, pt_handle) in verts.iter().zip(pts.iter()) {
                    let vi = vert_handle.index();
                    let pi = pt_handle.index();
                    let v = src_values.get(vi).cloned().unwrap_or(0.0);
                    accum[pi].push(v);
                }
            }
            accum.iter().map(|vals| aggregate(vals, method)).collect()
        }

        _ => {
            return Err(SopError::InvalidParam(format!(
                "promote from {:?} to {:?} is not supported",
                from_class, to_class
            )));
        }
    };
    Ok(result)
}

fn build_vec3_dst_values(
    geo: &Geometry,
    src_values: &[[f32; 3]],
    from_class: AttribClass,
    to_class: AttribClass,
    to_count: usize,
    method: PromoteMethod,
) -> Result<Vec<[f32; 3]>, SopError> {
    let result = match (from_class, to_class) {
        (AttribClass::Point, AttribClass::Primitive) => {
            (0..to_count)
                .map(|prim_idx| {
                    let prim_handle = PrimHandle::from_index(prim_idx);
                    let pt_handles = geo.prim_points(prim_handle);
                    let vals: Vec<[f32; 3]> = pt_handles
                        .iter()
                        .map(|ph| src_values.get(ph.index()).cloned().unwrap_or([0.0; 3]))
                        .collect();
                    aggregate_vec3(&vals, method)
                })
                .collect()
        }

        (AttribClass::Point, AttribClass::Detail) => {
            vec![aggregate_vec3(src_values, method)]
        }

        (AttribClass::Primitive, AttribClass::Point) => {
            let num_pts = geo.num_points();
            let num_prims = geo.num_prims();
            let mut accum: Vec<Vec<[f32; 3]>> = vec![Vec::new(); num_pts];
            for prim_idx in 0..num_prims {
                let prim_handle = PrimHandle::from_index(prim_idx);
                let prim_val = src_values.get(prim_idx).cloned().unwrap_or([0.0; 3]);
                for ph in geo.prim_points(prim_handle) {
                    accum[ph.index()].push(prim_val);
                }
            }
            accum.iter().map(|vals| aggregate_vec3(vals, method)).collect()
        }

        (AttribClass::Primitive, AttribClass::Detail) => {
            vec![aggregate_vec3(src_values, method)]
        }

        (AttribClass::Detail, AttribClass::Point) => {
            let detail_val = src_values.first().cloned().unwrap_or([0.0; 3]);
            vec![detail_val; to_count]
        }

        (AttribClass::Detail, AttribClass::Primitive) => {
            let detail_val = src_values.first().cloned().unwrap_or([0.0; 3]);
            vec![detail_val; to_count]
        }

        _ => {
            return Err(SopError::InvalidParam(format!(
                "promote vec3 from {:?} to {:?} is not supported",
                from_class, to_class
            )));
        }
    };
    Ok(result)
}

fn element_count(geo: &Geometry, class: AttribClass) -> usize {
    match class {
        AttribClass::Point => geo.num_points(),
        AttribClass::Vertex => geo.num_vertices(),
        AttribClass::Primitive => geo.num_prims(),
        AttribClass::Detail => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attributes::create::{AttribCreateSop, AttribCreateParams};
    use crate::creation::grid::{GridSop, GridParams, GridOrientation};
    use crate::{GeometryExt, generate};
    use procgeo_core::AttribType;
    use approx::assert_relative_eq;
    use glam::Vec3;

    fn make_grid_with_attrib() -> Geometry {
        // 3x3 grid (4 quads)
        let grid = generate(&GridSop, &GridParams {
            size: [2.0, 2.0],
            rows: 3,
            cols: 3,
            center: Vec3::ZERO,
            orientation: GridOrientation::XZ,
        })
        .unwrap();

        // Create "val" attribute on points: value = index as f32
        let sop = AttribCreateSop;
        let params = AttribCreateParams {
            name: "val".to_string(),
            class: AttribClass::Point,
            attrib_type: AttribType::Float,
            value_float: 0.0,
            ..Default::default()
        };
        let mut geo = grid.apply(&sop, &params).unwrap();

        // Set per-point values
        let handle = geo.find_attrib::<f32>(AttribClass::Point, "val").unwrap();
        for i in 0..geo.num_points() {
            geo.set_attrib(&handle, i, i as f32).unwrap();
        }
        geo
    }

    #[test]
    fn promote_point_to_prim_avg() {
        let geo = make_grid_with_attrib();
        let num_prims = geo.num_prims();

        let sop = AttribPromoteSop;
        let params = AttribPromoteParams {
            name: "val".to_string(),
            from_class: AttribClass::Point,
            to_class: AttribClass::Primitive,
            method: PromoteMethod::Average,
            delete_original: false,
        };
        let result = geo.apply(&sop, &params).unwrap();

        let handle = result.find_attrib::<f32>(AttribClass::Primitive, "val").unwrap();
        // Verify all prims have a reasonable average (between 0 and num_points-1)
        let max_pt = result.num_points() as f32 - 1.0;
        for i in 0..num_prims {
            let v = result.get_attrib(&handle, i).unwrap();
            assert!(v >= 0.0 && v <= max_pt, "prim {i} val {v} out of range");
        }
    }

    #[test]
    fn promote_point_to_detail() {
        let geo = make_grid_with_attrib();
        let num_pts = geo.num_points();
        // expected average of 0..num_pts
        let expected_avg = (num_pts - 1) as f32 / 2.0;

        let sop = AttribPromoteSop;
        let params = AttribPromoteParams {
            name: "val".to_string(),
            from_class: AttribClass::Point,
            to_class: AttribClass::Detail,
            method: PromoteMethod::Average,
            delete_original: false,
        };
        let result = geo.apply(&sop, &params).unwrap();

        let handle = result.find_attrib::<f32>(AttribClass::Detail, "val").unwrap();
        let v = result.get_attrib(&handle, 0).unwrap();
        assert_relative_eq!(v, expected_avg, epsilon = 1e-4);
    }

    #[test]
    fn promote_deletes_original() {
        let geo = make_grid_with_attrib();

        // Verify attrib exists on points
        assert!(geo.find_attrib::<f32>(AttribClass::Point, "val").is_ok());

        let sop = AttribPromoteSop;
        let params = AttribPromoteParams {
            name: "val".to_string(),
            from_class: AttribClass::Point,
            to_class: AttribClass::Primitive,
            method: PromoteMethod::Average,
            delete_original: true,
        };
        let result = geo.apply(&sop, &params).unwrap();

        // Source attrib should be gone
        assert!(result.find_attrib::<f32>(AttribClass::Point, "val").is_err());
        // Destination attrib should exist
        assert!(result.find_attrib::<f32>(AttribClass::Primitive, "val").is_ok());
    }

    #[test]
    fn promote_min_max() {
        let geo = make_grid_with_attrib();
        let num_prims = geo.num_prims();

        let sop = AttribPromoteSop;

        // Min: each prim should have the minimum point index of its corners
        let min_result = sop.execute(&[&geo], &AttribPromoteParams {
            name: "val".to_string(),
            from_class: AttribClass::Point,
            to_class: AttribClass::Primitive,
            method: PromoteMethod::Min,
            delete_original: false,
        }).unwrap();

        let max_result = sop.execute(&[&geo], &AttribPromoteParams {
            name: "val".to_string(),
            from_class: AttribClass::Point,
            to_class: AttribClass::Primitive,
            method: PromoteMethod::Max,
            delete_original: false,
        }).unwrap();

        let h_min = min_result.find_attrib::<f32>(AttribClass::Primitive, "val").unwrap();
        let h_max = max_result.find_attrib::<f32>(AttribClass::Primitive, "val").unwrap();

        for i in 0..num_prims {
            let min_v = min_result.get_attrib(&h_min, i).unwrap();
            let max_v = max_result.get_attrib(&h_max, i).unwrap();
            assert!(
                min_v <= max_v,
                "prim {i}: min ({min_v}) should be <= max ({max_v})"
            );
        }
    }

    #[test]
    fn promote_vector3() {
        // Make a grid with a Vector3 point attribute
        let grid = generate(&GridSop, &GridParams {
            size: [2.0, 2.0],
            rows: 3,
            cols: 3,
            center: Vec3::ZERO,
            orientation: GridOrientation::XZ,
        }).unwrap();

        let sop_create = AttribCreateSop;
        let params_create = AttribCreateParams {
            name: "vel".to_string(),
            class: AttribClass::Point,
            attrib_type: AttribType::Vector3,
            value_vector3: [0.0; 3],
            ..Default::default()
        };
        let mut geo = grid.apply(&sop_create, &params_create).unwrap();

        // Set each point's velocity to (i, i*2, i*3)
        let handle = geo.find_attrib::<[f32; 3]>(AttribClass::Point, "vel").unwrap();
        for i in 0..geo.num_points() {
            geo.set_attrib(&handle, i, [i as f32, i as f32 * 2.0, i as f32 * 3.0]).unwrap();
        }

        let sop = AttribPromoteSop;
        let result = sop.execute(&[&geo], &AttribPromoteParams {
            name: "vel".to_string(),
            from_class: AttribClass::Point,
            to_class: AttribClass::Primitive,
            method: PromoteMethod::Average,
            delete_original: false,
        }).unwrap();

        let h = result.find_attrib::<[f32; 3]>(AttribClass::Primitive, "vel").unwrap();
        let num_prims = result.num_prims();
        let max_pt = result.num_points() as f32 - 1.0;

        for i in 0..num_prims {
            let v = result.get_attrib(&h, i).unwrap();
            assert!(
                v[0] >= 0.0 && v[0] <= max_pt,
                "prim {i} vel.x={} out of range",
                v[0]
            );
        }
    }

    #[test]
    fn promote_detail_to_point() {
        // Create a Detail float attribute with value 42.0, then broadcast to all points
        let grid = generate(&GridSop, &GridParams {
            size: [2.0, 2.0],
            rows: 3,
            cols: 3,
            center: Vec3::ZERO,
            orientation: GridOrientation::XZ,
        }).unwrap();

        let sop_create = AttribCreateSop;
        let mut geo = grid.apply(&sop_create, &AttribCreateParams {
            name: "scale".to_string(),
            class: AttribClass::Detail,
            attrib_type: AttribType::Float,
            value_float: 42.0,
            ..Default::default()
        }).unwrap();

        // Ensure the detail attribute is set
        let dh = geo.find_attrib::<f32>(AttribClass::Detail, "scale").unwrap();
        geo.set_attrib(&dh, 0, 42.0).unwrap();

        let sop = AttribPromoteSop;
        let result = sop.execute(&[&geo], &AttribPromoteParams {
            name: "scale".to_string(),
            from_class: AttribClass::Detail,
            to_class: AttribClass::Point,
            method: PromoteMethod::Average,
            delete_original: false,
        }).unwrap();

        let h = result.find_attrib::<f32>(AttribClass::Point, "scale").unwrap();
        for i in 0..result.num_points() {
            let v = result.get_attrib(&h, i).unwrap();
            assert_relative_eq!(v, 42.0, epsilon = 1e-4);
        }
    }
}
