use std::collections::HashMap;

use glam::Vec3;
use serde::{Deserialize, Serialize};

use procgeo_core::Geometry;

use crate::{Sop, SopError};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IcosphereParams {
    pub radius: f32,
    pub center: Vec3,
    pub subdivisions: u32,
}

impl Default for IcosphereParams {
    fn default() -> Self {
        Self {
            radius: 0.5,
            center: Vec3::ZERO,
            subdivisions: 2,
        }
    }
}

pub struct IcosphereSop;

fn base_icosahedron() -> (Vec<Vec3>, Vec<[usize; 3]>) {
    let t = (1.0 + 5.0_f32.sqrt()) * 0.5;
    let vertices = vec![
        Vec3::new(-1.0, t, 0.0).normalize(),
        Vec3::new(1.0, t, 0.0).normalize(),
        Vec3::new(-1.0, -t, 0.0).normalize(),
        Vec3::new(1.0, -t, 0.0).normalize(),
        Vec3::new(0.0, -1.0, t).normalize(),
        Vec3::new(0.0, 1.0, t).normalize(),
        Vec3::new(0.0, -1.0, -t).normalize(),
        Vec3::new(0.0, 1.0, -t).normalize(),
        Vec3::new(t, 0.0, -1.0).normalize(),
        Vec3::new(t, 0.0, 1.0).normalize(),
        Vec3::new(-t, 0.0, -1.0).normalize(),
        Vec3::new(-t, 0.0, 1.0).normalize(),
    ];

    let faces = vec![
        [0, 11, 5],
        [0, 5, 1],
        [0, 1, 7],
        [0, 7, 10],
        [0, 10, 11],
        [1, 5, 9],
        [5, 11, 4],
        [11, 10, 2],
        [10, 7, 6],
        [7, 1, 8],
        [3, 9, 4],
        [3, 4, 2],
        [3, 2, 6],
        [3, 6, 8],
        [3, 8, 9],
        [4, 9, 5],
        [2, 4, 11],
        [6, 2, 10],
        [8, 6, 7],
        [9, 8, 1],
    ];

    (vertices, faces)
}

fn midpoint(
    a: usize,
    b: usize,
    vertices: &mut Vec<Vec3>,
    cache: &mut HashMap<(usize, usize), usize>,
) -> usize {
    let key = if a < b { (a, b) } else { (b, a) };
    if let Some(&idx) = cache.get(&key) {
        return idx;
    }

    let pos = (vertices[a] + vertices[b]).normalize();
    let idx = vertices.len();
    vertices.push(pos);
    cache.insert(key, idx);
    idx
}

impl Sop for IcosphereSop {
    type Params = IcosphereParams;

    fn name(&self) -> &'static str {
        "icosphere"
    }

    fn input_count(&self) -> (usize, usize) {
        (0, 0)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;

        if params.radius <= 0.0 {
            return Err(SopError::InvalidParam(format!(
                "radius must be > 0, got {}",
                params.radius
            )));
        }

        let (mut vertices, mut faces) = base_icosahedron();
        for _ in 0..params.subdivisions {
            let mut cache = HashMap::with_capacity(faces.len() * 3);
            let mut next_faces = Vec::with_capacity(faces.len() * 4);

            for [a, b, c] in faces {
                let ab = midpoint(a, b, &mut vertices, &mut cache);
                let bc = midpoint(b, c, &mut vertices, &mut cache);
                let ca = midpoint(c, a, &mut vertices, &mut cache);

                next_faces.push([a, ab, ca]);
                next_faces.push([b, bc, ab]);
                next_faces.push([c, ca, bc]);
                next_faces.push([ab, bc, ca]);
            }

            faces = next_faces;
        }

        let mut geo = Geometry::with_capacity(vertices.len(), faces.len());
        let handles: Vec<_> = vertices
            .iter()
            .map(|pos| geo.add_point(params.center + *pos * params.radius))
            .collect();

        for [a, b, c] in faces {
            let pa = vertices[a];
            let pb = vertices[b];
            let pc = vertices[c];
            let normal = (pb - pa).cross(pc - pa);
            let centroid = (pa + pb + pc) / 3.0;
            if normal.dot(centroid) >= 0.0 {
                geo.add_face(&[handles[a], handles[b], handles[c]]);
            } else {
                geo.add_face(&[handles[a], handles[c], handles[b]]);
            }
        }

        Ok(geo)
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use procgeo_core::PrimHandle;

    use super::*;
    use crate::generate;

    fn face_normal(geo: &Geometry, prim_idx: usize) -> Vec3 {
        let points = geo.prim_points(PrimHandle::from_index(prim_idx));
        let p0 = geo.point_pos(points[0]);
        let p1 = geo.point_pos(points[1]);
        let p2 = geo.point_pos(points[2]);
        (p1 - p0).cross(p2 - p0)
    }

    #[test]
    fn icosphere_default() {
        let geo = generate(&IcosphereSop, &IcosphereParams::default()).unwrap();

        assert_eq!(geo.num_points(), 162);
        assert_eq!(geo.num_prims(), 320);

        for point in geo.points() {
            assert_relative_eq!(point.length(), 0.5, epsilon = 1e-5);
        }
    }

    #[test]
    fn icosphere_base_counts() {
        let geo = generate(
            &IcosphereSop,
            &IcosphereParams {
                subdivisions: 0,
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(geo.num_points(), 12);
        assert_eq!(geo.num_prims(), 20);

        let bb = geo.bounding_box();
        assert!(bb.max.x > 0.4);
        assert!(bb.min.x < -0.4);
    }

    #[test]
    fn icosphere_winding_is_outward() {
        let geo = generate(
            &IcosphereSop,
            &IcosphereParams {
                subdivisions: 1,
                ..Default::default()
            },
        )
        .unwrap();

        for prim_idx in 0..geo.num_prims() {
            let points = geo.prim_points(PrimHandle::from_index(prim_idx));
            let centroid =
                points.iter().map(|&p| geo.point_pos(p)).sum::<Vec3>() / points.len() as f32;
            assert!(
                face_normal(&geo, prim_idx).dot(centroid) > 0.0,
                "face {prim_idx} should point outward"
            );
        }
    }

    #[test]
    fn icosphere_rejects_invalid_radius() {
        assert!(
            generate(
                &IcosphereSop,
                &IcosphereParams {
                    radius: 0.0,
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
}
