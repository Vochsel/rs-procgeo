use serde::{Deserialize, Serialize};

use procgeo_core::{AttribClass, AttribDefault, Geometry, TypeQualifier};

use crate::{Sop, SopError};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ColorParams {
    /// RGB color to assign to all points.
    pub color: [f32; 3],
}

impl Default for ColorParams {
    fn default() -> Self {
        ColorParams {
            color: [1.0, 1.0, 1.0],
        }
    }
}

pub struct ColorSop;

impl Sop for ColorSop {
    type Params = ColorParams;

    fn name(&self) -> &'static str {
        "color"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 1)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;
        let mut out = inputs[0].clone();

        // Create "Cd" attribute if it doesn't already exist; ignore error if already present
        let _ = out.add_attrib(
            AttribClass::Point,
            "Cd",
            AttribDefault::Vector3([1.0, 1.0, 1.0]),
            TypeQualifier::Color,
        );

        let handle = out
            .find_attrib::<[f32; 3]>(AttribClass::Point, "Cd")
            .map_err(SopError::Core)?;

        let num_pts = out.num_points();
        for i in 0..num_pts {
            out.set_attrib(&handle, i, params.color)
                .map_err(SopError::Core)?;
        }

        Ok(out)
    }
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
    fn color_sets_cd() {
        let box_geo = make_box();
        let params = ColorParams { color: [1.0, 0.0, 0.0] };
        let result = box_geo.apply(&ColorSop, &params).unwrap();

        let handle = result
            .find_attrib::<[f32; 3]>(AttribClass::Point, "Cd")
            .unwrap();

        for i in 0..result.num_points() {
            let cd = result.get_attrib(&handle, i).unwrap();
            assert_eq!(cd, [1.0, 0.0, 0.0], "point {i} should be red");
        }
    }

    #[test]
    fn color_overwrites() {
        let box_geo = make_box();

        // Apply blue first, then green
        let params_blue = ColorParams { color: [0.0, 0.0, 1.0] };
        let params_green = ColorParams { color: [0.0, 1.0, 0.0] };

        let result = box_geo
            .apply(&ColorSop, &params_blue)
            .unwrap()
            .apply(&ColorSop, &params_green)
            .unwrap();

        let handle = result
            .find_attrib::<[f32; 3]>(AttribClass::Point, "Cd")
            .unwrap();

        for i in 0..result.num_points() {
            let cd = result.get_attrib(&handle, i).unwrap();
            assert_eq!(cd, [0.0, 1.0, 0.0], "point {i} should be green after overwrite");
        }
    }

    #[test]
    fn color_point_handle_type() {
        // Verify the attribute is on points
        let box_geo = make_box();
        let params = ColorParams::default();
        let result = box_geo.apply(&ColorSop, &params).unwrap();

        // find_attrib on Point class must succeed
        assert!(result
            .find_attrib::<[f32; 3]>(AttribClass::Point, "Cd")
            .is_ok());
    }
}
