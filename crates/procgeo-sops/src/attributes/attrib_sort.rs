use serde::{Deserialize, Serialize};

use procgeo_core::{AttribClass, AttribType, Geometry};

use crate::{Sop, SopError};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub enum AttribSortOrder {
    #[default]
    Ascending,
    Descending,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttribSortParams {
    pub attrib_name: String,
    pub class: AttribClass,
    pub attrib_type: AttribType,
    pub order: AttribSortOrder,
    /// Which component to use for sorting vector attributes.
    pub component: usize,
}

impl Default for AttribSortParams {
    fn default() -> Self {
        AttribSortParams {
            attrib_name: "attrib".to_string(),
            class: AttribClass::Point,
            attrib_type: AttribType::Float,
            order: AttribSortOrder::Ascending,
            component: 0,
        }
    }
}

pub struct AttribSortSop;

fn element_count(geo: &Geometry, class: AttribClass) -> usize {
    match class {
        AttribClass::Point => geo.num_points(),
        AttribClass::Vertex => geo.num_vertices(),
        AttribClass::Primitive => geo.num_prims(),
        AttribClass::Detail => 1,
    }
}

impl Sop for AttribSortSop {
    type Params = AttribSortParams;

    fn name(&self) -> &'static str {
        "attrib_sort"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 1)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;
        let mut out = inputs[0].clone();

        let count = element_count(&out, params.class);
        if count == 0 {
            return Ok(out);
        }

        match params.attrib_type {
            AttribType::Float => {
                let handle = out
                    .find_attrib::<f32>(params.class, &params.attrib_name)
                    .map_err(SopError::Core)?;

                let mut values: Vec<f32> = (0..count)
                    .map(|i| out.get_attrib(&handle, i).unwrap_or(0.0))
                    .collect();

                values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

                if matches!(params.order, AttribSortOrder::Descending) {
                    values.reverse();
                }

                for (i, v) in values.into_iter().enumerate() {
                    out.set_attrib(&handle, i, v)?;
                }
            }

            AttribType::Vector3 => {
                let handle = out
                    .find_attrib::<[f32; 3]>(params.class, &params.attrib_name)
                    .map_err(SopError::Core)?;

                let comp = params.component.min(2);
                let mut values: Vec<[f32; 3]> = (0..count)
                    .map(|i| out.get_attrib(&handle, i).unwrap_or([0.0; 3]))
                    .collect();

                values.sort_by(|a, b| {
                    a[comp].partial_cmp(&b[comp]).unwrap_or(std::cmp::Ordering::Equal)
                });

                if matches!(params.order, AttribSortOrder::Descending) {
                    values.reverse();
                }

                for (i, v) in values.into_iter().enumerate() {
                    out.set_attrib(&handle, i, v)?;
                }
            }

            other => {
                return Err(SopError::InvalidParam(format!(
                    "AttribSort: unsupported attrib_type {:?}",
                    other
                )));
            }
        }

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attributes::create::{AttribCreateSop, AttribCreateParams};
    use crate::creation::box_sop::{BoxSop, BoxParams};
    use crate::{GeometryExt, generate};
    use approx::assert_relative_eq;

    fn make_box_with_float_attrib(name: &str, values: &[f32]) -> Geometry {
        let geo = generate(&BoxSop, &BoxParams::default()).unwrap();
        let sop = AttribCreateSop;
        let params = AttribCreateParams {
            name: name.to_string(),
            class: AttribClass::Point,
            attrib_type: AttribType::Float,
            value_float: 0.0,
            ..Default::default()
        };
        let mut result = geo.apply(&sop, &params).unwrap();
        let handle = result
            .find_attrib::<f32>(AttribClass::Point, name)
            .unwrap();
        for (i, &v) in values.iter().enumerate() {
            result.set_attrib(&handle, i, v).unwrap();
        }
        result
    }

    #[test]
    fn sort_float_ascending() {
        let values = vec![5.0, 2.0, 8.0, 1.0, 9.0, 3.0, 7.0, 4.0];
        let geo = make_box_with_float_attrib("val", &values);

        let sop = AttribSortSop;
        let params = AttribSortParams {
            attrib_name: "val".to_string(),
            class: AttribClass::Point,
            attrib_type: AttribType::Float,
            order: AttribSortOrder::Ascending,
            component: 0,
        };

        let result = geo.apply(&sop, &params).unwrap();
        let handle = result
            .find_attrib::<f32>(AttribClass::Point, "val")
            .unwrap();

        let mut sorted = values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        for i in 0..sorted.len() {
            let v = result.get_attrib(&handle, i).unwrap();
            assert_relative_eq!(v, sorted[i], epsilon = 1e-6);
        }
    }

    #[test]
    fn sort_descending() {
        let values = vec![5.0, 2.0, 8.0, 1.0, 9.0, 3.0, 7.0, 4.0];
        let geo = make_box_with_float_attrib("val", &values);

        let sop = AttribSortSop;
        let params = AttribSortParams {
            attrib_name: "val".to_string(),
            class: AttribClass::Point,
            attrib_type: AttribType::Float,
            order: AttribSortOrder::Descending,
            component: 0,
        };

        let result = geo.apply(&sop, &params).unwrap();
        let handle = result
            .find_attrib::<f32>(AttribClass::Point, "val")
            .unwrap();

        let mut sorted = values.clone();
        sorted.sort_by(|a, b| b.partial_cmp(a).unwrap());

        for i in 0..sorted.len() {
            let v = result.get_attrib(&handle, i).unwrap();
            assert_relative_eq!(v, sorted[i], epsilon = 1e-6);
        }
    }

    #[test]
    fn sort_vector_by_component() {
        // 8 points with Vector3 attrib; sort by Y (component 1)
        let geo = generate(&BoxSop, &BoxParams::default()).unwrap();
        let sop_create = AttribCreateSop;
        let params_create = AttribCreateParams {
            name: "vel".to_string(),
            class: AttribClass::Point,
            attrib_type: AttribType::Vector3,
            value_vector3: [0.0; 3],
            ..Default::default()
        };
        let mut geo = geo.apply(&sop_create, &params_create).unwrap();

        let vecs: Vec<[f32; 3]> = vec![
            [1.0, 5.0, 0.0],
            [1.0, 2.0, 0.0],
            [1.0, 8.0, 0.0],
            [1.0, 1.0, 0.0],
            [1.0, 9.0, 0.0],
            [1.0, 3.0, 0.0],
            [1.0, 7.0, 0.0],
            [1.0, 4.0, 0.0],
        ];
        let handle = geo.find_attrib::<[f32; 3]>(AttribClass::Point, "vel").unwrap();
        for (i, &v) in vecs.iter().enumerate() {
            geo.set_attrib(&handle, i, v).unwrap();
        }

        let sort_sop = AttribSortSop;
        let sort_params = AttribSortParams {
            attrib_name: "vel".to_string(),
            class: AttribClass::Point,
            attrib_type: AttribType::Vector3,
            order: AttribSortOrder::Ascending,
            component: 1,
        };

        let result = geo.apply(&sort_sop, &sort_params).unwrap();
        let handle = result
            .find_attrib::<[f32; 3]>(AttribClass::Point, "vel")
            .unwrap();

        // Y components should be sorted ascending
        let y_values: Vec<f32> = (0..8)
            .map(|i| result.get_attrib(&handle, i).unwrap()[1])
            .collect();

        for i in 0..y_values.len() - 1 {
            assert!(
                y_values[i] <= y_values[i + 1],
                "not sorted at index {i}: {} > {}",
                y_values[i],
                y_values[i + 1]
            );
        }
    }
}
