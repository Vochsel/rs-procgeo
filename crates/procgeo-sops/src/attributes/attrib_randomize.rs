use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;

use procgeo_core::{AttribClass, AttribDefault, AttribType, Geometry, TypeQualifier};

use crate::{Sop, SopError};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub enum RandomDistribution {
    #[default]
    Uniform,
    Gaussian,
    TwoValues,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub enum RandomOperation {
    #[default]
    Set,
    Add,
    Multiply,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttribRandomizeParams {
    pub attrib_name: String,
    pub class: AttribClass,
    pub attrib_type: AttribType,
    pub distribution: RandomDistribution,
    pub operation: RandomOperation,
    pub seed: u64,
    pub min_value: f32,
    pub max_value: f32,
    pub mean: f32,
    pub stddev: f32,
    pub value_a: f32,
    pub value_b: f32,
    pub probability: f32,
    pub dimensions: u32,
    pub global_scale: f32,
}

impl Default for AttribRandomizeParams {
    fn default() -> Self {
        AttribRandomizeParams {
            attrib_name: "randomize".to_string(),
            class: AttribClass::Point,
            attrib_type: AttribType::Float,
            distribution: RandomDistribution::Uniform,
            operation: RandomOperation::Set,
            seed: 0,
            min_value: 0.0,
            max_value: 1.0,
            mean: 0.0,
            stddev: 1.0,
            value_a: 0.0,
            value_b: 1.0,
            probability: 0.5,
            dimensions: 1,
            global_scale: 1.0,
        }
    }
}

pub struct AttribRandomizeSop;

fn element_count(geo: &Geometry, class: AttribClass) -> usize {
    match class {
        AttribClass::Point => geo.num_points(),
        AttribClass::Vertex => geo.num_vertices(),
        AttribClass::Primitive => geo.num_prims(),
        AttribClass::Detail => 1,
    }
}

/// Generate a single random float based on the distribution params.
fn gen_float(rng: &mut StdRng, params: &AttribRandomizeParams) -> f32 {
    match params.distribution {
        RandomDistribution::Uniform => {
            let lo = params.min_value.min(params.max_value);
            let hi = params.min_value.max(params.max_value);
            if (hi - lo).abs() < 1e-10 {
                lo
            } else {
                rng.random_range(lo..hi)
            }
        }
        RandomDistribution::Gaussian => {
            // Box-Muller transform
            let u1: f32 = rng.random_range(1e-10f32..1.0f32);
            let u2: f32 = rng.random_range(0.0f32..1.0f32);
            let z = (-2.0 * u1.ln()).sqrt() * (2.0 * PI * u2).cos();
            params.mean + params.stddev * z
        }
        RandomDistribution::TwoValues => {
            let r: f32 = rng.random_range(0.0f32..1.0f32);
            if r < params.probability {
                params.value_a
            } else {
                params.value_b
            }
        }
    }
}

fn apply_op(current: f32, generated: f32, op: RandomOperation, scale: f32) -> f32 {
    let scaled = generated * scale;
    match op {
        RandomOperation::Set => scaled,
        RandomOperation::Add => current + scaled,
        RandomOperation::Multiply => current * scaled,
    }
}

impl Sop for AttribRandomizeSop {
    type Params = AttribRandomizeParams;

    fn name(&self) -> &'static str {
        "attrib_randomize"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 1)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;
        let mut out = inputs[0].clone();

        let count = element_count(&out, params.class);
        if count == 0 {
            return Ok(out);
        }

        let mut rng = StdRng::seed_from_u64(params.seed);

        if params.dimensions <= 1 {
            // Scalar float attribute
            let _ = out.add_attrib(
                params.class,
                &params.attrib_name,
                AttribDefault::Float(0.0),
                TypeQualifier::None,
            );
            let handle = out
                .find_attrib::<f32>(params.class, &params.attrib_name)
                .map_err(SopError::Core)?;

            let new_vals: Vec<f32> = (0..count)
                .map(|i| {
                    let current = out.get_attrib(&handle, i).unwrap_or(0.0);
                    let generated = gen_float(&mut rng, params);
                    apply_op(current, generated, params.operation, params.global_scale)
                })
                .collect();

            for (i, v) in new_vals.into_iter().enumerate() {
                out.set_attrib(&handle, i, v)?;
            }
        } else {
            // Vector3 attribute (dimensions >= 3)
            let _ = out.add_attrib(
                params.class,
                &params.attrib_name,
                AttribDefault::Vector3([0.0; 3]),
                TypeQualifier::None,
            );
            let handle = out
                .find_attrib::<[f32; 3]>(params.class, &params.attrib_name)
                .map_err(SopError::Core)?;

            let new_vals: Vec<[f32; 3]> = (0..count)
                .map(|i| {
                    let current = out.get_attrib(&handle, i).unwrap_or([0.0; 3]);
                    let gx = gen_float(&mut rng, params);
                    let gy = gen_float(&mut rng, params);
                    let gz = gen_float(&mut rng, params);
                    [
                        apply_op(current[0], gx, params.operation, params.global_scale),
                        apply_op(current[1], gy, params.operation, params.global_scale),
                        apply_op(current[2], gz, params.operation, params.global_scale),
                    ]
                })
                .collect();

            for (i, v) in new_vals.into_iter().enumerate() {
                out.set_attrib(&handle, i, v)?;
            }
        }

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creation::box_sop::{BoxParams, BoxSop};
    use crate::{GeometryExt, generate};
    use approx::assert_relative_eq;

    fn make_box() -> Geometry {
        generate(&BoxSop, &BoxParams::default()).unwrap()
    }

    #[test]
    fn randomize_uniform_float() {
        let geo = make_box();
        let sop = AttribRandomizeSop;
        let params = AttribRandomizeParams {
            attrib_name: "rand_val".to_string(),
            min_value: 0.0,
            max_value: 1.0,
            distribution: RandomDistribution::Uniform,
            ..Default::default()
        };
        let result = geo.apply(&sop, &params).unwrap();
        let handle = result
            .find_attrib::<f32>(AttribClass::Point, "rand_val")
            .unwrap();
        for i in 0..result.num_points() {
            let v = result.get_attrib(&handle, i).unwrap();
            assert!(v >= 0.0 && v <= 1.0, "point {i} value {v} out of [0,1]");
        }
    }

    #[test]
    fn randomize_deterministic() {
        let geo = make_box();
        let sop = AttribRandomizeSop;
        let params = AttribRandomizeParams {
            attrib_name: "rand_val".to_string(),
            seed: 42,
            ..Default::default()
        };
        let result1 = geo.clone().apply(&sop, &params).unwrap();
        let result2 = geo.apply(&sop, &params).unwrap();

        let h1 = result1
            .find_attrib::<f32>(AttribClass::Point, "rand_val")
            .unwrap();
        let h2 = result2
            .find_attrib::<f32>(AttribClass::Point, "rand_val")
            .unwrap();

        for i in 0..result1.num_points() {
            let v1 = result1.get_attrib(&h1, i).unwrap();
            let v2 = result2.get_attrib(&h2, i).unwrap();
            assert_relative_eq!(v1, v2, epsilon = 1e-10);
        }
    }

    #[test]
    fn randomize_gaussian() {
        // With 100 points we should get a reasonable cluster around mean
        let mut geo = Geometry::new();
        for i in 0..100 {
            geo.add_point(glam::Vec3::new(i as f32, 0.0, 0.0));
        }

        let sop = AttribRandomizeSop;
        let params = AttribRandomizeParams {
            attrib_name: "noise".to_string(),
            distribution: RandomDistribution::Gaussian,
            mean: 5.0,
            stddev: 1.0,
            seed: 99,
            ..Default::default()
        };
        let result = geo.apply(&sop, &params).unwrap();
        let handle = result
            .find_attrib::<f32>(AttribClass::Point, "noise")
            .unwrap();

        let values: Vec<f32> = (0..result.num_points())
            .map(|i| result.get_attrib(&handle, i).unwrap())
            .collect();
        let avg = values.iter().sum::<f32>() / values.len() as f32;

        // Average should be within ~1.0 of mean=5.0
        assert!((avg - 5.0).abs() < 2.0, "expected avg near 5.0, got {avg}");
    }

    #[test]
    fn randomize_vector3() {
        let geo = make_box();
        let sop = AttribRandomizeSop;
        let params = AttribRandomizeParams {
            attrib_name: "vel".to_string(),
            dimensions: 3,
            min_value: -1.0,
            max_value: 1.0,
            ..Default::default()
        };
        let result = geo.apply(&sop, &params).unwrap();
        let handle = result
            .find_attrib::<[f32; 3]>(AttribClass::Point, "vel")
            .unwrap();

        for i in 0..result.num_points() {
            let v = result.get_attrib(&handle, i).unwrap();
            for c in 0..3 {
                assert!(
                    v[c] >= -1.0 && v[c] <= 1.0,
                    "point {i} component {c} value {} out of range",
                    v[c]
                );
            }
        }
    }
}
