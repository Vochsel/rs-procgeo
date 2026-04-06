use serde::{Deserialize, Serialize};

use procgeo_core::{ElementGroup, Geometry};

use crate::{Sop, SopError};

use super::group_create::GroupType;

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub enum GroupBooleanOp {
    #[default]
    Union,
    Intersect,
    Subtract,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroupCombineParams {
    pub name_a: String,
    pub name_b: String,
    pub result: String,
    pub operation: GroupBooleanOp,
    pub group_type: GroupType,
}

impl Default for GroupCombineParams {
    fn default() -> Self {
        GroupCombineParams {
            name_a: "group_a".to_string(),
            name_b: "group_b".to_string(),
            result: "group_result".to_string(),
            operation: GroupBooleanOp::Union,
            group_type: GroupType::Points,
        }
    }
}

pub struct GroupCombineSop;

impl Sop for GroupCombineSop {
    type Params = GroupCombineParams;

    fn name(&self) -> &'static str {
        "group_combine"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 1)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;
        let mut geo = inputs[0].clone();

        match params.group_type {
            GroupType::Points => {
                let a = geo
                    .groups()
                    .point_group(&params.name_a)
                    .ok_or_else(|| SopError::InvalidParam(format!("group '{}' not found", params.name_a)))?
                    .clone();
                let b = geo
                    .groups()
                    .point_group(&params.name_b)
                    .ok_or_else(|| SopError::InvalidParam(format!("group '{}' not found", params.name_b)))?
                    .clone();

                let mut result_group = apply_op(&a, &b, params.operation, geo.num_points());
                // Ensure result group is sized correctly
                result_group.resize(geo.num_points());

                geo.create_point_group(&params.result);
                *geo.groups_mut().point_group_mut(&params.result).unwrap() = result_group;
            }

            GroupType::Primitives => {
                let a = geo
                    .groups()
                    .prim_group(&params.name_a)
                    .ok_or_else(|| SopError::InvalidParam(format!("group '{}' not found", params.name_a)))?
                    .clone();
                let b = geo
                    .groups()
                    .prim_group(&params.name_b)
                    .ok_or_else(|| SopError::InvalidParam(format!("group '{}' not found", params.name_b)))?
                    .clone();

                let mut result_group = apply_op(&a, &b, params.operation, geo.num_prims());
                result_group.resize(geo.num_prims());

                geo.create_prim_group(&params.result);
                *geo.groups_mut().prim_group_mut(&params.result).unwrap() = result_group;
            }
        }

        Ok(geo)
    }
}

fn apply_op(a: &ElementGroup, b: &ElementGroup, op: GroupBooleanOp, size: usize) -> ElementGroup {
    let mut result = a.clone();
    result.resize(size);
    match op {
        GroupBooleanOp::Union => result.union(b),
        GroupBooleanOp::Intersect => result.intersect(b),
        GroupBooleanOp::Subtract => result.subtract(b),
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::group_create::{GroupCreateSop, GroupCreateParams, GroupCreateMode};
    use crate::creation::box_sop::{BoxSop, BoxParams};
    use crate::{GeometryExt, generate};

    fn make_two_point_groups() -> procgeo_core::Geometry {
        // Box: 8 points
        let box_geo = generate(&BoxSop, &BoxParams::default()).unwrap();

        // group_a: points 0..5 (indices 0,1,2,3,4)
        let geo = {
            let sop = GroupCreateSop;
            let params = GroupCreateParams {
                name: "group_a".to_string(),
                group_type: GroupType::Points,
                mode: GroupCreateMode::Range,
                range_start: 0,
                range_end: 5,
                ..Default::default()
            };
            box_geo.apply(&sop, &params).unwrap()
        };

        // group_b: points 3..8 (indices 3,4,5,6,7)
        let sop = GroupCreateSop;
        let params = GroupCreateParams {
            name: "group_b".to_string(),
            group_type: GroupType::Points,
            mode: GroupCreateMode::Range,
            range_start: 3,
            range_end: 8,
            ..Default::default()
        };
        geo.apply(&sop, &params).unwrap()
    }

    #[test]
    fn combine_union() {
        let geo = make_two_point_groups();

        let sop = GroupCombineSop;
        let params = GroupCombineParams {
            name_a: "group_a".to_string(),
            name_b: "group_b".to_string(),
            result: "combined".to_string(),
            operation: GroupBooleanOp::Union,
            group_type: GroupType::Points,
        };
        let result = geo.apply(&sop, &params).unwrap();

        let g = result.groups().point_group("combined").unwrap();
        // union of {0,1,2,3,4} and {3,4,5,6,7} = {0,1,2,3,4,5,6,7}
        assert_eq!(g.count(), 8);
    }

    #[test]
    fn combine_intersect() {
        let geo = make_two_point_groups();

        let sop = GroupCombineSop;
        let params = GroupCombineParams {
            name_a: "group_a".to_string(),
            name_b: "group_b".to_string(),
            result: "combined".to_string(),
            operation: GroupBooleanOp::Intersect,
            group_type: GroupType::Points,
        };
        let result = geo.apply(&sop, &params).unwrap();

        let g = result.groups().point_group("combined").unwrap();
        // intersection of {0,1,2,3,4} and {3,4,5,6,7} = {3,4}
        assert_eq!(g.count(), 2);
        assert!(g.contains(3));
        assert!(g.contains(4));
        assert!(!g.contains(0));
        assert!(!g.contains(5));
    }

    #[test]
    fn combine_subtract() {
        let geo = make_two_point_groups();

        let sop = GroupCombineSop;
        let params = GroupCombineParams {
            name_a: "group_a".to_string(),
            name_b: "group_b".to_string(),
            result: "combined".to_string(),
            operation: GroupBooleanOp::Subtract,
            group_type: GroupType::Points,
        };
        let result = geo.apply(&sop, &params).unwrap();

        let g = result.groups().point_group("combined").unwrap();
        // {0,1,2,3,4} - {3,4,5,6,7} = {0,1,2}
        assert_eq!(g.count(), 3);
        assert!(g.contains(0));
        assert!(g.contains(1));
        assert!(g.contains(2));
        assert!(!g.contains(3));
        assert!(!g.contains(4));
    }
}
