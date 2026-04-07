use serde::{Deserialize, Serialize};

use procgeo_core::{AttribClass, AttribDefault, AttribType, Geometry, PointHandle, TypeQualifier};

use crate::{Sop, SopError};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttribTransferParams {
    pub attrib_name: String,
    pub class: AttribClass,
    pub attrib_type: AttribType,
    pub max_samples: u32,
    pub distance_threshold: f32,
}

impl Default for AttribTransferParams {
    fn default() -> Self {
        AttribTransferParams {
            attrib_name: "attrib".to_string(),
            class: AttribClass::Point,
            attrib_type: AttribType::Float,
            max_samples: 1,
            distance_threshold: f32::MAX,
        }
    }
}

pub struct AttribTransferSop;

impl Sop for AttribTransferSop {
    type Params = AttribTransferParams;

    fn name(&self) -> &'static str {
        "attrib_transfer"
    }

    fn input_count(&self) -> (usize, usize) {
        (2, 2)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;
        let dst_geo = inputs[0];
        let src_geo = inputs[1];

        let mut out = dst_geo.clone();

        let num_dst = out.num_points();
        let num_src = src_geo.num_points();

        if num_src == 0 || num_dst == 0 {
            return Ok(out);
        }

        match params.attrib_type {
            AttribType::Float => {
                // Ensure destination attribute exists
                let _ = out.add_attrib(
                    params.class,
                    &params.attrib_name,
                    AttribDefault::Float(0.0),
                    TypeQualifier::None,
                );

                // Collect source values
                let src_handle = src_geo
                    .find_attrib::<f32>(params.class, &params.attrib_name)
                    .map_err(SopError::Core)?;
                let src_values: Vec<f32> = (0..num_src)
                    .map(|i| src_geo.get_attrib(&src_handle, i).unwrap_or(0.0))
                    .collect();

                // Collect source positions
                let src_positions: Vec<glam::Vec3> = (0..num_src)
                    .map(|i| src_geo.point_pos(PointHandle::from_index(i)))
                    .collect();

                let dst_handle = out
                    .find_attrib::<f32>(params.class, &params.attrib_name)
                    .map_err(SopError::Core)?;

                let new_values: Vec<f32> = (0..num_dst)
                    .map(|di| {
                        let dst_pos = out.point_pos(PointHandle::from_index(di));
                        transfer_float(
                            dst_pos,
                            &src_positions,
                            &src_values,
                            params.max_samples,
                            params.distance_threshold,
                        )
                    })
                    .collect();

                for (i, v) in new_values.into_iter().enumerate() {
                    out.set_attrib(&dst_handle, i, v)?;
                }
            }

            AttribType::Vector3 => {
                let _ = out.add_attrib(
                    params.class,
                    &params.attrib_name,
                    AttribDefault::Vector3([0.0; 3]),
                    TypeQualifier::None,
                );

                let src_handle = src_geo
                    .find_attrib::<[f32; 3]>(params.class, &params.attrib_name)
                    .map_err(SopError::Core)?;
                let src_values: Vec<[f32; 3]> = (0..num_src)
                    .map(|i| src_geo.get_attrib(&src_handle, i).unwrap_or([0.0; 3]))
                    .collect();

                let src_positions: Vec<glam::Vec3> = (0..num_src)
                    .map(|i| src_geo.point_pos(PointHandle::from_index(i)))
                    .collect();

                let dst_handle = out
                    .find_attrib::<[f32; 3]>(params.class, &params.attrib_name)
                    .map_err(SopError::Core)?;

                let new_values: Vec<[f32; 3]> = (0..num_dst)
                    .map(|di| {
                        let dst_pos = out.point_pos(PointHandle::from_index(di));
                        transfer_vec3(
                            dst_pos,
                            &src_positions,
                            &src_values,
                            params.max_samples,
                            params.distance_threshold,
                        )
                    })
                    .collect();

                for (i, v) in new_values.into_iter().enumerate() {
                    out.set_attrib(&dst_handle, i, v)?;
                }
            }

            other => {
                return Err(SopError::InvalidParam(format!(
                    "AttribTransfer: unsupported attrib_type {:?}",
                    other
                )));
            }
        }

        Ok(out)
    }
}

/// Find up to `max_samples` nearest source points within `distance_threshold`,
/// then return the inverse-distance weighted average float value.
fn transfer_float(
    dst_pos: glam::Vec3,
    src_positions: &[glam::Vec3],
    src_values: &[f32],
    max_samples: u32,
    distance_threshold: f32,
) -> f32 {
    let max_samples = max_samples as usize;

    // Compute squared distances
    let mut dist_sq_vals: Vec<(f32, f32)> = src_positions
        .iter()
        .zip(src_values.iter())
        .map(|(&pos, &val)| (dst_pos.distance_squared(pos), val))
        .filter(|(d2, _)| *d2 <= distance_threshold * distance_threshold)
        .collect();

    if dist_sq_vals.is_empty() {
        return 0.0;
    }

    dist_sq_vals.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    dist_sq_vals.truncate(max_samples);

    if max_samples == 1 {
        return dist_sq_vals[0].1;
    }

    // Inverse distance weighted average
    let weights: Vec<f32> = dist_sq_vals
        .iter()
        .map(|(d2, _)| {
            if *d2 < 1e-10 {
                f32::MAX
            } else {
                1.0 / d2.sqrt()
            }
        })
        .collect();

    let total_weight: f32 = weights.iter().sum();
    if total_weight == 0.0 {
        return 0.0;
    }

    dist_sq_vals
        .iter()
        .zip(weights.iter())
        .map(|((_, val), w)| val * w)
        .sum::<f32>()
        / total_weight
}

