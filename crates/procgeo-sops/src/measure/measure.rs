use serde::{Deserialize, Serialize};

use glam::Vec3;

use procgeo_core::{AttribClass, AttribDefault, Geometry, PrimHandle, TypeQualifier};

use crate::{Sop, SopError};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub enum MeasureType {
    #[default]
    Area,
    Perimeter,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MeasureParams {
    pub measure_type: MeasureType,
    /// Attribute name; if empty, defaults to "area" or "perimeter" based on type.
    pub attrib_name: String,
}

impl MeasureParams {
    fn resolved_attrib_name(&self) -> &str {
        if self.attrib_name.is_empty() {
            match self.measure_type {
                MeasureType::Area => "area",
                MeasureType::Perimeter => "perimeter",
            }
        } else {
            &self.attrib_name
        }
    }
}

pub struct MeasureSop;

/// Compute polygon area via triangle fan from first vertex.
fn poly_area(positions: &[Vec3]) -> f32 {
    let n = positions.len();
    if n < 3 {
        return 0.0;
    }
    let v0 = positions[0];
    let mut area = 0.0_f32;
    for i in 1..n - 1 {
        let v1 = positions[i];
        let v2 = positions[i + 1];
        let cross = (v1 - v0).cross(v2 - v0);
        area += cross.length() * 0.5;
    }
    area
}

/// Compute polygon perimeter as sum of edge lengths.
fn poly_perimeter(positions: &[Vec3]) -> f32 {
    let n = positions.len();
    if n < 2 {
        return 0.0;
    }
    let mut perimeter = 0.0_f32;
    for i in 0..n {
        let next = (i + 1) % n;
        perimeter += (positions[next] - positions[i]).length();
    }
    perimeter
}

impl Sop for MeasureSop {
    type Params = MeasureParams;

    fn name(&self) -> &'static str {
        "measure"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 1)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;
        let geo = inputs[0];

        let attrib_name = params.resolved_attrib_name();
        let mut out = geo.clone();

        // Add prim attribute
        out.add_attrib(
            AttribClass::Primitive,
            attrib_name,
            AttribDefault::Float(0.0),
            TypeQualifier::None,
        )?;

        let handle = out.find_attrib::<f32>(AttribClass::Primitive, attrib_name)?;
        let mut total = 0.0_f32;

        for prim_idx in 0..out.num_prims() {
            let ph = PrimHandle::from_index(prim_idx);
            let pt_handles = out.prim_points(ph);
            let positions: Vec<Vec3> = pt_handles.iter().map(|&h| out.point_pos(h)).collect();

            let value = match params.measure_type {
                MeasureType::Area => poly_area(&positions),
                MeasureType::Perimeter => poly_perimeter(&positions),
            };

            out.set_attrib(&handle, prim_idx, value)?;
            total += value;
        }

        // For Area, also write a detail total attribute
        if matches!(params.measure_type, MeasureType::Area) {
            let total_name = format!("{}_total", attrib_name);
            out.add_attrib(
                AttribClass::Detail,
                &total_name,
                AttribDefault::Float(0.0),
                TypeQualifier::None,
            )?;
            let total_handle = out.find_attrib::<f32>(AttribClass::Detail, &total_name)?;
            out.set_attrib(&total_handle, 0, total)?;
        }

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creation::box_sop::{BoxSop, BoxParams};
    use crate::{GeometryExt, generate};
    use approx::assert_relative_eq;
    use glam::Vec3;

    fn make_box() -> Geometry {
        generate(&BoxSop, &BoxParams::default()).unwrap()
    }

    fn make_unit_quad() -> Geometry {
        // 1×1 quad in XZ plane
        let mut geo = Geometry::new();
        let p0 = geo.add_point(Vec3::new(-0.5, 0.0, -0.5));
        let p1 = geo.add_point(Vec3::new( 0.5, 0.0, -0.5));
        let p2 = geo.add_point(Vec3::new( 0.5, 0.0,  0.5));
        let p3 = geo.add_point(Vec3::new(-0.5, 0.0,  0.5));
        geo.add_face(&[p0, p1, p2, p3]);
        geo
    }

    #[test]
    fn measure_area_unit_quad() {
        let quad = make_unit_quad();
        let params = MeasureParams {
            measure_type: MeasureType::Area,
            attrib_name: String::new(),
        };
        let result = quad.apply(&MeasureSop, &params).unwrap();

        let handle = result.find_attrib::<f32>(AttribClass::Primitive, "area").unwrap();
        let area = result.get_attrib(&handle, 0).unwrap();
        assert_relative_eq!(area, 1.0, epsilon = 1e-5);
    }

    #[test]
    fn measure_area_box() {
        // Default box → 6 faces, each 1.0×1.0 = 1.0, total = 6.0
        let box_geo = make_box();
        let params = MeasureParams::default(); // Area
        let result = box_geo.apply(&MeasureSop, &params).unwrap();

        let handle = result.find_attrib::<f32>(AttribClass::Primitive, "area").unwrap();
        let mut total = 0.0;
        for i in 0..result.num_prims() {
            let area = result.get_attrib(&handle, i).unwrap();
            assert_relative_eq!(area, 1.0, epsilon = 1e-5);
            total += area;
        }
        assert_relative_eq!(total, 6.0, epsilon = 1e-5);

        // Check detail total attribute
        let total_handle = result.find_attrib::<f32>(AttribClass::Detail, "area_total").unwrap();
        let detail_total = result.get_attrib(&total_handle, 0).unwrap();
        assert_relative_eq!(detail_total, 6.0, epsilon = 1e-5);
    }

    #[test]
    fn measure_perimeter_quad() {
        // 1×1 quad → perimeter = 4×1.0 = 4.0
        let quad = make_unit_quad();
        let params = MeasureParams {
            measure_type: MeasureType::Perimeter,
            attrib_name: String::new(),
        };
        let result = quad.apply(&MeasureSop, &params).unwrap();

        let handle = result.find_attrib::<f32>(AttribClass::Primitive, "perimeter").unwrap();
        let perimeter = result.get_attrib(&handle, 0).unwrap();
        assert_relative_eq!(perimeter, 4.0, epsilon = 1e-5);
    }
}
