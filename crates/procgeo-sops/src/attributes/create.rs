use serde::{Deserialize, Serialize};

use procgeo_core::{AttribClass, AttribDefault, AttribType, Geometry, TypeQualifier};

use crate::{Sop, SopError};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttribCreateParams {
    pub name: String,
    pub class: AttribClass,
    pub attrib_type: AttribType,
    pub value_int: i32,
    pub value_float: f32,
    pub value_vector3: [f32; 3],
    pub value_string: String,
    pub qualifier: TypeQualifier,
}

impl Default for AttribCreateParams {
    fn default() -> Self {
        AttribCreateParams {
            name: "attrib1".to_string(),
            class: AttribClass::Point,
            attrib_type: AttribType::Float,
            value_int: 0,
            value_float: 0.0,
            value_vector3: [0.0; 3],
            value_string: String::new(),
            qualifier: TypeQualifier::None,
        }
    }
}

pub struct AttribCreateSop;

impl Sop for AttribCreateSop {
    type Params = AttribCreateParams;

    fn name(&self) -> &'static str {
        "attrib_create"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 1)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;
        let mut geo = inputs[0].clone();

        let default = match params.attrib_type {
            AttribType::Int => AttribDefault::Int(params.value_int),
            AttribType::Float => AttribDefault::Float(params.value_float),
            AttribType::Vector3 => AttribDefault::Vector3(params.value_vector3),
            AttribType::String => AttribDefault::String(params.value_string.clone()),
            AttribType::Int64 => AttribDefault::Int64(params.value_int as i64),
            AttribType::Float64 => AttribDefault::Float64(params.value_float as f64),
            AttribType::Vector2 => {
                AttribDefault::Vector2([params.value_vector3[0], params.value_vector3[1]])
            }
            AttribType::Vector4 => AttribDefault::Vector4([
                params.value_vector3[0],
                params.value_vector3[1],
                params.value_vector3[2],
                0.0,
            ]),
            AttribType::Matrix3 => AttribDefault::Matrix3([0.0; 9]),
            AttribType::Matrix4 => AttribDefault::Matrix4([0.0; 16]),
        };

        // add_attrib handles resize; ignore if already exists
        let _ = geo.add_attrib(params.class, &params.name, default, params.qualifier);

        // Set non-zero values explicitly on all elements
        let count = match params.class {
            AttribClass::Point => geo.num_points(),
            AttribClass::Vertex => geo.num_vertices(),
            AttribClass::Primitive => geo.num_prims(),
            AttribClass::Detail => 1,
        };

        match params.attrib_type {
            AttribType::Int => {
                let handle = geo.find_attrib::<i32>(params.class, &params.name)?;
                for i in 0..count {
                    geo.set_attrib(&handle, i, params.value_int)?;
                }
            }
            AttribType::Int64 => {
                let handle = geo.find_attrib::<i64>(params.class, &params.name)?;
                for i in 0..count {
                    geo.set_attrib(&handle, i, params.value_int as i64)?;
                }
            }
            AttribType::Float => {
                let handle = geo.find_attrib::<f32>(params.class, &params.name)?;
                for i in 0..count {
                    geo.set_attrib(&handle, i, params.value_float)?;
                }
            }
            AttribType::Float64 => {
                let handle = geo.find_attrib::<f64>(params.class, &params.name)?;
                for i in 0..count {
                    geo.set_attrib(&handle, i, params.value_float as f64)?;
                }
            }
            AttribType::Vector2 => {
                let handle = geo.find_attrib::<[f32; 2]>(params.class, &params.name)?;
                let v = [params.value_vector3[0], params.value_vector3[1]];
                for i in 0..count {
                    geo.set_attrib(&handle, i, v)?;
                }
            }
            AttribType::Vector3 => {
                let handle = geo.find_attrib::<[f32; 3]>(params.class, &params.name)?;
                for i in 0..count {
                    geo.set_attrib(&handle, i, params.value_vector3)?;
                }
            }
            AttribType::Vector4 => {
                let handle = geo.find_attrib::<[f32; 4]>(params.class, &params.name)?;
                let v = [
                    params.value_vector3[0],
                    params.value_vector3[1],
                    params.value_vector3[2],
                    0.0,
                ];
                for i in 0..count {
                    geo.set_attrib(&handle, i, v)?;
                }
            }
            AttribType::Matrix3 => {
                let handle = geo.find_attrib::<[f32; 9]>(params.class, &params.name)?;
                for i in 0..count {
                    geo.set_attrib(&handle, i, [0.0f32; 9])?;
                }
            }
            AttribType::Matrix4 => {
                let handle = geo.find_attrib::<[f32; 16]>(params.class, &params.name)?;
                for i in 0..count {
                    geo.set_attrib(&handle, i, [0.0f32; 16])?;
                }
            }
            AttribType::String => {
                let handle = geo.find_attrib::<String>(params.class, &params.name)?;
                for i in 0..count {
                    geo.set_attrib(&handle, i, params.value_string.clone())?;
                }
            }
        }

        Ok(geo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creation::box_sop::{BoxParams, BoxSop};
    use crate::creation::grid::{GridOrientation, GridParams, GridSop};
    use crate::{GeometryExt, generate};
    use approx::assert_relative_eq;
    use glam::Vec3;

    #[test]
    fn create_float_attrib() {
        let box_geo = generate(&BoxSop, &BoxParams::default()).unwrap();
        assert_eq!(box_geo.num_points(), 8);

        let sop = AttribCreateSop;
        let params = AttribCreateParams {
            name: "pscale".to_string(),
            class: AttribClass::Point,
            attrib_type: AttribType::Float,
            value_float: 1.5,
            ..Default::default()
        };
        let result = box_geo.apply(&sop, &params).unwrap();

        let handle = result
            .find_attrib::<f32>(AttribClass::Point, "pscale")
            .unwrap();
        for i in 0..result.num_points() {
            assert_relative_eq!(result.get_attrib(&handle, i).unwrap(), 1.5, epsilon = 1e-6);
        }
    }

    #[test]
    fn create_vector3_attrib() {
        let grid = generate(
            &GridSop,
            &GridParams {
                size: [2.0, 2.0],
                rows: 3,
                cols: 3,
                center: Vec3::ZERO,
                orientation: GridOrientation::XZ,
            },
        )
        .unwrap();

        let sop = AttribCreateSop;
        let params = AttribCreateParams {
            name: "Cd".to_string(),
            class: AttribClass::Point,
            attrib_type: AttribType::Vector3,
            value_vector3: [1.0, 0.0, 0.0],
            qualifier: TypeQualifier::Color,
            ..Default::default()
        };
        let result = grid.apply(&sop, &params).unwrap();

        let handle = result
            .find_attrib::<[f32; 3]>(AttribClass::Point, "Cd")
            .unwrap();
        for i in 0..result.num_points() {
            let v = result.get_attrib(&handle, i).unwrap();
            assert_relative_eq!(v[0], 1.0, epsilon = 1e-6);
            assert_relative_eq!(v[1], 0.0, epsilon = 1e-6);
            assert_relative_eq!(v[2], 0.0, epsilon = 1e-6);
        }
    }

    #[test]
    fn create_on_prims() {
        let box_geo = generate(&BoxSop, &BoxParams::default()).unwrap();

        let sop = AttribCreateSop;
        let params = AttribCreateParams {
            name: "id".to_string(),
            class: AttribClass::Primitive,
            attrib_type: AttribType::Int,
            value_int: 42,
            ..Default::default()
        };
        let result = box_geo.apply(&sop, &params).unwrap();

        let handle = result
            .find_attrib::<i32>(AttribClass::Primitive, "id")
            .unwrap();
        for i in 0..result.num_prims() {
            assert_eq!(result.get_attrib(&handle, i).unwrap(), 42);
        }
    }

    #[test]
    fn create_on_detail() {
        let box_geo = generate(&BoxSop, &BoxParams::default()).unwrap();

        let sop = AttribCreateSop;
        let params = AttribCreateParams {
            name: "name".to_string(),
            class: AttribClass::Detail,
            attrib_type: AttribType::String,
            value_string: "test".to_string(),
            ..Default::default()
        };
        let result = box_geo.apply(&sop, &params).unwrap();

        let handle = result
            .find_attrib::<String>(AttribClass::Detail, "name")
            .unwrap();
        assert_eq!(result.get_attrib(&handle, 0).unwrap(), "test");
    }
}
