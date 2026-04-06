use serde::{Deserialize, Serialize};

use procgeo_core::Geometry;

use crate::{Sop, SopError};

/// Which kind of element the blast group refers to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlastEntity {
    #[default]
    Primitives,
    Points,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlastParams {
    /// Name of the group to blast.
    pub group_name: String,
    /// Whether the group references points or primitives.
    pub entity: BlastEntity,
    /// If false (default): remove group members. If true: keep only group members.
    pub negate: bool,
}

impl Default for BlastParams {
    fn default() -> Self {
        BlastParams {
            group_name: String::new(),
            entity: BlastEntity::Primitives,
            negate: false,
        }
    }
}

pub struct BlastSop;

impl Sop for BlastSop {
    type Params = BlastParams;

    fn name(&self) -> &'static str {
        "blast"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 1)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;
        let geo = inputs[0];

        match params.entity {
            BlastEntity::Primitives => {
                let group = geo
                    .groups()
                    .prim_group(&params.group_name)
                    .ok_or_else(|| {
                        SopError::Other(format!(
                            "prim group '{}' not found",
                            params.group_name
                        ))
                    })?;

                let keep: Vec<bool> = (0..geo.num_prims())
                    .map(|i| {
                        let in_group = group.contains(i);
                        if params.negate { in_group } else { !in_group }
                    })
                    .collect();

                Ok(geo.rebuild_keeping_prims(&keep))
            }
            BlastEntity::Points => {
                let group = geo
                    .groups()
                    .point_group(&params.group_name)
                    .ok_or_else(|| {
                        SopError::Other(format!(
                            "point group '{}' not found",
                            params.group_name
                        ))
                    })?;

                let keep: Vec<bool> = (0..geo.num_points())
                    .map(|i| {
                        let in_group = group.contains(i);
                        if params.negate { in_group } else { !in_group }
                    })
                    .collect();

                Ok(geo.rebuild_keeping_points(&keep))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creation::box_sop::{BoxSop, BoxParams};
    use crate::groups::group_create::{GroupCreateSop, GroupCreateParams, GroupType, GroupCreateMode};
    use crate::{GeometryExt, generate};

    fn make_box() -> Geometry {
        generate(&BoxSop, &BoxParams::default()).unwrap()
    }

    #[test]
    fn blast_by_prim_group() {
        // Create box (6 faces), group first 2 prims, blast them → 4 remain
        let mut geo = make_box();
        geo.create_prim_group("to_blast");
        geo.groups_mut().prim_group_mut("to_blast").unwrap().add(0);
        geo.groups_mut().prim_group_mut("to_blast").unwrap().add(1);

        let params = BlastParams {
            group_name: "to_blast".to_string(),
            entity: BlastEntity::Primitives,
            negate: false,
        };
        let result = geo.apply(&BlastSop, &params).unwrap();
        assert_eq!(result.num_prims(), 4);
    }

    #[test]
    fn blast_negate() {
        // Same setup, but negate=true → keep only the 2 group members
        let mut geo = make_box();
        geo.create_prim_group("to_keep");
        geo.groups_mut().prim_group_mut("to_keep").unwrap().add(0);
        geo.groups_mut().prim_group_mut("to_keep").unwrap().add(1);

        let params = BlastParams {
            group_name: "to_keep".to_string(),
            entity: BlastEntity::Primitives,
            negate: true,
        };
        let result = geo.apply(&BlastSop, &params).unwrap();
        assert_eq!(result.num_prims(), 2);
    }

    #[test]
    fn blast_points() {
        // Group the 4 bottom points of a box (indices 0-3) and blast them.
        // The 4 top points remain; any face referencing removed points is dropped.
        let params_gc = GroupCreateParams {
            name: "bottom".to_string(),
            group_type: GroupType::Points,
            mode: GroupCreateMode::Range,
            range_start: 0,
            range_end: 4,
            ..Default::default()
        };
        let geo = make_box()
            .apply(&GroupCreateSop, &params_gc)
            .unwrap();

        let params_blast = BlastParams {
            group_name: "bottom".to_string(),
            entity: BlastEntity::Points,
            negate: false,
        };
        let result = geo.apply(&BlastSop, &params_blast).unwrap();

        // 4 top points remain; only the top face (referencing only top points) is kept
        assert_eq!(result.num_points(), 4);
        assert!(result.num_prims() >= 1, "at least top face should remain");
    }
}
