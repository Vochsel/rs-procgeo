use std::collections::HashSet;

use glam::Vec3;
use serde::{Deserialize, Serialize};

use procgeo_core::{AttribClass, AttribDefault, Geometry, PointHandle, PrimHandle, TypeQualifier};

use crate::reshape::{ClipParams, ClipSop};
use crate::{Sop, SopError};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VoronoiFractureParams {
    /// Offset the cut planes along their normals. Positive values create gaps
    /// between pieces; negative values create overlap.
    pub cut_plane_offset: f32,
    /// Whether to create interior faces on cut surfaces.
    pub create_inside_faces: bool,
}

impl Default for VoronoiFractureParams {
    fn default() -> Self {
        VoronoiFractureParams {
            cut_plane_offset: 0.0,
            create_inside_faces: true,
        }
    }
}

pub struct VoronoiFractureSop;

/// Merge geometry pieces (each with `prims_per_piece` face counts) into a single
/// geometry and add a "piece" primitive integer attribute.
fn merge_pieces_with_attrib(pieces: Vec<Geometry>) -> Result<Geometry, SopError> {
    let mut out = Geometry::new();

    // Create the "piece" primitive attribute upfront (it gets auto-resized as prims are added)
    out.add_attrib(
        AttribClass::Primitive,
        "piece",
        AttribDefault::Int(0),
        TypeQualifier::None,
    )?;

    let piece_handle = out.find_attrib::<i32>(AttribClass::Primitive, "piece")?;

    for (piece_idx, piece) in pieces.into_iter().enumerate() {
        let point_offset = out.num_points();
        let prim_offset = out.num_prims();

        // Copy all points
        for pt_pos in piece.points() {
            out.add_point(pt_pos);
        }

        // Copy all primitives with remapped points
        for prim_i in 0..piece.num_prims() {
            let ph = PrimHandle::from_index(prim_i);
            let pt_handles = piece.prim_points(ph);

            let remapped: Vec<PointHandle> = pt_handles
                .iter()
                .map(|p| PointHandle::from_index(p.index() + point_offset))
                .collect();

            let prim = piece.prim(ph);
            match prim {
                procgeo_core::Primitive::Polygon(poly) => {
                    use procgeo_core::PolyType;
                    match poly.poly_type {
                        PolyType::Closed => {
                            out.add_face(&remapped);
                        }
                        PolyType::Open => {
                            out.add_polyline(&remapped);
                        }
                    }
                }
            }

            // Set piece attribute for this primitive
            let new_prim_idx = prim_offset + prim_i;
            out.set_attrib(&piece_handle, new_prim_idx, piece_idx as i32)?;
        }
    }

    Ok(out)
}

impl Sop for VoronoiFractureSop {
    type Params = VoronoiFractureParams;

