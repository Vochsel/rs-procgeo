use serde::{Deserialize, Serialize};

use procgeo_core::Geometry;

use crate::{Sop, SopError};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NullParams;

pub struct NullSop;

impl Sop for NullSop {
    type Params = NullParams;

    fn name(&self) -> &'static str {
        "null"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 1)
    }

    fn execute(&self, inputs: &[&Geometry], _params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;
        Ok(inputs[0].clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creation::box_sop::{BoxSop, BoxParams};
    use crate::{GeometryExt, generate};
    use glam::Vec3;

    fn make_box() -> Geometry {
        generate(&BoxSop, &BoxParams::default()).unwrap()
    }

    #[test]
    fn null_passthrough() {
        let box_geo = make_box();
        let num_points = box_geo.num_points();
        let num_prims = box_geo.num_prims();

        // Collect positions before
        let positions_before: Vec<Vec3> = (0..num_points)
            .map(|i| box_geo.point_pos(procgeo_core::PointHandle::from_index(i)))
            .collect();

        let result = box_geo.apply(&NullSop, &NullParams).unwrap();

        assert_eq!(result.num_points(), num_points);
        assert_eq!(result.num_prims(), num_prims);

        for i in 0..num_points {
            let pos = result.point_pos(procgeo_core::PointHandle::from_index(i));
            assert_eq!(pos, positions_before[i]);
        }
    }
}
