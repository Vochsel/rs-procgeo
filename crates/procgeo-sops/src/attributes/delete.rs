use serde::{Deserialize, Serialize};

use procgeo_core::{AttribClass, Geometry};

use crate::{Sop, SopError};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttribDeleteParams {
    pub name: String,
    pub class: AttribClass,
}

impl Default for AttribDeleteParams {
    fn default() -> Self {
        AttribDeleteParams {
            name: "attrib1".to_string(),
            class: AttribClass::Point,
        }
    }
}

pub struct AttribDeleteSop;

impl Sop for AttribDeleteSop {
    type Params = AttribDeleteParams;

    fn name(&self) -> &'static str {
        "attrib_delete"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 1)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;
        let mut geo = inputs[0].clone();
        // Don't error if not found — just a no-op
        geo.attributes_mut().delete(params.class, &params.name);
        Ok(geo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attributes::create::{AttribCreateSop, AttribCreateParams};
    use crate::creation::box_sop::{BoxSop, BoxParams};
    use crate::{GeometryExt, generate};
    use procgeo_core::AttribType;

    #[test]
    fn delete_existing() {
        let box_geo = generate(&BoxSop, &BoxParams::default()).unwrap();

        // First create an attribute
        let create_sop = AttribCreateSop;
        let create_params = AttribCreateParams {
            name: "pscale".to_string(),
            class: AttribClass::Point,
            attrib_type: AttribType::Float,
            value_float: 1.0,
            ..Default::default()
        };
        let geo_with_attrib = box_geo.apply(&create_sop, &create_params).unwrap();

        // Verify it exists
        assert!(geo_with_attrib.find_attrib::<f32>(AttribClass::Point, "pscale").is_ok());

        // Now delete it
        let delete_sop = AttribDeleteSop;
        let delete_params = AttribDeleteParams {
            name: "pscale".to_string(),
            class: AttribClass::Point,
        };
        let result = geo_with_attrib.apply(&delete_sop, &delete_params).unwrap();

        // Verify it is gone
        assert!(result.find_attrib::<f32>(AttribClass::Point, "pscale").is_err());
    }

    #[test]
    fn delete_nonexistent() {
        let box_geo = generate(&BoxSop, &BoxParams::default()).unwrap();

        let sop = AttribDeleteSop;
        let params = AttribDeleteParams {
            name: "does_not_exist".to_string(),
            class: AttribClass::Point,
        };

        // Should pass through without error
        let result = box_geo.apply(&sop, &params);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().num_points(), 8);
    }
}
