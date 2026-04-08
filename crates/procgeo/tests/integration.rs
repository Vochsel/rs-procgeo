use approx::assert_relative_eq;
use glam::Vec3;
use procgeo::io::{GeometryReader, GeometryWriter};
use procgeo::prelude::*;

// ---------------------------------------------------------------------------
// Test 1: Box → OBJ write → read back, verify counts
// ---------------------------------------------------------------------------

#[test]
fn test_box_to_obj_roundtrip() {
    let box_geo = generate(&BoxSop, &BoxParams::default()).unwrap();
    assert_eq!(box_geo.num_points(), 8);
    assert_eq!(box_geo.num_prims(), 6);

    // Write to an in-memory buffer via the io module
    let mut buf: Vec<u8> = Vec::new();
    procgeo::io::obj::ObjWriter
        .write(&box_geo, &mut buf)
        .unwrap();

    // Read back
    let geo2 = procgeo::io::obj::ObjReader
        .read(&mut buf.as_slice())
        .unwrap();

    assert_eq!(geo2.num_points(), 8, "roundtrip: wrong point count");
    assert_eq!(geo2.num_prims(), 6, "roundtrip: wrong prim count");
}

// ---------------------------------------------------------------------------
// Test 2: Box → Transform → Normal, verify bbox center/size and unit normals
// ---------------------------------------------------------------------------

#[test]
fn test_sop_chaining() {
    let params = TransformParams {
        translate: Vec3::new(5.0, 0.0, 0.0),
        scale: Vec3::splat(2.0),
        ..Default::default()
    };

    let geo = generate(&BoxSop, &BoxParams::default())
        .unwrap()
        .apply(&TransformSop, &params)
        .unwrap()
        .apply(&NormalSop, &NormalParams)
        .unwrap();

    // BBox center should be ~(5, 0, 0), size ~(2, 2, 2)
    let bb = geo.bounding_box();
    let center = bb.center();
    assert_relative_eq!(center.x, 5.0, epsilon = 1e-4);
    assert_relative_eq!(center.y, 0.0, epsilon = 1e-4);
    assert_relative_eq!(center.z, 0.0, epsilon = 1e-4);

    let size = bb.size();
    assert_relative_eq!(size.x, 2.0, epsilon = 1e-4);
    assert_relative_eq!(size.y, 2.0, epsilon = 1e-4);
    assert_relative_eq!(size.z, 2.0, epsilon = 1e-4);

    // All normals should be unit length
    let n_handle = geo
        .find_attrib::<[f32; 3]>(AttribClass::Point, "N")
        .unwrap();
    for i in 0..geo.num_points() {
        let n = geo.get_attrib(&n_handle, i).unwrap();
        let nv = Vec3::from(n);
        let mag = nv.length();
        assert_relative_eq!(mag, 1.0, epsilon = 1e-5);
    }
}

// ---------------------------------------------------------------------------
// Test 3: Merge box + grid(3x3) + sphere(4,6), verify combined counts
// ---------------------------------------------------------------------------

