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

        let from_count = element_count(&geo, params.from_class);
        let to_count = element_count(&geo, params.to_class);

        // Collect source float values
        let src_handle = geo.find_attrib::<f32>(params.from_class, &params.name)?;
        let src_values: Vec<f32> = (0..from_count)
            .map(|i| geo.get_attrib(&src_handle, i).unwrap_or(0.0))
            .collect();

        // Build target float values
        let dst_values: Vec<f32> = match (params.from_class, params.to_class) {
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
                        aggregate(&vals, params.method)
                    })
                    .collect()
            }

            // Point -> Detail
            (AttribClass::Point, AttribClass::Detail) => {
                vec![aggregate(&src_values, params.method)]
            }

            // Primitive -> Point
            (AttribClass::Primitive, AttribClass::Point) => {
                let num_pts = geo.num_points();
                let num_prims = geo.num_prims();

                // For each point collect values from primitives that reference it
                let mut accum: Vec<Vec<f32>> = vec![Vec::new(); num_pts];
                for prim_idx in 0..num_prims {
                    let prim_handle = PrimHandle::from_index(prim_idx);
                    let prim_val = src_values.get(prim_idx).cloned().unwrap_or(0.0);
                    for ph in geo.prim_points(prim_handle) {
                        accum[ph.index()].push(prim_val);
                    }
                }
                accum.iter().map(|vals| aggregate(vals, params.method)).collect()
            }

            // Primitive -> Detail
            (AttribClass::Primitive, AttribClass::Detail) => {
                vec![aggregate(&src_values, params.method)]
            }

            _ => {
                return Err(SopError::InvalidParam(format!(
                    "promote from {:?} to {:?} is not supported",
                    params.from_class, params.to_class
                )));
            }
        };

        // Ensure destination attribute exists
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

        // Delete original attribute if requested
        if params.delete_original {
            geo.attributes_mut().delete(params.from_class, &params.name);
        }

        Ok(geo)
    }
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
}
