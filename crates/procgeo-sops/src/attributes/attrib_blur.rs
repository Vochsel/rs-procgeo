use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use procgeo_core::{AttribClass, AttribType, Geometry, PrimHandle};

use crate::{Sop, SopError};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttribBlurParams {
    pub attrib_name: String,
    pub attrib_type: AttribType,
    pub iterations: u32,
    /// Blending factor: 0 = no change, 1 = full Laplacian average.
    pub step_size: f32,
}

impl Default for AttribBlurParams {
    fn default() -> Self {
        AttribBlurParams {
            attrib_name: "attrib".to_string(),
            attrib_type: AttribType::Float,
            iterations: 1,
            step_size: 1.0,
        }
    }
}

pub struct AttribBlurSop;

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

impl Sop for AttribBlurSop {
    type Params = AttribBlurParams;

    fn name(&self) -> &'static str {
        "attrib_blur"
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

        let adjacency = build_adjacency(&out);
        let num_pts = out.num_points();

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
                        out.set_attrib(&handle, i, v)?;
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
                        out.set_attrib(&handle, i, v)?;
                    }
                }
            }

            other => {
                return Err(SopError::InvalidParam(format!(
                    "AttribBlur: unsupported attrib_type {:?}",
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

    fn make_grid_with_hot_point() -> (Geometry, usize) {
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

        let sop = AttribCreateSop;
        let params = AttribCreateParams {
            name: "heat".to_string(),
            class: AttribClass::Point,
            attrib_type: AttribType::Float,
            value_float: 0.0,
            ..Default::default()
        };
        let mut result = geo.apply(&sop, &params).unwrap();

        // Set the center point (index 12 in 5x5 grid) to 1.0
        let hot_index = 12;
        let handle = result
            .find_attrib::<f32>(AttribClass::Point, "heat")
            .unwrap();
        result.set_attrib(&handle, hot_index, 1.0).unwrap();

        (result, hot_index)
    }

    #[test]
    fn blur_float() {
        let (geo, hot_idx) = make_grid_with_hot_point();

        let sop = AttribBlurSop;
        let params = AttribBlurParams {
            attrib_name: "heat".to_string(),
            attrib_type: AttribType::Float,
            iterations: 1,
            step_size: 1.0,
        };

        let result = geo.apply(&sop, &params).unwrap();
        let handle = result
            .find_attrib::<f32>(AttribClass::Point, "heat")
            .unwrap();

        // After one blur, the hot point should have decreased
        let hot_val = result.get_attrib(&handle, hot_idx).unwrap();
        assert!(hot_val < 1.0, "hot point should spread: got {hot_val}");

        // At least some neighboring points should be > 0
        let nonzero_count = (0..result.num_points())
            .filter(|&i| result.get_attrib(&handle, i).unwrap() > 0.0)
            .count();
        assert!(nonzero_count > 1, "heat should spread to neighbors");
    }

    #[test]
    fn blur_iterations() {
        let (geo, hot_idx) = make_grid_with_hot_point();

        let sop = AttribBlurSop;

        let result1 = geo
            .clone()
            .apply(
                &sop,
                &AttribBlurParams {
                    attrib_name: "heat".to_string(),
                    attrib_type: AttribType::Float,
                    iterations: 1,
                    step_size: 1.0,
                },
            )
            .unwrap();
        let result5 = geo
            .apply(
                &sop,
                &AttribBlurParams {
                    attrib_name: "heat".to_string(),
                    attrib_type: AttribType::Float,
                    iterations: 5,
                    step_size: 1.0,
                },
            )
            .unwrap();

        let h1 = result1
            .find_attrib::<f32>(AttribClass::Point, "heat")
            .unwrap();
        let h5 = result5
            .find_attrib::<f32>(AttribClass::Point, "heat")
            .unwrap();

        // After more iterations the hot point decreases more (spreads more)
        let val1 = result1.get_attrib(&h1, hot_idx).unwrap();
        let val5 = result5.get_attrib(&h5, hot_idx).unwrap();
        assert!(
            val5 <= val1,
            "more iterations should spread more: val1={val1}, val5={val5}"
        );

        // More iterations should produce more non-zero points
        let nonzero1 = (0..result1.num_points())
            .filter(|&i| result1.get_attrib(&h1, i).unwrap() > 1e-6)
            .count();
        let nonzero5 = (0..result5.num_points())
            .filter(|&i| result5.get_attrib(&h5, i).unwrap() > 1e-6)
            .count();
        assert!(
            nonzero5 >= nonzero1,
            "more iterations should spread heat further: nonzero1={nonzero1}, nonzero5={nonzero5}"
        );
    }

    #[test]
    fn blur_preserves_sum() {
        // On a uniform mesh with step_size=1, the sum of values is approximately preserved
        // (Laplacian smoothing conserves the total)
        let (geo, _) = make_grid_with_hot_point();

        let handle_before = geo.find_attrib::<f32>(AttribClass::Point, "heat").unwrap();
        let sum_before: f32 = (0..geo.num_points())
            .map(|i| geo.get_attrib(&handle_before, i).unwrap())
            .sum();

        let sop = AttribBlurSop;
        let result = geo
            .apply(
                &sop,
                &AttribBlurParams {
                    attrib_name: "heat".to_string(),
                    attrib_type: AttribType::Float,
                    iterations: 3,
                    step_size: 1.0,
                },
            )
            .unwrap();

        let handle_after = result
            .find_attrib::<f32>(AttribClass::Point, "heat")
            .unwrap();
        let sum_after: f32 = (0..result.num_points())
            .map(|i| result.get_attrib(&handle_after, i).unwrap())
            .sum();

        // Allow 20% tolerance (boundary effects on non-uniform valence mesh)
        assert!(
            (sum_after - sum_before).abs() < sum_before * 0.20 + 0.01,
            "sum not preserved: before={sum_before}, after={sum_after}"
        );
    }
}