    fn name(&self) -> &'static str {
        "voronoi_fracture"
    }

    fn input_count(&self) -> (usize, usize) {
        (2, 2)
    }

    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError> {
        self.validate_inputs(inputs)?;

        let input_mesh = inputs[0];
        let seed_geo = inputs[1];

        // Gather seed points from second input
        let seeds: Vec<Vec3> = (0..seed_geo.num_points())
            .map(|i| seed_geo.point_pos(PointHandle::from_index(i)))
            .collect();

        if seeds.is_empty() {
            return Ok(input_mesh.clone());
        }

        // Deduplicate seeds (in case input points overlap)
        let seeds = {
            let mut seen: HashSet<[u32; 3]> = HashSet::new();
            seeds
                .into_iter()
                .filter(|s| {
                    let key = [s.x.to_bits(), s.y.to_bits(), s.z.to_bits()];
                    seen.insert(key)
                })
                .collect::<Vec<_>>()
        };

        let n = seeds.len();
        let mut pieces: Vec<Geometry> = Vec::with_capacity(n);

        let clip_sop = ClipSop;

        for i in 0..n {
            // Start with a copy of the input mesh
            let mut piece = input_mesh.clone();

            // Clip by each bisecting plane between seed[i] and seed[j]
            for j in 0..n {
                if i == j {
                    continue;
                }

                let midpoint = (seeds[i] + seeds[j]) * 0.5;
                let diff = seeds[i] - seeds[j];
                let normal = diff.normalize_or_zero();

                if normal.length_squared() < 1e-10 {
                    // Seeds are identical, skip
                    continue;
                }

                let clip_params = ClipParams {
                    origin: midpoint + normal * params.cut_plane_offset,
                    normal,
                    keep_above: true,
                    create_cap: params.create_inside_faces,
                };

                piece = clip_sop.execute(&[&piece], &clip_params)?;

                // If the piece is empty after clipping, no need to continue
                if piece.num_prims() == 0 {
                    break;
                }
            }

            pieces.push(piece);
        }

        merge_pieces_with_attrib(pieces)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creation::box_sop::{BoxParams, BoxSop};
    use crate::generate;
    use glam::Vec3;

    fn make_box() -> Geometry {
        generate(&BoxSop, &BoxParams::default()).unwrap()
    }

    fn make_seed_points(positions: &[Vec3]) -> Geometry {
        let mut geo = Geometry::new();
        for &pos in positions {
            geo.add_point(pos);
        }
        geo
    }

    #[test]
    fn voronoi_basic() {
        // Box + 3 explicit seed points → should produce 3 pieces, total prims > original 6
        let box_geo = make_box();
        let seeds = make_seed_points(&[
            Vec3::new(-0.4, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.4, 0.0, 0.0),
        ]);

        let params = VoronoiFractureParams::default();

        let result = VoronoiFractureSop
            .execute(&[&box_geo, &seeds], &params)
            .unwrap();

        // Should have more faces than the original 6 (each piece is a clipped sub-box)
        assert!(
            result.num_prims() > 6,
            "expected more than 6 prims after fracture, got {}",
            result.num_prims()
        );
    }

    #[test]
    fn voronoi_piece_attribute() {
        // Verify "piece" attribute exists and contains values 0, 1, 2
        let box_geo = make_box();
        let seeds = make_seed_points(&[
            Vec3::new(-0.3, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.3, 0.0, 0.0),
        ]);

        let params = VoronoiFractureParams::default();
        let result = VoronoiFractureSop
            .execute(&[&box_geo, &seeds], &params)
            .unwrap();

        // Verify "piece" attribute exists
        let piece_handle = result
            .find_attrib::<i32>(AttribClass::Primitive, "piece")
            .expect("piece attribute should exist");

        // Collect all piece values
        let mut piece_values: std::collections::HashSet<i32> = std::collections::HashSet::new();
        for prim_i in 0..result.num_prims() {
            let val = result.get_attrib(&piece_handle, prim_i).unwrap();
            piece_values.insert(val);
        }

        // All three pieces should be represented (0, 1, 2)
        assert!(piece_values.contains(&0), "piece 0 should be present");
        assert!(piece_values.contains(&1), "piece 1 should be present");
        assert!(piece_values.contains(&2), "piece 2 should be present");
    }

    #[test]
    fn voronoi_requires_two_inputs() {
        // 1 input only should fail validation
        let box_geo = make_box();
        let params = VoronoiFractureParams::default();
        let result = VoronoiFractureSop.execute(&[&box_geo], &params);
        assert!(result.is_err(), "should require 2 inputs");
    }

    #[test]
    fn voronoi_preserves_volume() {
        // All pieces together should roughly cover the original bbox
        let box_geo = make_box();
        let orig_bbox = box_geo.bounding_box();

        let seeds = make_seed_points(&[
            Vec3::new(-0.3, -0.1, 0.0),
            Vec3::new(0.2, 0.1, 0.0),
            Vec3::new(0.0, 0.2, 0.2),
        ]);

        let params = VoronoiFractureParams::default();
        let result = VoronoiFractureSop
            .execute(&[&box_geo, &seeds], &params)
            .unwrap();

        // The combined bounding box of all pieces should approximately match the original
        let result_bbox = result.bounding_box();

        let eps = 0.1; // Tolerance: clip planes may slightly cut into the bbox
        assert!(
            result_bbox.min.x >= orig_bbox.min.x - eps,
            "result bbox min.x should be >= original"
        );
        assert!(
            result_bbox.max.x <= orig_bbox.max.x + eps,
            "result bbox max.x should be <= original"
        );
        assert!(
            result_bbox.min.y >= orig_bbox.min.y - eps,
            "result bbox min.y should be >= original"
        );
        assert!(
            result_bbox.max.y <= orig_bbox.max.y + eps,
            "result bbox max.y should be <= original"
        );
    }

    #[test]
    fn voronoi_cut_plane_offset() {
        // Positive offset should shrink pieces (creating gaps), resulting in fewer total prims
        let box_geo = make_box();
        let seeds = make_seed_points(&[Vec3::new(-0.3, 0.0, 0.0), Vec3::new(0.3, 0.0, 0.0)]);

        let no_offset = VoronoiFractureSop
            .execute(&[&box_geo, &seeds], &VoronoiFractureParams::default())
            .unwrap();

        let with_offset = VoronoiFractureSop
            .execute(
                &[&box_geo, &seeds],
                &VoronoiFractureParams {
                    cut_plane_offset: 0.1,
                    ..Default::default()
                },
            )
            .unwrap();

        // With offset, pieces are clipped more aggressively so should have fewer or equal points
        assert!(
            with_offset.num_points() <= no_offset.num_points(),
            "offset should shrink pieces: {} points with offset vs {} without",
            with_offset.num_points(),
            no_offset.num_points(),
        );
    }

    #[test]
    fn voronoi_generates_inside_faces() {
        let box_geo = make_box();
        let seeds = make_seed_points(&[Vec3::new(-0.25, 0.0, 0.0), Vec3::new(0.25, 0.0, 0.0)]);

        let open_shell = VoronoiFractureSop
            .execute(
                &[&box_geo, &seeds],
                &VoronoiFractureParams {
                    create_inside_faces: false,
                    ..Default::default()
                },
            )
            .unwrap();
        let capped = VoronoiFractureSop
            .execute(
                &[&box_geo, &seeds],
                &VoronoiFractureParams {
                    create_inside_faces: true,
                    ..Default::default()
                },
            )
            .unwrap();

        assert_eq!(
            open_shell.num_prims(),
            10,
            "uncapped two-piece fracture should have 10 faces, got {}",
            open_shell.num_prims()
        );
        assert_eq!(
            capped.num_prims(),
            12,
            "capped two-piece fracture should have 12 faces, got {}",
            capped.num_prims()
        );

        let piece_handle = capped
            .find_attrib::<i32>(AttribClass::Primitive, "piece")
            .expect("piece attribute should exist");

        let mut cap_faces = 0;
        let mut piece_caps = std::collections::HashSet::new();
        for prim_idx in 0..capped.num_prims() {
            let pts = capped.prim_points(PrimHandle::from_index(prim_idx));
            let positions: Vec<Vec3> = pts.iter().map(|&h| capped.point_pos(h)).collect();
            if !positions.iter().all(|p| p.x.abs() < 1e-5) {
                continue;
            }

            cap_faces += 1;
            let piece = capped.get_attrib(&piece_handle, prim_idx).unwrap();
            piece_caps.insert(piece);

            let mut nx = 0.0_f32;
            let mut ny = 0.0_f32;
            let mut nz = 0.0_f32;
            for i in 0..positions.len() {
                let cur = positions[i];
                let next = positions[(i + 1) % positions.len()];
                nx += (cur.y - next.y) * (cur.z + next.z);
                ny += (cur.z - next.z) * (cur.x + next.x);
                nz += (cur.x - next.x) * (cur.y + next.y);
            }
            let normal = Vec3::new(nx, ny, nz).normalize_or_zero();
            if piece == 0 {
                assert!(
                    normal.x > 0.9,
                    "left piece cap should face +X, got normal {:?}",
                    normal
                );
            } else if piece == 1 {
                assert!(
                    normal.x < -0.9,
                    "right piece cap should face -X, got normal {:?}",
                    normal
                );
            }
        }

        assert_eq!(cap_faces, 2, "expected one interior face per piece");
        assert_eq!(piece_caps.len(), 2, "both pieces should receive a cap face");
    }
}