fn transfer_vec3(
    dst_pos: glam::Vec3,
    src_positions: &[glam::Vec3],
    src_values: &[[f32; 3]],
    max_samples: u32,
    distance_threshold: f32,
) -> [f32; 3] {
    let max_samples = max_samples as usize;

    let mut dist_sq_vals: Vec<(f32, [f32; 3])> = src_positions
        .iter()
        .zip(src_values.iter())
        .map(|(&pos, &val)| (dst_pos.distance_squared(pos), val))
        .filter(|(d2, _)| *d2 <= distance_threshold * distance_threshold)
        .collect();

    if dist_sq_vals.is_empty() {
        return [0.0; 3];
    }

    dist_sq_vals.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    dist_sq_vals.truncate(max_samples);

    if max_samples == 1 {
        return dist_sq_vals[0].1;
    }

    let weights: Vec<f32> = dist_sq_vals
        .iter()
        .map(|(d2, _)| {
            if *d2 < 1e-10 {
                f32::MAX
            } else {
                1.0 / d2.sqrt()
            }
        })
        .collect();

    let total_weight: f32 = weights.iter().sum();
    if total_weight == 0.0 {
        return [0.0; 3];
    }

    let mut result = [0.0f32; 3];
    for ((_, val), w) in dist_sq_vals.iter().zip(weights.iter()) {
        result[0] += val[0] * w;
        result[1] += val[1] * w;
        result[2] += val[2] * w;
    }
    result[0] /= total_weight;
    result[1] /= total_weight;
    result[2] /= total_weight;
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attributes::create::{AttribCreateSop, AttribCreateParams};
    use crate::creation::grid::{GridSop, GridParams, GridOrientation};
    use crate::{GeometryExt, generate};
    use approx::assert_relative_eq;
    use glam::Vec3;

    fn make_grid(rows: u32, cols: u32) -> Geometry {
        generate(&GridSop, &GridParams {
            size: [2.0, 2.0],
            rows,
            cols,
            center: Vec3::ZERO,
            orientation: GridOrientation::XZ,
        })
        .unwrap()
    }

    fn add_float_attrib(geo: Geometry, name: &str, values: &[f32]) -> Geometry {
        let sop = AttribCreateSop;
        let params = AttribCreateParams {
            name: name.to_string(),
            class: AttribClass::Point,
            attrib_type: AttribType::Float,
            value_float: 0.0,
            ..Default::default()
        };
        let mut result = geo.apply(&sop, &params).unwrap();
        let handle = result.find_attrib::<f32>(AttribClass::Point, name).unwrap();
        for (i, &v) in values.iter().enumerate() {
            result.set_attrib(&handle, i, v).unwrap();
        }
        result
    }

    #[test]
    fn transfer_nearest() {
        // Source grid: 2x2 (4 points) with "temperature" attribute
        let src = make_grid(2, 2);
        let num_src = src.num_points();
        let src_temps: Vec<f32> = (0..num_src).map(|i| i as f32 * 10.0).collect();
        let src = add_float_attrib(src, "temperature", &src_temps);

        // Destination grid: same 2x2 layout
        let dst = make_grid(2, 2);

        let transfer_sop = AttribTransferSop;
        let params = AttribTransferParams {
            attrib_name: "temperature".to_string(),
            class: AttribClass::Point,
            attrib_type: AttribType::Float,
            max_samples: 1,
            distance_threshold: f32::MAX,
        };

        let result = transfer_sop.execute(&[&dst, &src], &params).unwrap();
        let handle = result
            .find_attrib::<f32>(AttribClass::Point, "temperature")
            .unwrap();

        // Since grids are identical, nearest point is itself → same values
        for i in 0..num_src {
            let v = result.get_attrib(&handle, i).unwrap();
            assert_relative_eq!(v, src_temps[i], epsilon = 1e-4);
        }
    }

    #[test]
    fn transfer_threshold() {
        // Source: single point at origin with temperature=100
        let mut src = Geometry::new();
        src.add_point(Vec3::ZERO);
        let _ = src.add_attrib(
            AttribClass::Point,
            "temperature",
            AttribDefault::Float(0.0),
            TypeQualifier::None,
        );
        let src_h = src.find_attrib::<f32>(AttribClass::Point, "temperature").unwrap();
        src.set_attrib(&src_h, 0, 100.0).unwrap();

        // Destination: two points, one close (dist=0.5) and one far (dist=10.0)
        let mut dst = Geometry::new();
        dst.add_point(Vec3::new(0.5, 0.0, 0.0));
        dst.add_point(Vec3::new(10.0, 0.0, 0.0));

        let transfer_sop = AttribTransferSop;
        let params = AttribTransferParams {
            attrib_name: "temperature".to_string(),
            class: AttribClass::Point,
            attrib_type: AttribType::Float,
            max_samples: 1,
            distance_threshold: 1.0, // only source within 1.0 distance
        };

        let result = transfer_sop.execute(&[&dst, &src], &params).unwrap();
        let handle = result
            .find_attrib::<f32>(AttribClass::Point, "temperature")
            .unwrap();

        // Point 0 (close) should get value 100
        let v0 = result.get_attrib(&handle, 0).unwrap();
        assert_relative_eq!(v0, 100.0, epsilon = 1e-4);

        // Point 1 (far) beyond threshold — default 0.0
        let v1 = result.get_attrib(&handle, 1).unwrap();
        assert_relative_eq!(v1, 0.0, epsilon = 1e-4);
    }
}
