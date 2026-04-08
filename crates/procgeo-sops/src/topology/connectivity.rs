use serde::{Deserialize, Serialize};

use procgeo_core::{AttribClass, AttribDefault, Geometry, PrimHandle, TypeQualifier};

use crate::{Sop, SopError};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConnectivityParams {
    /// Name of the integer primitive attribute to write component IDs into.
    pub attrib_name: String,
}

impl Default for ConnectivityParams {
    fn default() -> Self {
        ConnectivityParams {
            attrib_name: "class".to_string(),
        }
    }
}

pub struct ConnectivitySop;

impl Sop for ConnectivitySop {
    type Params = ConnectivityParams;

    fn name(&self) -> &'static str {
        "connectivity"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 1)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;
        let geo = inputs[0];

        let num_prims = geo.num_prims();
        let num_pts = geo.num_points();

        // Build point→prims lookup: for each point, which prims reference it
        let mut point_to_prims: Vec<Vec<usize>> = vec![Vec::new(); num_pts];
        for prim_idx in 0..num_prims {
            let ph = PrimHandle::from_index(prim_idx);
            for pt in geo.prim_points(ph) {
                point_to_prims[pt.index()].push(prim_idx);
            }
        }

        // Build prim adjacency: two prims are adjacent if they share at least one point
        // We'll use BFS flood fill with a visited array
        let mut component_id: Vec<i32> = vec![-1; num_prims];
        let mut next_component = 0i32;

        for start in 0..num_prims {
            if component_id[start] >= 0 {
                continue;
            }

            // BFS from `start`
            let mut queue = std::collections::VecDeque::new();
            queue.push_back(start);
            component_id[start] = next_component;

            while let Some(prim_idx) = queue.pop_front() {
                let ph = PrimHandle::from_index(prim_idx);
                for pt in geo.prim_points(ph) {
                    for &neighbor in &point_to_prims[pt.index()] {
                        if component_id[neighbor] < 0 {
                            component_id[neighbor] = next_component;
                            queue.push_back(neighbor);
                        }
                    }
                }
            }

            next_component += 1;
        }

        // Clone geometry and add component attribute
        let mut out = geo.clone();
        out.add_attrib(
            AttribClass::Primitive,
            &params.attrib_name,
            AttribDefault::Int(0),
            TypeQualifier::None,
        )?;

        let handle = out.find_attrib::<i32>(AttribClass::Primitive, &params.attrib_name)?;
        for (prim_idx, &comp) in component_id.iter().enumerate() {
            out.set_attrib(&handle, prim_idx, comp.max(0))?;
        }

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creation::box_sop::{BoxParams, BoxSop};
    use crate::merge::{MergeParams, MergeSop};
    use crate::transform::{TransformParams, TransformSop};
    use crate::{GeometryExt, generate};
    use glam::Vec3;

    fn make_box() -> Geometry {
        generate(&BoxSop, &BoxParams::default()).unwrap()
    }

    #[test]
    fn connectivity_one_mesh() {
        // A single box → all 6 prims in component 0
        let box_geo = make_box();
        let params = ConnectivityParams::default();
        let result = box_geo.apply(&ConnectivitySop, &params).unwrap();

        assert_eq!(result.num_prims(), 6);
        let h = result
            .find_attrib::<i32>(AttribClass::Primitive, "class")
            .unwrap();
        for i in 0..result.num_prims() {
            assert_eq!(
                result.get_attrib(&h, i).unwrap(),
                0,
                "all prims should be class 0"
            );
        }
    }

    #[test]
    fn connectivity_two_boxes() {
        // Two non-overlapping boxes → one gets class 0, the other class 1
        let box1 = make_box();
        let box2 = make_box()
            .apply(
                &TransformSop,
                &TransformParams {
                    translate: Vec3::new(10.0, 0.0, 0.0),
                    ..Default::default()
                },
            )
            .unwrap();

        let merged = MergeSop.execute(&[&box1, &box2], &MergeParams).unwrap();
        let params = ConnectivityParams::default();
        let result = merged.apply(&ConnectivitySop, &params).unwrap();

        assert_eq!(result.num_prims(), 12);
        let h = result
            .find_attrib::<i32>(AttribClass::Primitive, "class")
            .unwrap();

        // First 6 prims should be class 0, next 6 should be class 1
        for i in 0..6 {
            assert_eq!(
                result.get_attrib(&h, i).unwrap(),
                0,
                "prim {i} should be class 0"
            );
        }
        for i in 6..12 {
            assert_eq!(
                result.get_attrib(&h, i).unwrap(),
                1,
                "prim {i} should be class 1"
            );
        }
    }
}
