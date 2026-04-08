use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use procgeo_core::{AttribClass, AttribType, Geometry, PrimHandle};

use crate::{Sop, SopError};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttribFillParams {
    pub attrib_name: String,
    pub attrib_type: AttribType,
    /// Point group name — these points are fixed source values.
    pub boundary_group: String,
    pub iterations: u32,
    pub step_size: f32,
}

impl Default for AttribFillParams {
    fn default() -> Self {
        AttribFillParams {
            attrib_name: "attrib".to_string(),
            attrib_type: AttribType::Float,
            boundary_group: String::new(),
            iterations: 10,
            step_size: 0.5,
        }
    }
}

pub struct AttribFillSop;

fn build_adjacency(geo: &Geometry) -> HashMap<usize, HashSet<usize>> {
    let mut adjacency: HashMap<usize, HashSet<usize>> = HashMap::new();
    for i in 0..geo.num_points() {
        adjacency.entry(i).or_default();
    }
    for prim_idx in 0..geo.num_prims() {
        let ph = PrimHandle::from_index(prim_idx);
        let pts = geo.prim_points(ph);
        let n = pts.len();
        if n < 2 {
            continue;
        }
        for i in 0..n {
            let a = pts[i].index();
            let b = pts[(i + 1) % n].index();
            adjacency.entry(a).or_default().insert(b);
            adjacency.entry(b).or_default().insert(a);
        }
    }
    adjacency
}

impl Sop for AttribFillSop {
    type Params = AttribFillParams;

    fn name(&self) -> &'static str {
        "attrib_fill"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 1)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;
        let mut out = inputs[0].clone();

        if params.iterations == 0 {
            return Ok(out);
        }

        let num_pts = out.num_points();

        // Determine boundary set
        let boundary: HashSet<usize> = if params.boundary_group.is_empty() {
            HashSet::new()
        } else {
            match out.groups().point_group(&params.boundary_group) {
                Some(g) => g.iter_set().collect(),
                None => HashSet::new(),
            }
        };

        let adjacency = build_adjacency(&out);

        match params.attrib_type {
            AttribType::Float => {
                let handle = out
                    .find_attrib::<f32>(AttribClass::Point, &params.attrib_name)
                    .map_err(SopError::Core)?;

                for _ in 0..params.iterations {
                    let current: Vec<f32> = (0..num_pts)
                        .map(|i| out.get_attrib(&handle, i).unwrap_or(0.0))
                        .collect();

                    let new_vals: Vec<f32> = (0..num_pts)
                        .map(|i| {
                            // Boundary points stay fixed
                            if boundary.contains(&i) {
                                return current[i];
                            }
                            let neighbors = &adjacency[&i];
                            if neighbors.is_empty() {
                                return current[i];
                            }
                            let avg = neighbors.iter().map(|&j| current[j]).sum::<f32>()
                                / neighbors.len() as f32;
                            current[i] + (avg - current[i]) * params.step_size
                        })
                        .collect();

                    for (i, v) in new_vals.into_iter().enumerate() {
                        // Don't update boundary points
                        if !boundary.contains(&i) {
                            out.set_attrib(&handle, i, v)?;
                        }
                    }
                }
            }

            AttribType::Vector3 => {
                let handle = out
                    .find_attrib::<[f32; 3]>(AttribClass::Point, &params.attrib_name)
                    .map_err(SopError::Core)?;

                for _ in 0..params.iterations {
                    let current: Vec<[f32; 3]> = (0..num_pts)
                        .map(|i| out.get_attrib(&handle, i).unwrap_or([0.0; 3]))
                        .collect();

                    let new_vals: Vec<[f32; 3]> = (0..num_pts)
                        .map(|i| {
                            if boundary.contains(&i) {
                                return current[i];
                            }
                            let neighbors = &adjacency[&i];
                            if neighbors.is_empty() {
                                return current[i];
                            }
                            let n = neighbors.len() as f32;
                            let mut avg = [0.0f32; 3];
                            for &j in neighbors.iter() {
                                avg[0] += current[j][0];
                                avg[1] += current[j][1];
                                avg[2] += current[j][2];
                            }
                            avg[0] /= n;
                            avg[1] /= n;
                            avg[2] /= n;
                            [
                                current[i][0] + (avg[0] - current[i][0]) * params.step_size,
                                current[i][1] + (avg[1] - current[i][1]) * params.step_size,
                                current[i][2] + (avg[2] - current[i][2]) * params.step_size,
                            ]
                        })
                        .collect();

                    for (i, v) in new_vals.into_iter().enumerate() {
                        if !boundary.contains(&i) {
                            out.set_attrib(&handle, i, v)?;
                        }
                    }
                }
            }

            other => {
                return Err(SopError::InvalidParam(format!(
                    "AttribFill: unsupported attrib_type {:?}",
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
    use crate::attributes::create::{AttribCreateParams, AttribCreateSop};
    use crate::creation::grid::{GridOrientation, GridParams, GridSop};
    use crate::{GeometryExt, generate};
    use glam::Vec3;

    fn make_grid_with_corner_boundary() -> Geometry {
        let geo = generate(
            &GridSop,
            &GridParams {
                size: [4.0, 4.0],
                rows: 5,
                cols: 5,
                center: Vec3::ZERO,
                orientation: GridOrientation::XZ,
            },
        )
        .unwrap();

        // Create "heat" attribute, all 0.0
        let sop = AttribCreateSop;
        let params = AttribCreateParams {
            name: "heat".to_string(),
            class: AttribClass::Point,
            attrib_type: AttribType::Float,
            value_float: 0.0,
            ..Default::default()
        };
        let mut result = geo.apply(&sop, &params).unwrap();

        // Create boundary group with point 0 (corner)
        result.create_point_group("boundary");
        result
            .groups_mut()
            .point_group_mut("boundary")
            .unwrap()
            .add(0);

        // Set point 0 to 1.0
        let handle = result
            .find_attrib::<f32>(AttribClass::Point, "heat")
            .unwrap();
        result.set_attrib(&handle, 0, 1.0).unwrap();

        result
    }

    #[test]
    fn fill_diffusion() {
        let geo = make_grid_with_corner_boundary();

        let sop = AttribFillSop;
        let params = AttribFillParams {
            attrib_name: "heat".to_string(),
            attrib_type: AttribType::Float,
            boundary_group: "boundary".to_string(),
            iterations: 10,
            step_size: 0.5,
        };

        let result = geo.apply(&sop, &params).unwrap();
        let handle = result
            .find_attrib::<f32>(AttribClass::Point, "heat")
            .unwrap();

        // Some points near the corner should have values > 0
        let positive_count = (0..result.num_points())
            .filter(|&i| result.get_attrib(&handle, i).unwrap() > 1e-4)
            .count();
        assert!(
            positive_count > 1,
            "heat should diffuse from corner: only {} positive points",
            positive_count
        );
    }

    #[test]
    fn fill_preserves_boundary() {
        let geo = make_grid_with_corner_boundary();

        let sop = AttribFillSop;
        let params = AttribFillParams {
            attrib_name: "heat".to_string(),
            attrib_type: AttribType::Float,
            boundary_group: "boundary".to_string(),
            iterations: 10,
            step_size: 0.5,
        };

        let result = geo.apply(&sop, &params).unwrap();
        let handle = result
            .find_attrib::<f32>(AttribClass::Point, "heat")
            .unwrap();

        // Point 0 (boundary) must remain at 1.0
        let boundary_val = result.get_attrib(&handle, 0).unwrap();
        approx::assert_relative_eq!(boundary_val, 1.0, epsilon = 1e-6);
    }
}
