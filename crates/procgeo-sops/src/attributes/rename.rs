use serde::{Deserialize, Serialize};

use procgeo_core::{AttribClass, Geometry};

use crate::{Sop, SopError};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttribRenameParams {
    pub from_name: String,
    pub to_name: String,
    pub class: AttribClass,
}

impl Default for AttribRenameParams {
    fn default() -> Self {
        AttribRenameParams {
            from_name: "attrib1".to_string(),
            to_name: "attrib2".to_string(),
            class: AttribClass::Point,
        }
    }
}

pub struct AttribRenameSop;

impl Sop for AttribRenameSop {
    type Params = AttribRenameParams;

    fn name(&self) -> &'static str {
        "attrib_rename"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 1)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;
        let mut geo = inputs[0].clone();
        geo.attributes_mut()
            .rename(params.class, &params.from_name, &params.to_name)?;
        Ok(geo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attributes::create::{AttribCreateSop, AttribCreateParams};
    use crate::creation::box_sop::{BoxSop, BoxParams};
    use crate::{GeometryExt, generate};
    use procgeo_core::{AttribType, TypeQualifier};
    use approx::assert_relative_eq;

    #[test]
    fn rename_attribute() {
        let box_geo = generate(&BoxSop, &BoxParams::default()).unwrap();

        // Create "Cd" attribute
        let create_sop = AttribCreateSop;
        let create_params = AttribCreateParams {
            name: "Cd".to_string(),
            class: AttribClass::Point,
            attrib_type: AttribType::Vector3,
            value_vector3: [0.5, 0.5, 0.5],
            qualifier: TypeQualifier::Color,
            ..Default::default()
        };
        let geo_with_attrib = box_geo.apply(&create_sop, &create_params).unwrap();

        // Verify "Cd" exists
        assert!(geo_with_attrib.find_attrib::<[f32; 3]>(AttribClass::Point, "Cd").is_ok());

        // Rename "Cd" -> "color"
        let rename_sop = AttribRenameSop;
        let rename_params = AttribRenameParams {
            from_name: "Cd".to_string(),
            to_name: "color".to_string(),
            class: AttribClass::Point,
        };
        let result = geo_with_attrib.apply(&rename_sop, &rename_params).unwrap();

        // Old name should be gone
        assert!(result.find_attrib::<[f32; 3]>(AttribClass::Point, "Cd").is_err());

        // New name should exist with the same data
        let handle = result.find_attrib::<[f32; 3]>(AttribClass::Point, "color").unwrap();
        for i in 0..result.num_points() {
            let v = result.get_attrib(&handle, i).unwrap();
            assert_relative_eq!(v[0], 0.5, epsilon = 1e-6);
            assert_relative_eq!(v[1], 0.5, epsilon = 1e-6);
            assert_relative_eq!(v[2], 0.5, epsilon = 1e-6);
        }
    }
}
