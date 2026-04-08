use serde::{Deserialize, Serialize};

use procgeo_core::{AttribClass, AttribDefault, Geometry, TypeQualifier};

use crate::{Sop, SopError};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnumerateParams {
    /// Name of the integer attribute to create.
    pub name: String,
    /// Which class (Point, Primitive, etc.) to enumerate.
    pub class: AttribClass,
    /// Starting index value.
    pub start: i32,
}

impl Default for EnumerateParams {
    fn default() -> Self {
        EnumerateParams {
            name: "index".to_string(),
            class: AttribClass::Point,
            start: 0,
        }
    }
}

pub struct EnumerateSop;

impl Sop for EnumerateSop {
    type Params = EnumerateParams;

    fn name(&self) -> &'static str {
        "enumerate"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 1)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;
        let mut out = inputs[0].clone();

        out.add_attrib(
            params.class,
            &params.name,
            AttribDefault::Int(params.start),
            TypeQualifier::None,
        )?;

        let handle = out.find_attrib::<i32>(params.class, &params.name)?;

        let count = match params.class {
            AttribClass::Point => out.num_points(),
            AttribClass::Vertex => out.num_vertices(),
            AttribClass::Primitive => out.num_prims(),
            AttribClass::Detail => 1,
        };

        for i in 0..count {
            out.set_attrib(&handle, i, params.start + i as i32)?;
        }

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creation::box_sop::{BoxParams, BoxSop};
    use crate::{GeometryExt, generate};

    fn make_box() -> Geometry {
        generate(&BoxSop, &BoxParams::default()).unwrap()
    }

    #[test]
    fn enumerate_points() {
        // 8-point box → 0..7
        let box_geo = make_box();
        assert_eq!(box_geo.num_points(), 8);

        let params = EnumerateParams::default(); // Point, "index", start=0
        let result = box_geo.apply(&EnumerateSop, &params).unwrap();

        let handle = result
            .find_attrib::<i32>(AttribClass::Point, "index")
            .unwrap();
        for i in 0..result.num_points() {
            assert_eq!(result.get_attrib(&handle, i).unwrap(), i as i32);
        }
    }

    #[test]
    fn enumerate_prims() {
        // 6-prim box → 0..5
        let box_geo = make_box();
        assert_eq!(box_geo.num_prims(), 6);

        let params = EnumerateParams {
            name: "prim_idx".to_string(),
            class: AttribClass::Primitive,
            start: 0,
        };
        let result = box_geo.apply(&EnumerateSop, &params).unwrap();

        let handle = result
            .find_attrib::<i32>(AttribClass::Primitive, "prim_idx")
            .unwrap();
        for i in 0..result.num_prims() {
            assert_eq!(result.get_attrib(&handle, i).unwrap(), i as i32);
        }
    }

    #[test]
    fn enumerate_with_offset() {
        // start=10 → 10..17 for 8-point box
        let box_geo = make_box();
        let params = EnumerateParams {
            name: "index".to_string(),
            class: AttribClass::Point,
            start: 10,
        };
        let result = box_geo.apply(&EnumerateSop, &params).unwrap();

        let handle = result
            .find_attrib::<i32>(AttribClass::Point, "index")
            .unwrap();
        for i in 0..result.num_points() {
            assert_eq!(result.get_attrib(&handle, i).unwrap(), 10 + i as i32);
        }
    }
}
