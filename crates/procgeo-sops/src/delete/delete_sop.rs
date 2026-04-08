use serde::{Deserialize, Serialize};

use procgeo_core::Geometry;

use crate::{Sop, SopError};

/// Which kind of element to delete.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeleteEntity {
    #[default]
    Primitives,
    Points,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeleteParams {
    pub entity: DeleteEntity,
    /// Inclusive start of the range to delete.
    pub range_start: usize,
    /// Exclusive end of the range to delete.
    pub range_end: usize,
}

impl Default for DeleteParams {
    fn default() -> Self {
        DeleteParams {
            entity: DeleteEntity::Primitives,
            range_start: 0,
            range_end: 0,
        }
    }
}

pub struct DeleteSop;

impl Sop for DeleteSop {
    type Params = DeleteParams;

    fn name(&self) -> &'static str {
        "delete"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 1)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;
        let geo = inputs[0];

        match params.entity {
            DeleteEntity::Primitives => {
                let keep: Vec<bool> = (0..geo.num_prims())
                    .map(|i| i < params.range_start || i >= params.range_end)
                    .collect();
                Ok(geo.rebuild_keeping_prims(&keep))
            }
            DeleteEntity::Points => {
                let keep: Vec<bool> = (0..geo.num_points())
                    .map(|i| i < params.range_start || i >= params.range_end)
                    .collect();
                Ok(geo.rebuild_keeping_points(&keep))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creation::box_sop::{BoxParams, BoxSop};
    use crate::creation::grid::{GridParams, GridSop};
    use crate::{GeometryExt, generate};

    fn make_box() -> Geometry {
        generate(&BoxSop, &BoxParams::default()).unwrap()
    }

    #[test]
    fn delete_prim_range() {
        // Box has 6 prims; delete [0, 3) → 3 prims remain
        let params = DeleteParams {
            entity: DeleteEntity::Primitives,
            range_start: 0,
            range_end: 3,
        };
        let result = make_box().apply(&DeleteSop, &params).unwrap();
        assert_eq!(result.num_prims(), 3);
    }

    #[test]
    fn delete_points_range() {
        // 3x3 grid has 9 points; delete [0, 3) → 6 points remain
        // Prims referencing deleted points are also removed.
        let grid = generate(
            &GridSop,
            &GridParams {
                rows: 3,
                cols: 3,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(grid.num_points(), 9);

        let params = DeleteParams {
            entity: DeleteEntity::Points,
            range_start: 0,
            range_end: 3,
        };
        let result = grid.apply(&DeleteSop, &params).unwrap();
        assert_eq!(result.num_points(), 6);
        // Prims that referenced removed points should be gone
        // The 3x3 grid has 4 prims; top row prims reference points 0-5
        // so some will be removed.
        assert!(result.num_prims() < 4);
    }

    #[test]
    fn delete_empty_range() {
        // Deleting empty range keeps everything
        let params = DeleteParams {
            entity: DeleteEntity::Primitives,
            range_start: 2,
            range_end: 2,
        };
        let result = make_box().apply(&DeleteSop, &params).unwrap();
        assert_eq!(result.num_prims(), 6);
    }
}