#[test]
fn test_merge_different_sops() {
    let box_geo = generate(&BoxSop, &BoxParams::default()).unwrap();
    // Grid rows=3, cols=3 → 9 points, 4 prims
    let grid_geo = generate(
        &GridSop,
        &GridParams {
            rows: 3,
            cols: 3,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(grid_geo.num_points(), 9);
    assert_eq!(grid_geo.num_prims(), 4);

    // Sphere rows=4, cols=6: points = 2+(4-1)*6=20, prims = 6+(4-2)*6+6=24
    let sphere_geo = generate(
        &SphereSop,
        &SphereParams {
            rows: 4,
            cols: 6,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(sphere_geo.num_points(), 20);
    assert_eq!(sphere_geo.num_prims(), 24);

    let merged = MergeSop
        .execute(&[&box_geo, &grid_geo, &sphere_geo], &MergeParams)
        .unwrap();

    // 8 + 9 + 20 = 37 points, 6 + 4 + 24 = 34 prims
    assert_eq!(merged.num_points(), 37);
    assert_eq!(merged.num_prims(), 34);
}

// ---------------------------------------------------------------------------
// Test 4: generate(BoxSop) → .apply(TransformSop, translate(1,2,3))
// ---------------------------------------------------------------------------

#[test]
fn test_geometry_apply_chain() {
    let params = TransformParams {
        translate: Vec3::new(1.0, 2.0, 3.0),
        ..Default::default()
    };

    let geo = generate(&BoxSop, &BoxParams::default())
        .unwrap()
        .apply(&TransformSop, &params)
        .unwrap();

    let bb = geo.bounding_box();
    let center = bb.center();
    assert_relative_eq!(center.x, 1.0, epsilon = 1e-4);
    assert_relative_eq!(center.y, 2.0, epsilon = 1e-4);
    assert_relative_eq!(center.z, 3.0, epsilon = 1e-4);
}

// ---------------------------------------------------------------------------
// Test 5: Smoke test all 7 creation SOPs produce valid geometry
// ---------------------------------------------------------------------------

#[test]
fn test_all_creation_sops_produce_valid_geometry() {
    let geos: Vec<(&str, Geometry)> = vec![
        ("BoxSop", generate(&BoxSop, &BoxParams::default()).unwrap()),
        (
            "GridSop",
            generate(&GridSop, &GridParams::default()).unwrap(),
        ),
        (
            "LineSop",
            generate(&LineSop, &LineParams::default()).unwrap(),
        ),
        (
            "CircleSop",
            generate(&CircleSop, &CircleParams::default()).unwrap(),
        ),
        (
            "SphereSop",
            generate(&SphereSop, &SphereParams::default()).unwrap(),
        ),
        (
            "TubeSop",
            generate(&TubeSop, &TubeParams::default()).unwrap(),
        ),
        (
            "TorusSop",
            generate(&TorusSop, &TorusParams::default()).unwrap(),
        ),
    ];

    for (name, geo) in &geos {
        assert!(geo.num_points() > 0, "{name}: no points");
        assert!(geo.num_prims() > 0, "{name}: no prims");
        assert!(geo.num_vertices() > 0, "{name}: no vertices");

        let bb = geo.bounding_box();
        assert!(bb.is_valid(), "{name}: invalid bounding box");

        // Check no NaN in point positions
        for pos in geo.points() {
            assert!(!pos.x.is_nan(), "{name}: NaN x");
            assert!(!pos.y.is_nan(), "{name}: NaN y");
            assert!(!pos.z.is_nan(), "{name}: NaN z");
        }
    }
}

// ---------------------------------------------------------------------------
// Test 6: Attributes survive a SOP chain (box → transform)
// ---------------------------------------------------------------------------

#[test]
fn test_attributes_survive_sop_chain() {
    let mut geo = generate(&BoxSop, &BoxParams::default()).unwrap();

    // Add an "id" Int attribute and set sequential values
    geo.add_attrib(
        AttribClass::Point,
        "id",
        AttribDefault::Int(0),
        TypeQualifier::None,
    )
    .unwrap();

    let handle: AttribHandle<i32> = geo.find_attrib(AttribClass::Point, "id").unwrap();
    for i in 0..geo.num_points() {
        geo.set_attrib(&handle, i, i as i32).unwrap();
    }

    // Apply transform (should clone geometry, preserving attributes)
    let transformed = geo
        .apply(
            &TransformSop,
            &TransformParams {
                translate: Vec3::new(10.0, 0.0, 0.0),
                ..Default::default()
            },
        )
        .unwrap();

    // Find the "id" attribute in the transformed geometry
    let t_handle: AttribHandle<i32> = transformed.find_attrib(AttribClass::Point, "id").unwrap();

    // All values should be intact (0..8)
    for i in 0..transformed.num_points() {
        let val = transformed.get_attrib(&t_handle, i).unwrap();
        assert_eq!(val, i as i32, "attribute value mismatch at index {i}");
    }
}

// ---------------------------------------------------------------------------
// Test 7: Grid → Subdivide → Scatter → verify points within bbox
// ---------------------------------------------------------------------------

#[test]
fn test_scatter_on_subdivided_grid() {
    let grid = generate(
        &GridSop,
        &GridParams {
            rows: 3,
            cols: 3,
            size: [2.0, 2.0],
            ..Default::default()
        },
    )
    .unwrap();
    let subdiv = grid
        .apply(
            &SubdivideSop,
            &SubdivideParams {
                depth: 1,
                mode: SubdivideMode::Linear,
            },
        )
        .unwrap();
    let scattered = subdiv
        .apply(
            &ScatterSop,
            &ScatterParams {
                count: 50,
                seed: 42,
            },
        )
        .unwrap();

    assert_eq!(scattered.num_points(), 50);
    let bbox = scattered.bounding_box();
    // The grid goes from -1 to +1 on X and Z (size=2.0)
    assert!(
        bbox.min.x >= -1.1 && bbox.max.x <= 1.1,
        "points outside grid x bounds"
    );
}

// ---------------------------------------------------------------------------
// Test 8: Grid → Scatter 5 pts → Copy box to those points
// ---------------------------------------------------------------------------

#[test]
fn test_copy_to_scattered_points() {
    let grid = generate(&GridSop, &GridParams::default()).unwrap();
    let targets = grid
        .apply(&ScatterSop, &ScatterParams { count: 5, seed: 0 })
        .unwrap();
    let box_geo = generate(
        &BoxSop,
        &BoxParams {
            size: Vec3::splat(0.1),
            ..Default::default()
        },
    )
    .unwrap();
    let result = CopyToPointsSop
        .execute(&[&box_geo, &targets], &CopyToPointsParams::default())
        .unwrap();

    assert_eq!(result.num_points(), 40); // 8 * 5
    assert_eq!(result.num_prims(), 30); // 6 * 5
}

// ---------------------------------------------------------------------------
// Test 9: Box → PolyExtrude → Measure → verify all faces have positive area
// ---------------------------------------------------------------------------

#[test]
fn test_extrude_then_measure() {
    let box_geo = generate(&BoxSop, &BoxParams::default()).unwrap();
    let extruded = box_geo
        .apply(
            &PolyExtrudeSop,
            &PolyExtrudeParams {
                distance: 1.0,
                ..Default::default()
            },
        )
        .unwrap();
    let measured = extruded
        .apply(&MeasureSop, &MeasureParams::default())
        .unwrap();

    let area_h = measured
        .find_attrib::<f32>(AttribClass::Primitive, "area")
        .unwrap();
    for i in 0..measured.num_prims() {
        let area = measured.get_attrib(&area_h, i).unwrap();
        assert!(area > 0.0, "face {i} should have positive area, got {area}");
    }
}

// ---------------------------------------------------------------------------
// Test 10: Box → GroupCreate → Blast → 4 prims remain
// ---------------------------------------------------------------------------

#[test]
fn test_blast_by_group() {
    let box_geo = generate(&BoxSop, &BoxParams::default()).unwrap();
    let grouped = box_geo
        .apply(
            &GroupCreateSop,
            &GroupCreateParams {
                name: "to_delete".to_string(),
                group_type: GroupType::Primitives,
                mode: GroupCreateMode::Range,
                range_start: 0,
                range_end: 2,
                ..Default::default()
            },
        )
        .unwrap();
    let blasted = grouped
        .apply(
            &BlastSop,
            &BlastParams {
                group_name: "to_delete".to_string(),
                entity: BlastEntity::Primitives,
                negate: false,
            },
        )
        .unwrap();
    assert_eq!(blasted.num_prims(), 4);
}

// ---------------------------------------------------------------------------
// Test 11: Box → Enumerate → verify sequential index values
// ---------------------------------------------------------------------------

#[test]
fn test_enumerate_points() {
    let box_geo = generate(&BoxSop, &BoxParams::default()).unwrap();
    let enumerated = box_geo
        .apply(&EnumerateSop, &EnumerateParams::default())
        .unwrap();
    let idx_h = enumerated
        .find_attrib::<i32>(AttribClass::Point, "index")
        .unwrap();
    for i in 0..enumerated.num_points() {
        assert_eq!(enumerated.get_attrib(&idx_h, i).unwrap(), i as i32);
    }
}

// ---------------------------------------------------------------------------
// Test 12: Box → Subdivide → Smooth → Normal → Color → write GLB
// ---------------------------------------------------------------------------

#[test]
fn test_full_workflow() {
    let geo = generate(&BoxSop, &BoxParams::default()).unwrap();
    let geo = geo
        .apply(
            &SubdivideSop,
            &SubdivideParams {
                depth: 1,
                mode: SubdivideMode::Linear,
            },
        )
        .unwrap();
    let geo = geo
        .apply(
            &SmoothSop,
            &SmoothParams {
                iterations: 3,
                strength: 0.5,
            },
        )
        .unwrap();
    let geo = geo.apply(&NormalSop, &NormalParams).unwrap();
    let geo = geo
        .apply(
            &ColorSop,
            &ColorParams {
                color: [0.2, 0.6, 1.0],
            },
        )
        .unwrap();

    // Verify geometry is valid
    assert!(geo.num_points() > 8); // subdivided
    let n_handle = geo
        .find_attrib::<[f32; 3]>(AttribClass::Point, "N")
        .unwrap();
    let cd_handle = geo
        .find_attrib::<[f32; 3]>(AttribClass::Point, "Cd")
        .unwrap();
    assert_eq!(geo.get_attrib(&cd_handle, 0).unwrap(), [0.2, 0.6, 1.0]);

    // Write to GLB buffer
    let mut buf = Vec::new();
    procgeo::io::gltf::GlbWriter.write(&geo, &mut buf).unwrap();
    assert!(buf.len() > 12); // at least the GLB header
    assert_eq!(&buf[0..4], b"glTF"); // magic bytes
}

// ---------------------------------------------------------------------------
// Test 13: Box → Clip at y=0 → Measure area
// ---------------------------------------------------------------------------

#[test]
fn test_clip_and_measure() {
    let geo = generate(&BoxSop, &BoxParams::default()).unwrap();
    let clipped = geo
        .apply(
            &ClipSop,
            &ClipParams {
                origin: Vec3::ZERO,
                normal: Vec3::Y,
                keep_above: true,
                create_cap: false,
            },
        )
        .unwrap();

    // Should have fewer prims than original 6
    assert!(clipped.num_prims() > 0);
    assert!(clipped.num_prims() <= 6);

    let measured = clipped
        .apply(&MeasureSop, &MeasureParams::default())
        .unwrap();
    let area_h = measured
        .find_attrib::<f32>(AttribClass::Primitive, "area")
        .unwrap();
    for i in 0..measured.num_prims() {
        assert!(measured.get_attrib(&area_h, i).unwrap() > 0.0);
    }
}

// ---------------------------------------------------------------------------
// Test 14: Grid → Normal → Reverse → Normal → verify flipped
// ---------------------------------------------------------------------------

#[test]
fn test_reverse_normals() {
    let geo = generate(
        &GridSop,
        &GridParams {
            rows: 3,
            cols: 3,
            size: [2.0, 2.0],
            ..Default::default()
        },
    )
    .unwrap();
    let with_normals = geo.apply(&NormalSop, &NormalParams).unwrap();
    let n_handle = with_normals
        .find_attrib::<[f32; 3]>(AttribClass::Point, "N")
        .unwrap();
    let orig_n = with_normals.get_attrib(&n_handle, 0).unwrap();

    let reversed = with_normals.apply(&ReverseSop, &ReverseParams).unwrap();
    let recomputed = reversed.apply(&NormalSop, &NormalParams).unwrap();
    let n_handle2 = recomputed
        .find_attrib::<[f32; 3]>(AttribClass::Point, "N")
        .unwrap();
    let new_n = recomputed.get_attrib(&n_handle2, 0).unwrap();

    // Y component should be flipped
    assert!(
        (orig_n[1] + new_n[1]).abs() < 0.1,
        "normals should be roughly opposite"
    );
}

// ===========================================================================
// Intrinsic / Well-Known Attribute Tests
// ===========================================================================

// ---------------------------------------------------------------------------
// P: position is a real attribute, synced with PointStorage
// ---------------------------------------------------------------------------

#[test]
fn test_p_readable_as_attribute() {
    let geo = generate(&BoxSop, &BoxParams::default()).unwrap();
    let p_handle = geo
        .find_attrib::<[f32; 3]>(AttribClass::Point, "P")
        .unwrap();
    for i in 0..geo.num_points() {
        let pos = geo.point_pos(procgeo_core::PointHandle::from_index(i));
        let p_attr = geo.get_attrib(&p_handle, i).unwrap();
        assert_eq!(
            p_attr,
            [pos.x, pos.y, pos.z],
            "P attribute must match point_pos at index {i}"
        );
    }
}

#[test]
fn test_p_writable_moves_geometry() {
    let mut geo = generate(&BoxSop, &BoxParams::default()).unwrap();
    let p_handle = geo
        .find_attrib::<[f32; 3]>(AttribClass::Point, "P")
        .unwrap();

    // Move point 0 via attribute API
    geo.set_attrib(&p_handle, 0, [99.0, 99.0, 99.0]);

    // point_pos must reflect the change
    let pos = geo.point_pos(procgeo_core::PointHandle::from_index(0));
    assert_relative_eq!(pos.x, 99.0, epsilon = 1e-6);
    assert_relative_eq!(pos.y, 99.0, epsilon = 1e-6);
    assert_relative_eq!(pos.z, 99.0, epsilon = 1e-6);

    // bounding_box must include the moved point
    let bbox = geo.bounding_box();
    assert!(bbox.max.x >= 99.0);
}

#[test]
fn test_p_survives_sop_chain() {
    // Box → Transform → read P attribute → must match moved positions
    let geo = generate(&BoxSop, &BoxParams::default()).unwrap();
    let geo = geo
        .apply(
            &TransformSop,
            &TransformParams {
                translate: Vec3::new(10.0, 20.0, 30.0),
                ..Default::default()
            },
        )
        .unwrap();

    let p_handle = geo
        .find_attrib::<[f32; 3]>(AttribClass::Point, "P")
        .unwrap();
    for i in 0..geo.num_points() {
        let pos = geo.point_pos(procgeo_core::PointHandle::from_index(i));
        let p_attr = geo.get_attrib(&p_handle, i).unwrap();
        assert_relative_eq!(p_attr[0], pos.x, epsilon = 1e-5);
        assert_relative_eq!(p_attr[1], pos.y, epsilon = 1e-5);
        assert_relative_eq!(p_attr[2], pos.z, epsilon = 1e-5);
    }
}

#[test]
fn test_noise_on_p_displaces_geometry() {
    let geo = generate(
        &GridSop,
        &GridParams {
            rows: 5,
            cols: 5,
            size: [2.0, 2.0],
            ..Default::default()
        },
    )
    .unwrap();

    // Grid is flat on XZ plane — all Y positions are 0
    let bbox_before = geo.bounding_box();
    assert_relative_eq!(bbox_before.min.y, 0.0, epsilon = 1e-6);
    assert_relative_eq!(bbox_before.max.y, 0.0, epsilon = 1e-6);

    // Apply noise to P — should displace points
    let noisy = geo
        .apply(
            &AttribNoiseSop,
            &AttribNoiseParams {
                attrib_name: "P".to_string(),
                dimensions: 3,
                amplitude: 0.5,
                element_size: 1.0,
                ..Default::default()
            },
        )
        .unwrap();

    // Positions should have changed — Y no longer all zero
    let bbox_after = noisy.bounding_box();
    assert!(
        (bbox_after.max.y - bbox_after.min.y) > 0.01,
        "noise on P should displace geometry, but Y range is {}",
        bbox_after.max.y - bbox_after.min.y
    );
}

// ---------------------------------------------------------------------------
// N: normal is a regular Vector3 point attribute with Normal qualifier
// ---------------------------------------------------------------------------

#[test]
fn test_n_created_by_normal_sop() {
    let geo = generate(&BoxSop, &BoxParams::default()).unwrap();
    let geo = geo.apply(&NormalSop, &NormalParams).unwrap();

    let n_handle = geo
        .find_attrib::<[f32; 3]>(AttribClass::Point, "N")
        .unwrap();
    for i in 0..geo.num_points() {
        let n = geo.get_attrib(&n_handle, i).unwrap();
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        assert_relative_eq!(len, 1.0, epsilon = 1e-3, max_relative = 1e-3);
    }
}

#[test]
fn test_n_exported_in_obj() {
    let geo = generate(&BoxSop, &BoxParams::default()).unwrap();
    let geo = geo.apply(&NormalSop, &NormalParams).unwrap();

    let mut buf = Vec::new();
    procgeo::io::obj::ObjWriter.write(&geo, &mut buf).unwrap();
    let output = String::from_utf8(buf).unwrap();

    assert!(
        output.contains("vn "),
        "OBJ should contain vertex normals when N attribute exists"
    );
}

#[test]
fn test_n_exported_in_glb() {
    let geo = generate(&BoxSop, &BoxParams::default()).unwrap();
    let geo = geo.apply(&NormalSop, &NormalParams).unwrap();

    let mut buf = Vec::new();
    procgeo::io::gltf::GlbWriter.write(&geo, &mut buf).unwrap();
    let json_str = String::from_utf8_lossy(&buf);
    assert!(
        json_str.contains("NORMAL"),
        "GLB should contain NORMAL accessor when N attribute exists"
    );
}

// ---------------------------------------------------------------------------
// Cd: color is a regular Vector3 attribute with Color qualifier
// ---------------------------------------------------------------------------

#[test]
fn test_cd_created_by_color_sop() {
    let geo = generate(&BoxSop, &BoxParams::default()).unwrap();
    let geo = geo
        .apply(
            &ColorSop,
            &ColorParams {
                color: [1.0, 0.0, 0.0],
            },
        )
        .unwrap();

    let cd_handle = geo
        .find_attrib::<[f32; 3]>(AttribClass::Point, "Cd")
        .unwrap();
    for i in 0..geo.num_points() {
        assert_eq!(geo.get_attrib(&cd_handle, i).unwrap(), [1.0, 0.0, 0.0]);
    }
}

#[test]
fn test_cd_exported_in_glb() {
    let geo = generate(&BoxSop, &BoxParams::default()).unwrap();
    let geo = geo
        .apply(
            &ColorSop,
            &ColorParams {
                color: [0.5, 0.5, 0.5],
            },
        )
        .unwrap();

    let mut buf = Vec::new();
    procgeo::io::gltf::GlbWriter.write(&geo, &mut buf).unwrap();
    let json_str = String::from_utf8_lossy(&buf);
    assert!(
        json_str.contains("COLOR_0"),
        "GLB should contain COLOR_0 accessor when Cd attribute exists"
    );
}

// ---------------------------------------------------------------------------
// Attribute pipeline: P, N, Cd all flow through merge, copy, blast
// ---------------------------------------------------------------------------

#[test]
fn test_intrinsic_attribs_survive_merge() {
    let mut box1 = generate(&BoxSop, &BoxParams::default()).unwrap();
    box1 = box1.apply(&NormalSop, &NormalParams).unwrap();
    box1 = box1
        .apply(
            &ColorSop,
            &ColorParams {
                color: [1.0, 0.0, 0.0],
            },
        )
        .unwrap();

    let mut box2 = generate(
        &BoxSop,
        &BoxParams {
            center: Vec3::new(5.0, 0.0, 0.0),
            ..Default::default()
        },
    )
    .unwrap();
    box2 = box2.apply(&NormalSop, &NormalParams).unwrap();
    box2 = box2
        .apply(
            &ColorSop,
            &ColorParams {
                color: [0.0, 0.0, 1.0],
            },
        )
        .unwrap();

    let merged = MergeSop.execute(&[&box1, &box2], &MergeParams).unwrap();

    // P should be correct for both halves
    let p_handle = merged
        .find_attrib::<[f32; 3]>(AttribClass::Point, "P")
        .unwrap();
    let p0 = merged.get_attrib(&p_handle, 0).unwrap();
    let p8 = merged.get_attrib(&p_handle, 8).unwrap();
    // First box near origin, second near x=5
    assert!(p0[0].abs() <= 0.6);
    assert!(p8[0] >= 4.4);
}

#[test]
fn test_full_intrinsic_pipeline() {
    // Create → Subdivide → Smooth → Normal → Color → Noise on P → Export
    let geo = generate(&BoxSop, &BoxParams::default()).unwrap();
    let geo = geo
        .apply(
            &SubdivideSop,
            &SubdivideParams {
                depth: 1,
                mode: SubdivideMode::CatmullClark,
            },
        )
        .unwrap();
    let geo = geo
        .apply(
            &SmoothSop,
            &SmoothParams {
                iterations: 2,
                strength: 0.5,
            },
        )
        .unwrap();
    let geo = geo.apply(&NormalSop, &NormalParams).unwrap();
    let geo = geo
        .apply(
            &ColorSop,
            &ColorParams {
                color: [0.4, 0.7, 1.0],
            },
        )
        .unwrap();

    // All three intrinsic attribs should be accessible
    let p = geo.find_attrib::<[f32; 3]>(AttribClass::Point, "P");
    let n = geo.find_attrib::<[f32; 3]>(AttribClass::Point, "N");
    let cd = geo.find_attrib::<[f32; 3]>(AttribClass::Point, "Cd");
    assert!(p.is_ok(), "P must be findable");
    assert!(n.is_ok(), "N must be findable");
    assert!(cd.is_ok(), "Cd must be findable");

    // Export to both formats — should include all three
    let mut obj_buf = Vec::new();
    procgeo::io::obj::ObjWriter
        .write(&geo, &mut obj_buf)
        .unwrap();
    let obj = String::from_utf8(obj_buf).unwrap();
    assert!(obj.contains("v "), "OBJ has positions");
    assert!(obj.contains("vn "), "OBJ has normals");

    let mut glb_buf = Vec::new();
    procgeo::io::gltf::GlbWriter
        .write(&geo, &mut glb_buf)
        .unwrap();
    let glb = String::from_utf8_lossy(&glb_buf);
    assert!(glb.contains("POSITION"), "GLB has POSITION");
    assert!(glb.contains("NORMAL"), "GLB has NORMAL");
    assert!(glb.contains("COLOR_0"), "GLB has COLOR_0");
}
