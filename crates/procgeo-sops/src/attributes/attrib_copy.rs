use serde::{Deserialize, Serialize};

use procgeo_core::{AttribClass, AttribDefault, AttribType, Geometry, TypeQualifier};

use crate::{Sop, SopError};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttribCopyParams {
    pub attrib_name: String,
    pub class: AttribClass,
    pub attrib_type: AttribType,
    /// If non-empty, write the copied values under this name instead.
    pub new_name: String,
}

impl Default for AttribCopyParams {
    fn default() -> Self {
        AttribCopyParams {
            attrib_name: "attrib".to_string(),
            class: AttribClass::Point,
            attrib_type: AttribType::Float,
            new_name: String::new(),
        }
    }
}

pub struct AttribCopySop;

fn element_count(geo: &Geometry, class: AttribClass) -> usize {
    match class {
        AttribClass::Point => geo.num_points(),
        AttribClass::Vertex => geo.num_vertices(),
        AttribClass::Primitive => geo.num_prims(),
        AttribClass::Detail => 1,
    }
}

impl Sop for AttribCopySop {
    type Params = AttribCopyParams;

    fn name(&self) -> &'static str {
        "attrib_copy"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 2)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        if inputs.is_empty() {
            return Err(SopError::WrongInputCount {
                expected_min: 1,
                expected_max: 2,
                got: 0,
            });
        }

        let dst_geo = inputs[0];
        let src_geo: &Geometry = if inputs.len() >= 2 {
            inputs[1]
        } else {
            dst_geo
        };

        let mut out = dst_geo.clone();

        let dst_name = if params.new_name.is_empty() {
            params.attrib_name.clone()
        } else {
            params.new_name.clone()
        };

        let dst_count = element_count(&out, params.class);
        let src_count = element_count(src_geo, params.class);

        if src_count == 0 || dst_count == 0 {
            return Ok(out);
        }

        match params.attrib_type {
            AttribType::Float => {
                let src_handle = src_geo
                    .find_attrib::<f32>(params.class, &params.attrib_name)
                    .map_err(SopError::Core)?;
                let src_values: Vec<f32> = (0..src_count)
                    .map(|i| src_geo.get_attrib(&src_handle, i).unwrap_or(0.0))
                    .collect();

                let _ = out.add_attrib(
                    params.class,
                    &dst_name,
                    AttribDefault::Float(0.0),
                    TypeQualifier::None,
                );
                let dst_handle = out
                    .find_attrib::<f32>(params.class, &dst_name)
                    .map_err(SopError::Core)?;

                for di in 0..dst_count {
                    let si = di % src_count;
                    out.set_attrib(&dst_handle, di, src_values[si])?;
                }
            }

            AttribType::Vector3 => {
                let src_handle = src_geo
                    .find_attrib::<[f32; 3]>(params.class, &params.attrib_name)
                    .map_err(SopError::Core)?;
                let src_values: Vec<[f32; 3]> = (0..src_count)
                    .map(|i| src_geo.get_attrib(&src_handle, i).unwrap_or([0.0; 3]))
                    .collect();

                let _ = out.add_attrib(
                    params.class,
                    &dst_name,
                    AttribDefault::Vector3([0.0; 3]),
                    TypeQualifier::None,
                );
                let dst_handle = out
                    .find_attrib::<[f32; 3]>(params.class, &dst_name)
                    .map_err(SopError::Core)?;

                for di in 0..dst_count {
                    let si = di % src_count;
                    out.set_attrib(&dst_handle, di, src_values[si])?;
                }
            }

            AttribType::Int => {
                let src_handle = src_geo
                    .find_attrib::<i32>(params.class, &params.attrib_name)
                    .map_err(SopError::Core)?;
                let src_values: Vec<i32> = (0..src_count)
                    .map(|i| src_geo.get_attrib(&src_handle, i).unwrap_or(0))
                    .collect();

                let _ = out.add_attrib(
                    params.class,
                    &dst_name,
                    AttribDefault::Int(0),
                    TypeQualifier::None,
                );
                let dst_handle = out
                    .find_attrib::<i32>(params.class, &dst_name)
                    .map_err(SopError::Core)?;

                for di in 0..dst_count {
                    let si = di % src_count;
                    out.set_attrib(&dst_handle, di, src_values[si])?;
                }
            }

            other => {
                return Err(SopError::InvalidParam(format!(
                    "AttribCopy: unsupported attrib_type {:?}",
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
    use approx::assert_relative_eq;
    use glam::Vec3;

    fn make_grid(rows: u32, cols: u32) -> Geometry {
        generate(
            &GridSop,
            &GridParams {
                size: [2.0, 2.0],
                rows,
                cols,
                center: Vec3::ZERO,
                orientation: GridOrientation::XZ,
            },
        )
        .unwrap()
    }

    fn add_vec3_attrib(geo: Geometry, name: &str, values: &[[f32; 3]]) -> Geometry {
        let sop = AttribCreateSop;
        let params = AttribCreateParams {
            name: name.to_string(),
            class: AttribClass::Point,
            attrib_type: AttribType::Vector3,
            value_vector3: [0.0; 3],
            ..Default::default()
        };
        let mut result = geo.apply(&sop, &params).unwrap();
        let handle = result
            .find_attrib::<[f32; 3]>(AttribClass::Point, name)
            .unwrap();
        for (i, &v) in values.iter().enumerate() {
            result.set_attrib(&handle, i, v).unwrap();
        }
        result
    }

    #[test]
    fn copy_by_index() {
        // Source: 3-point grid with "Cd" attribute: [1,0,0], [0,1,0], [0,0,1]
        // Actually make a simple 2x2 grid (4 pts) and we'll set 3 of them,
        // but to keep it simple just create 3 custom points.
        let mut src = Geometry::new();
        src.add_point(Vec3::ZERO);
        src.add_point(Vec3::X);
        src.add_point(Vec3::Y);
        src = add_vec3_attrib(
            src,
            "Cd",
            &[[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        );

        // Destination: 6-point grid
        let dst = make_grid(3, 3);
        let num_dst = dst.num_points();

        let copy_sop = AttribCopySop;
        let params = AttribCopyParams {
            attrib_name: "Cd".to_string(),
            class: AttribClass::Point,
            attrib_type: AttribType::Vector3,
            new_name: String::new(),
        };

        let result = copy_sop.execute(&[&dst, &src], &params).unwrap();
        let handle = result
            .find_attrib::<[f32; 3]>(AttribClass::Point, "Cd")
            .unwrap();

        let src_colors: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        for di in 0..num_dst {
            let expected = src_colors[di % 3];
            let got = result.get_attrib(&handle, di).unwrap();
            assert_relative_eq!(got[0], expected[0], epsilon = 1e-6);
            assert_relative_eq!(got[1], expected[1], epsilon = 1e-6);
            assert_relative_eq!(got[2], expected[2], epsilon = 1e-6);
        }
    }

    #[test]
    fn copy_with_rename() {
        // Source with "Cd", copy to destination as "color"
        let mut src = Geometry::new();
        src.add_point(Vec3::ZERO);
        src.add_point(Vec3::X);
        src = add_vec3_attrib(src, "Cd", &[[0.5, 0.5, 0.5], [1.0, 1.0, 1.0]]);

        let dst = make_grid(2, 2);

        let copy_sop = AttribCopySop;
        let params = AttribCopyParams {
            attrib_name: "Cd".to_string(),
            class: AttribClass::Point,
            attrib_type: AttribType::Vector3,
            new_name: "color".to_string(),
        };

        let result = copy_sop.execute(&[&dst, &src], &params).unwrap();

        // "Cd" should NOT be on destination (was created from src which wasn't cloned onto dst)
        assert!(
            result
                .find_attrib::<[f32; 3]>(AttribClass::Point, "Cd")
                .is_err()
        );

        // "color" SHOULD exist
        let handle = result
            .find_attrib::<[f32; 3]>(AttribClass::Point, "color")
            .unwrap();

        // 4 dst points cycling over 2 src values
        let expected: [[f32; 3]; 4] = [
            [0.5, 0.5, 0.5],
            [1.0, 1.0, 1.0],
            [0.5, 0.5, 0.5],
            [1.0, 1.0, 1.0],
        ];
        for (i, exp) in expected.iter().enumerate() {
            let got = result.get_attrib(&handle, i).unwrap();
            assert_relative_eq!(got[0], exp[0], epsilon = 1e-6);
            assert_relative_eq!(got[1], exp[1], epsilon = 1e-6);
            assert_relative_eq!(got[2], exp[2], epsilon = 1e-6);
        }
    }
}
