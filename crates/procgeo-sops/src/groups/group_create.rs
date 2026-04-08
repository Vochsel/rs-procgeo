use serde::{Deserialize, Serialize};

use glam::Vec3;

use procgeo_core::{Geometry, PrimHandle};

use crate::{Sop, SopError};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub enum GroupType {
    #[default]
    Points,
    Primitives,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub enum GroupCreateMode {
    #[default]
    Range,
    BoundingBox,
    Normal,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroupCreateParams {
    pub name: String,
    pub group_type: GroupType,
    pub mode: GroupCreateMode,
    /// Inclusive start of range (element index)
    pub range_start: usize,
    /// Exclusive end of range (element index); use usize::MAX for "all"
    pub range_end: usize,
    pub bbox_min: Vec3,
    pub bbox_max: Vec3,
    /// Direction to compare face normals against (for Normal mode)
    pub normal_direction: Vec3,
    /// Maximum angle (degrees) between face normal and direction
    pub normal_angle: f32,
}

impl Default for GroupCreateParams {
    fn default() -> Self {
        GroupCreateParams {
            name: "group1".to_string(),
            group_type: GroupType::Points,
            mode: GroupCreateMode::Range,
            range_start: 0,
            range_end: usize::MAX,
            bbox_min: Vec3::NEG_INFINITY,
            bbox_max: Vec3::INFINITY,
            normal_direction: Vec3::Y,
            normal_angle: 45.0,
        }
    }
}

pub struct GroupCreateSop;

/// Compute a face normal via Newell's method.
fn face_normal(positions: &[Vec3]) -> Vec3 {
    let n = positions.len();
    let mut nx = 0.0_f32;
    let mut ny = 0.0_f32;
    let mut nz = 0.0_f32;
    for i in 0..n {
        let cur = positions[i];
        let next = positions[(i + 1) % n];
        nx += (cur.y - next.y) * (cur.z + next.z);
        ny += (cur.z - next.z) * (cur.x + next.x);
        nz += (cur.x - next.x) * (cur.y + next.y);
    }
    Vec3::new(nx, ny, nz).normalize_or_zero()
}

impl Sop for GroupCreateSop {
    type Params = GroupCreateParams;

    fn name(&self) -> &'static str {
        "group_create"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 1)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;
        let mut geo = inputs[0].clone();

        match params.group_type {
            GroupType::Points => {
                geo.create_point_group(&params.name);
                let count = geo.num_points();
                let end = params.range_end.min(count);

                match params.mode {
                    GroupCreateMode::Range => {
                        for i in params.range_start..end {
                            geo.groups_mut()
                                .point_group_mut(&params.name)
                                .unwrap()
                                .add(i);
                        }
                    }
                    GroupCreateMode::BoundingBox => {
                        for i in 0..count {
                            let ph = procgeo_core::PointHandle::from_index(i);
                            let pos = geo.point_pos(ph);
                            if pos.x >= params.bbox_min.x
                                && pos.x <= params.bbox_max.x
                                && pos.y >= params.bbox_min.y
                                && pos.y <= params.bbox_max.y
                                && pos.z >= params.bbox_min.z
                                && pos.z <= params.bbox_max.z
                            {
                                geo.groups_mut()
                                    .point_group_mut(&params.name)
                                    .unwrap()
                                    .add(i);
                            }
                        }
                    }
                    GroupCreateMode::Normal => {
                        // Normal mode is prim-oriented; for points, fall back to range
                        for i in params.range_start..end {
                            geo.groups_mut()
                                .point_group_mut(&params.name)
                                .unwrap()
                                .add(i);
                        }
                    }
                }
            }

            GroupType::Primitives => {
                geo.create_prim_group(&params.name);
                let count = geo.num_prims();
                let end = params.range_end.min(count);

                match params.mode {
                    GroupCreateMode::Range => {
                        for i in params.range_start..end {
                            geo.groups_mut()
                                .prim_group_mut(&params.name)
                                .unwrap()
                                .add(i);
                        }
                    }
                    GroupCreateMode::BoundingBox => {
                        // Add prim if its centroid is inside the bbox
                        for prim_idx in 0..count {
                            let prim_handle = PrimHandle::from_index(prim_idx);
                            let pts = geo.prim_points(prim_handle);
                            if pts.is_empty() {
                                continue;
                            }
                            let centroid: Vec3 =
                                pts.iter().map(|&ph| geo.point_pos(ph)).sum::<Vec3>()
                                    / pts.len() as f32;
                            if centroid.x >= params.bbox_min.x
                                && centroid.x <= params.bbox_max.x
                                && centroid.y >= params.bbox_min.y
                                && centroid.y <= params.bbox_max.y
                                && centroid.z >= params.bbox_min.z
                                && centroid.z <= params.bbox_max.z
                            {
                                geo.groups_mut()
                                    .prim_group_mut(&params.name)
                                    .unwrap()
                                    .add(prim_idx);
                            }
                        }
                    }
                    GroupCreateMode::Normal => {
                        let cos_threshold = params.normal_angle.to_radians().cos();
                        let dir = params.normal_direction.normalize_or_zero();

                        for prim_idx in 0..count {
                            let prim_handle = PrimHandle::from_index(prim_idx);
                            let pt_handles = geo.prim_points(prim_handle);
                            if pt_handles.len() < 3 {
                                continue;
                            }
                            let positions: Vec<Vec3> =
                                pt_handles.iter().map(|&ph| geo.point_pos(ph)).collect();
                            let n = face_normal(&positions);
                            let cos_angle = n.dot(dir);
                            if cos_angle >= cos_threshold {
                                geo.groups_mut()
                                    .prim_group_mut(&params.name)
                                    .unwrap()
                                    .add(prim_idx);
                            }
                        }
                    }
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
    use crate::{GeometryExt, generate};

    #[test]
    fn group_by_range() {
        // Box has 8 points; add first 4 to group
        let box_geo = generate(&BoxSop, &BoxParams::default()).unwrap();
        assert_eq!(box_geo.num_points(), 8);

        let sop = GroupCreateSop;
        let params = GroupCreateParams {
            name: "first_half".to_string(),
            group_type: GroupType::Points,
            mode: GroupCreateMode::Range,
            range_start: 0,
            range_end: 4,
            ..Default::default()
        };
        let result = box_geo.apply(&sop, &params).unwrap();

        let g = result.groups().point_group("first_half").unwrap();
        assert_eq!(g.count(), 4);
        assert!(g.contains(0));
        assert!(g.contains(3));
        assert!(!g.contains(4));
        assert!(!g.contains(7));
    }

    #[test]
    fn group_by_bbox() {
        // Default box spans [-0.5, 0.5] on all axes.
        // Points 0-3 are at y = -0.5, points 4-7 are at y = 0.5.
        // Use bbox y: [-1, 0] to capture lower half.
        let box_geo = generate(&BoxSop, &BoxParams::default()).unwrap();

        let sop = GroupCreateSop;
        let params = GroupCreateParams {
            name: "lower".to_string(),
            group_type: GroupType::Points,
            mode: GroupCreateMode::BoundingBox,
            bbox_min: Vec3::new(-1.0, -1.0, -1.0),
            bbox_max: Vec3::new(1.0, 0.0, 1.0),
            ..Default::default()
        };
        let result = box_geo.apply(&sop, &params).unwrap();

        let g = result.groups().point_group("lower").unwrap();
        // Lower 4 points (y = -0.5) should be in group
        assert_eq!(g.count(), 4);
        for i in 0..4 {
            assert!(g.contains(i), "expected point {i} in group");
        }
        for i in 4..8 {
            assert!(!g.contains(i), "did not expect point {i} in group");
        }
    }

    #[test]
    fn group_by_normal() {
        // Default box. Top face (y=+0.5) has normal pointing up (+Y).
        // With direction Y and angle 45 degrees, only the top face prim should be in group.
        let box_geo = generate(&BoxSop, &BoxParams::default()).unwrap();

        let sop = GroupCreateSop;
        let params = GroupCreateParams {
            name: "top".to_string(),
            group_type: GroupType::Primitives,
            mode: GroupCreateMode::Normal,
            normal_direction: Vec3::Y,
            normal_angle: 45.0,
            ..Default::default()
        };
        let result = box_geo.apply(&sop, &params).unwrap();

        let g = result.groups().prim_group("top").unwrap();
        // Exactly 1 face should face upward
        assert_eq!(g.count(), 1);
    }
}
