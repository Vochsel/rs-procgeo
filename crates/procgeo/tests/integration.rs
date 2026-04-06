use procgeo::prelude::*;
use procgeo::io::{GeometryReader, GeometryWriter};
use approx::assert_relative_eq;
use glam::Vec3;

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
    let grid_geo = generate(&GridSop, &GridParams {
        rows: 3,
        cols: 3,
        ..Default::default()
    }).unwrap();
    assert_eq!(grid_geo.num_points(), 9);
    assert_eq!(grid_geo.num_prims(), 4);

    // Sphere rows=4, cols=6: points = 2+(4-1)*6=20, prims = 6+(4-2)*6+6=24
    let sphere_geo = generate(&SphereSop, &SphereParams {
        rows: 4,
        cols: 6,
        ..Default::default()
    }).unwrap();
    assert_eq!(sphere_geo.num_points(), 20);
    assert_eq!(sphere_geo.num_prims(), 24);

    let merged = MergeSop.execute(
        &[&box_geo, &grid_geo, &sphere_geo],
        &MergeParams,
    ).unwrap();

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
        ("BoxSop",    generate(&BoxSop,    &BoxParams::default()).unwrap()),
        ("GridSop",   generate(&GridSop,   &GridParams::default()).unwrap()),
        ("LineSop",   generate(&LineSop,   &LineParams::default()).unwrap()),
        ("CircleSop", generate(&CircleSop, &CircleParams::default()).unwrap()),
        ("SphereSop", generate(&SphereSop, &SphereParams::default()).unwrap()),
        ("TubeSop",   generate(&TubeSop,   &TubeParams::default()).unwrap()),
        ("TorusSop",  generate(&TorusSop,  &TorusParams::default()).unwrap()),
    ];

    for (name, geo) in &geos {
        assert!(geo.num_points() > 0,   "{name}: no points");
        assert!(geo.num_prims() > 0,    "{name}: no prims");
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
    ).unwrap();

    let handle: AttribHandle<i32> = geo.find_attrib(AttribClass::Point, "id").unwrap();
    for i in 0..geo.num_points() {
        geo.set_attrib(&handle, i, i as i32).unwrap();
    }

    // Apply transform (should clone geometry, preserving attributes)
    let transformed = geo
        .apply(&TransformSop, &TransformParams {
            translate: Vec3::new(10.0, 0.0, 0.0),
            ..Default::default()
        })
        .unwrap();

    // Find the "id" attribute in the transformed geometry
    let t_handle: AttribHandle<i32> = transformed
        .find_attrib(AttribClass::Point, "id")
        .unwrap();

    // All values should be intact (0..8)
    for i in 0..transformed.num_points() {
        let val = transformed.get_attrib(&t_handle, i).unwrap();
        assert_eq!(val, i as i32, "attribute value mismatch at index {i}");
    }
}
