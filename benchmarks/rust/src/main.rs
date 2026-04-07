use glam::Vec3;
use procgeo::prelude::*;
use serde::Serialize;
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Result format
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct BenchResult {
    framework: &'static str,
    language: &'static str,
    category: &'static str,
    operation: &'static str,
    scale: u32,
    mean_ms: f64,
    std_ms: f64,
    iterations: u32,
}

// ---------------------------------------------------------------------------
// Timing harness
// ---------------------------------------------------------------------------

fn bench<F: FnMut()>(mut f: F) -> (f64, f64, u32) {
    // Warmup
    for _ in 0..3 {
        f();
    }

    // Determine iteration count: at least 10, enough to fill ~2s
    let probe_start = Instant::now();
    f();
    let probe_dur = probe_start.elapsed();

    let iters = if probe_dur < Duration::from_millis(1) {
        1000u32
    } else if probe_dur < Duration::from_millis(10) {
        200
    } else if probe_dur < Duration::from_millis(100) {
        50
    } else {
        10
    };

    let mut times = Vec::with_capacity(iters as usize);
    for _ in 0..iters {
        let start = Instant::now();
        f();
        times.push(start.elapsed().as_secs_f64() * 1000.0); // ms
    }

    let mean = times.iter().sum::<f64>() / times.len() as f64;
    let variance = times.iter().map(|t| (t - mean).powi(2)).sum::<f64>() / times.len() as f64;
    let std = variance.sqrt();

    (mean, std, iters)
}

// ---------------------------------------------------------------------------
// Scale helpers
// ---------------------------------------------------------------------------

/// Grid rows/cols to produce approximately `target` vertices.
fn grid_rc(target: u32) -> u32 {
    (target as f64).sqrt().ceil() as u32
}

/// Sphere rows/cols to produce approximately `target` vertices.
fn sphere_rc(target: u32) -> (u32, u32) {
    let cols = ((target as f64) * 2.0).sqrt().ceil() as u32;
    let rows = (target as f64 / cols as f64).ceil() as u32;
    (rows.max(3), cols.max(4))
}

// ---------------------------------------------------------------------------
// procgeo benchmarks
// ---------------------------------------------------------------------------

fn bench_procgeo(results: &mut Vec<BenchResult>) {
    let scales: &[u32] = &[100, 10_000, 100_000];

    for &scale in scales {
        let rc = grid_rc(scale);

        // -- Creation: Grid --
        let (mean, std, iters) = bench(|| {
            let _ = generate(
                &GridSop,
                &GridParams {
                    rows: rc,
                    cols: rc,
                    ..Default::default()
                },
            );
        });
        results.push(BenchResult {
            framework: "procgeo",
            language: "rust",
            category: "creation",
            operation: "grid",
            scale,
            mean_ms: mean,
            std_ms: std,
            iterations: iters,
        });

        // -- Creation: Sphere --
        let (sr, sc) = sphere_rc(scale);
        let (mean, std, iters) = bench(|| {
            let _ = generate(
                &SphereSop,
                &SphereParams {
                    rows: sr,
                    cols: sc,
                    ..Default::default()
                },
            );
        });
        results.push(BenchResult {
            framework: "procgeo",
            language: "rust",
            category: "creation",
            operation: "sphere",
            scale,
            mean_ms: mean,
            std_ms: std,
            iterations: iters,
        });

        // -- Creation: Box --
        let (mean, std, iters) = bench(|| {
            let _ = generate(&BoxSop, &BoxParams::default());
        });
        results.push(BenchResult {
            framework: "procgeo",
            language: "rust",
            category: "creation",
            operation: "box",
            scale,
            mean_ms: mean,
            std_ms: std,
            iterations: iters,
        });

        // -- Transform --
        let grid =
            generate(&GridSop, &GridParams { rows: rc, cols: rc, ..Default::default() }).unwrap();
        let (mean, std, iters) = bench(|| {
            let _ = grid.clone().apply(
                &TransformSop,
                &TransformParams {
                    translate: Vec3::new(10.0, 0.0, 0.0),
                    scale: Vec3::splat(2.0),
                    ..Default::default()
                },
            );
        });
        results.push(BenchResult {
            framework: "procgeo",
            language: "rust",
            category: "transform",
            operation: "translate_scale",
            scale,
            mean_ms: mean,
            std_ms: std,
            iterations: iters,
        });

        // -- Subdivide (only at smaller scales to avoid blowup) --
        if scale <= 10_000 {
            let small_rc = grid_rc(scale);
            let small_grid = generate(
                &GridSop,
                &GridParams {
                    rows: small_rc,
                    cols: small_rc,
                    ..Default::default()
                },
            )
            .unwrap();
            let (mean, std, iters) = bench(|| {
                let _ = small_grid.clone().apply(
                    &SubdivideSop,
                    &SubdivideParams {
                        depth: 1,
                        ..Default::default()
                    },
                );
            });
            results.push(BenchResult {
                framework: "procgeo",
                language: "rust",
                category: "transform",
                operation: "subdivide",
                scale,
                mean_ms: mean,
                std_ms: std,
                iterations: iters,
            });
        }

        // -- Smooth --
        let (mean, std, iters) = bench(|| {
            let _ = grid.clone().apply(
                &SmoothSop,
                &SmoothParams {
                    iterations: 3,
                    strength: 0.5,
                },
            );
        });
        results.push(BenchResult {
            framework: "procgeo",
            language: "rust",
            category: "transform",
            operation: "smooth",
            scale,
            mean_ms: mean,
            std_ms: std,
            iterations: iters,
        });

        // -- Fuse --
        let (mean, std, iters) = bench(|| {
            let _ = grid.clone().apply(&FuseSop, &FuseParams { distance: 0.001 });
        });
        results.push(BenchResult {
            framework: "procgeo",
            language: "rust",
            category: "topology",
            operation: "fuse",
            scale,
            mean_ms: mean,
            std_ms: std,
            iterations: iters,
        });

        // -- Scatter --
        let (mean, std, iters) = bench(|| {
            let _ = grid.clone().apply(
                &ScatterSop,
                &ScatterParams {
                    count: scale,
                    seed: 42,
                },
            );
        });
        results.push(BenchResult {
            framework: "procgeo",
            language: "rust",
            category: "topology",
            operation: "scatter",
            scale,
            mean_ms: mean,
            std_ms: std,
            iterations: iters,
        });

        // -- Full Pipeline --
        let (mean, std, iters) = bench(|| {
            let _ = generate(
                &GridSop,
                &GridParams {
                    rows: rc,
                    cols: rc,
                    ..Default::default()
                },
            )
            .and_then(|g| {
                g.apply(
                    &TransformSop,
                    &TransformParams {
                        translate: Vec3::new(0.0, 1.0, 0.0),
                        scale: Vec3::splat(2.0),
                        ..Default::default()
                    },
                )
            })
            .and_then(|g| {
                g.apply(
                    &SmoothSop,
                    &SmoothParams {
                        iterations: 2,
                        strength: 0.5,
                    },
                )
            })
            .and_then(|g| g.apply(&FuseSop, &FuseParams { distance: 0.001 }))
            .and_then(|g| g.apply(&NormalSop, &NormalParams));
        });
        results.push(BenchResult {
            framework: "procgeo",
            language: "rust",
            category: "pipeline",
            operation: "full_pipeline",
            scale,
            mean_ms: mean,
            std_ms: std,
            iterations: iters,
        });
    }
}

// ---------------------------------------------------------------------------
// Deform benchmarks
// ---------------------------------------------------------------------------

fn bench_deform(results: &mut Vec<BenchResult>) {
    // ── Bend ─────────────────────────────────────────────────────────────
    let bend_scales: &[(u32, u32)] = &[(100, 10), (10_000, 100), (100_000, 317)];

    for &(scale, rc) in bend_scales {
        let grid = generate(
            &GridSop,
            &GridParams {
                rows: rc,
                cols: rc,
                size: [10.0, 10.0],
                orientation: GridOrientation::XY,
                ..Default::default()
            },
        )
        .unwrap();

        let (mean, std, iters) = bench(|| {
            let _ = grid.clone().apply(
                &BendSop,
                &BendParams {
                    bend_enable: true,
                    bend_angle: 90.0,
                    capture_origin: Vec3::ZERO,
                    capture_direction: Vec3::Y,
                    capture_length: 5.0,
                    up_vector: Vec3::Z,
                    limit_to_capture_region: false,
                    ..BendParams::default()
                },
            );
        });
        results.push(BenchResult {
            framework: "procgeo",
            language: "rust",
            category: "deform",
            operation: "bend",
            scale,
            mean_ms: mean,
            std_ms: std,
            iterations: iters,
        });
    }

    // ── Twist ────────────────────────────────────────────────────────────
    {
        let rc = 100;
        let scale = 10_000;
        let grid = generate(
            &GridSop,
            &GridParams {
                rows: rc,
                cols: rc,
                size: [10.0, 10.0],
                orientation: GridOrientation::XY,
                ..Default::default()
            },
        )
        .unwrap();

        let (mean, std, iters) = bench(|| {
            let _ = grid.clone().apply(
                &BendSop,
                &BendParams {
                    twist_enable: true,
                    twist_angle: 360.0,
                    capture_origin: Vec3::ZERO,
                    capture_direction: Vec3::Y,
                    capture_length: 5.0,
                    up_vector: Vec3::Z,
                    limit_to_capture_region: false,
                    ..BendParams::default()
                },
            );
        });
        results.push(BenchResult {
            framework: "procgeo",
            language: "rust",
            category: "deform",
            operation: "twist",
            scale,
            mean_ms: mean,
            std_ms: std,
            iterations: iters,
        });
    }

    // ── Length Scale ──────────────────────────────────────────────────────
    {
        let rc = 100;
        let scale = 10_000;
        let grid = generate(
            &GridSop,
            &GridParams {
                rows: rc,
                cols: rc,
                size: [10.0, 10.0],
                orientation: GridOrientation::XY,
                ..Default::default()
            },
        )
        .unwrap();

        let (mean, std, iters) = bench(|| {
            let _ = grid.clone().apply(
                &BendSop,
                &BendParams {
                    length_scale_enable: true,
                    length_scale: 2.0,
                    preserve_volume: true,
                    capture_origin: Vec3::ZERO,
                    capture_direction: Vec3::Y,
                    capture_length: 5.0,
                    up_vector: Vec3::Z,
                    limit_to_capture_region: false,
                    ..BendParams::default()
                },
            );
        });
        results.push(BenchResult {
            framework: "procgeo",
            language: "rust",
            category: "deform",
            operation: "length_scale",
            scale,
            mean_ms: mean,
            std_ms: std,
            iterations: iters,
        });
    }

    // ── Taper ────────────────────────────────────────────────────────────
    {
        let rc = 100;
        let scale = 10_000;
        let grid = generate(
            &GridSop,
            &GridParams {
                rows: rc,
                cols: rc,
                size: [10.0, 10.0],
                orientation: GridOrientation::XY,
                ..Default::default()
            },
        )
        .unwrap();

        let (mean, std, iters) = bench(|| {
            let _ = grid.clone().apply(
                &BendSop,
                &BendParams {
                    taper_enable: true,
                    taper_value: 0.5,
                    taper_along: [true, true],
                    capture_origin: Vec3::ZERO,
                    capture_direction: Vec3::Y,
                    capture_length: 5.0,
                    up_vector: Vec3::Z,
                    limit_to_capture_region: false,
                    ..BendParams::default()
                },
            );
        });
        results.push(BenchResult {
            framework: "procgeo",
            language: "rust",
            category: "deform",
            operation: "taper",
            scale,
            mean_ms: mean,
            std_ms: std,
            iterations: iters,
        });
    }

    // ── PointDeform ──────────────────────────────────────────────────────
    let pd_scales: &[(u32, u32)] = &[(100, 10), (10_000, 100)];

    for &(scale, rc) in pd_scales {
        let mesh = generate(
            &GridSop,
            &GridParams {
                rows: rc,
                cols: rc,
                size: [2.0, 2.0],
                orientation: GridOrientation::XY,
                ..Default::default()
            },
        )
        .unwrap();

        // 8-point lattice cube surrounding the mesh
        let lattice_pts: Vec<Vec3> = vec![
            Vec3::new(-2.0, -2.0, -2.0),
            Vec3::new(2.0, -2.0, -2.0),
            Vec3::new(2.0, 2.0, -2.0),
            Vec3::new(-2.0, 2.0, -2.0),
            Vec3::new(-2.0, -2.0, 2.0),
            Vec3::new(2.0, -2.0, 2.0),
            Vec3::new(2.0, 2.0, 2.0),
            Vec3::new(-2.0, 2.0, 2.0),
        ];

        let mut rest_geo = Geometry::new();
        let rest_handles: Vec<_> = lattice_pts.iter().map(|&p| rest_geo.add_point(p)).collect();
        if rest_handles.len() >= 3 {
            rest_geo.add_polygon(
                &rest_handles[..3],
                PolyType::Closed,
            );
        }

        // Translate the deformed lattice
        let offset = Vec3::new(1.0, 0.5, 0.0);
        let deformed_pts: Vec<Vec3> = lattice_pts.iter().map(|&p| p + offset).collect();
        let mut def_geo = Geometry::new();
        let def_handles: Vec<_> = deformed_pts.iter().map(|&p| def_geo.add_point(p)).collect();
        if def_handles.len() >= 3 {
            def_geo.add_polygon(
                &def_handles[..3],
                PolyType::Closed,
            );
        }

        let params = PointDeformParams {
            radius: 5.0,
            max_points: 8,
            ..Default::default()
        };

        let (mean, std, iters) = bench(|| {
            let _ = PointDeformSop.execute(&[&mesh, &rest_geo, &def_geo], &params);
        });
        results.push(BenchResult {
            framework: "procgeo",
            language: "rust",
            category: "deform",
            operation: "point_deform",
            scale,
            mean_ms: mean,
            std_ms: std,
            iterations: iters,
        });
    }
}

// ---------------------------------------------------------------------------
// Boolean benchmarks
// ---------------------------------------------------------------------------

fn bench_boolean(results: &mut Vec<BenchResult>) {
    use procgeo::sops::boolean::bvh::{Triangle, TriangleBvh};
    use procgeo::sops::boolean::classification::is_inside_mesh;

    // ── Helper: make box geometry ────────────────────────────────────────
    let make_box = |center: Vec3, size: Vec3| -> Geometry {
        generate(&BoxSop, &BoxParams { size, center }).unwrap()
    };

    // ── Helper: subdivided box (more triangles) ─────────────────────────
    let make_subdivided_box = |center: Vec3, size: Vec3, depth: u32| -> Geometry {
        let b = make_box(center, size);
        b.apply(&SubdivideSop, &SubdivideParams { depth, ..Default::default() })
            .unwrap()
    };

    // ── Union/small ──────────────────────────────────────────────────────
    {
        let a = make_box(Vec3::ZERO, Vec3::ONE);
        let b = make_box(Vec3::new(0.5, 0.0, 0.0), Vec3::ONE);

        let (mean, std, iters) = bench(|| {
            let _ = BooleanSop.execute(
                &[&a, &b],
                &BooleanParams {
                    operation: BooleanOp::Union,
                    ..Default::default()
                },
            );
        });
        results.push(BenchResult {
            framework: "procgeo",
            language: "rust",
            category: "boolean",
            operation: "union_small",
            scale: 8,
            mean_ms: mean,
            std_ms: std,
            iterations: iters,
        });
    }

    // ── Union/medium ─────────────────────────────────────────────────────
    {
        let a = make_subdivided_box(Vec3::ZERO, Vec3::ONE, 3);
        let b = make_subdivided_box(Vec3::new(0.5, 0.0, 0.0), Vec3::ONE, 3);

        let (mean, std, iters) = bench(|| {
            let _ = BooleanSop.execute(
                &[&a, &b],
                &BooleanParams {
                    operation: BooleanOp::Union,
                    ..Default::default()
                },
            );
        });
        results.push(BenchResult {
            framework: "procgeo",
            language: "rust",
            category: "boolean",
            operation: "union_medium",
            scale: a.num_prims() as u32 + b.num_prims() as u32,
            mean_ms: mean,
            std_ms: std,
            iterations: iters,
        });
    }

    // ── Intersect/medium ─────────────────────────────────────────────────
    {
        let a = make_subdivided_box(Vec3::ZERO, Vec3::ONE, 3);
        let b = make_subdivided_box(Vec3::new(0.5, 0.0, 0.0), Vec3::ONE, 3);

        let (mean, std, iters) = bench(|| {
            let _ = BooleanSop.execute(
                &[&a, &b],
                &BooleanParams {
                    operation: BooleanOp::Intersect,
                    ..Default::default()
                },
            );
        });
        results.push(BenchResult {
            framework: "procgeo",
            language: "rust",
            category: "boolean",
            operation: "intersect_medium",
            scale: a.num_prims() as u32 + b.num_prims() as u32,
            mean_ms: mean,
            std_ms: std,
            iterations: iters,
        });
    }

    // ── Subtract/medium ──────────────────────────────────────────────────
    {
        let a = make_subdivided_box(Vec3::ZERO, Vec3::ONE, 3);
        let b = make_subdivided_box(Vec3::new(0.5, 0.0, 0.0), Vec3::ONE, 3);

        let (mean, std, iters) = bench(|| {
            let _ = BooleanSop.execute(
                &[&a, &b],
                &BooleanParams {
                    operation: BooleanOp::Subtract,
                    ..Default::default()
                },
            );
        });
        results.push(BenchResult {
            framework: "procgeo",
            language: "rust",
            category: "boolean",
            operation: "subtract_medium",
            scale: a.num_prims() as u32 + b.num_prims() as u32,
            mean_ms: mean,
            std_ms: std,
            iterations: iters,
        });
    }

    // ── BVH Build/1000 ──────────────────────────────────────────────────
    {
        let tris_1000: Vec<Triangle> = (0..1000)
            .map(|i| {
                let x = (i % 32) as f32;
                let z = (i / 32) as f32;
                Triangle {
                    v0: Vec3::new(x, 0.0, z),
                    v1: Vec3::new(x + 1.0, 0.0, z),
                    v2: Vec3::new(x, 0.0, z + 1.0),
                    index: i,
                }
            })
            .collect();

        let (mean, std, iters) = bench(|| {
            let _ = TriangleBvh::build(&tris_1000);
        });
        results.push(BenchResult {
            framework: "procgeo",
            language: "rust",
            category: "boolean",
            operation: "bvh_build",
            scale: 1000,
            mean_ms: mean,
            std_ms: std,
            iterations: iters,
        });
    }

    // ── BVH Build/10000 ─────────────────────────────────────────────────
    {
        let tris_10k: Vec<Triangle> = (0..10_000)
            .map(|i| {
                let x = (i % 100) as f32;
                let z = (i / 100) as f32;
                Triangle {
                    v0: Vec3::new(x, 0.0, z),
                    v1: Vec3::new(x + 1.0, 0.0, z),
                    v2: Vec3::new(x, 0.0, z + 1.0),
                    index: i,
                }
            })
            .collect();

        let (mean, std, iters) = bench(|| {
            let _ = TriangleBvh::build(&tris_10k);
        });
        results.push(BenchResult {
            framework: "procgeo",
            language: "rust",
            category: "boolean",
            operation: "bvh_build",
            scale: 10_000,
            mean_ms: mean,
            std_ms: std,
            iterations: iters,
        });
    }

    // ── Classification/1000 ──────────────────────────────────────────────
    {
        // Build a unit cube as 12 triangles for classification
        let h = 0.5_f32;
        let mut cube_tris: Vec<Triangle> = Vec::with_capacity(12);
        let mut idx = 0usize;
        let mut push_quad = |a: Vec3, b: Vec3, c: Vec3, d: Vec3| {
            cube_tris.push(Triangle { v0: a, v1: b, v2: c, index: idx });
            idx += 1;
            cube_tris.push(Triangle { v0: a, v1: c, v2: d, index: idx });
            idx += 1;
        };
        push_quad(Vec3::new(-h,-h,h), Vec3::new(h,-h,h), Vec3::new(h,h,h), Vec3::new(-h,h,h));
        push_quad(Vec3::new(h,-h,-h), Vec3::new(-h,-h,-h), Vec3::new(-h,h,-h), Vec3::new(h,h,-h));
        push_quad(Vec3::new(h,-h,h), Vec3::new(h,-h,-h), Vec3::new(h,h,-h), Vec3::new(h,h,h));
        push_quad(Vec3::new(-h,-h,-h), Vec3::new(-h,-h,h), Vec3::new(-h,h,h), Vec3::new(-h,h,-h));
        push_quad(Vec3::new(-h,h,h), Vec3::new(h,h,h), Vec3::new(h,h,-h), Vec3::new(-h,h,-h));
        push_quad(Vec3::new(-h,-h,-h), Vec3::new(h,-h,-h), Vec3::new(h,-h,h), Vec3::new(-h,-h,h));

        // Generate 1000 random-ish test points in [-2, 2]^3
        let test_points: Vec<Vec3> = (0..1000)
            .map(|i| {
                let t = i as f32 / 1000.0;
                let x = ((t * 137.0).sin() * 2.0).clamp(-2.0, 2.0);
                let y = ((t * 251.0).sin() * 2.0).clamp(-2.0, 2.0);
                let z = ((t * 389.0).sin() * 2.0).clamp(-2.0, 2.0);
                Vec3::new(x, y, z)
            })
            .collect();

        let (mean, std, iters) = bench(|| {
            for pt in &test_points {
                let _ = is_inside_mesh(*pt, &cube_tris);
            }
        });
        results.push(BenchResult {
            framework: "procgeo",
            language: "rust",
            category: "boolean",
            operation: "classification",
            scale: 1000,
            mean_ms: mean,
            std_ms: std,
            iterations: iters,
        });
    }
}

// ---------------------------------------------------------------------------
// parry3d benchmarks
// ---------------------------------------------------------------------------

fn bench_parry3d(results: &mut Vec<BenchResult>) {
    use parry3d::math::Point;
    use parry3d::shape::{Ball, Cuboid, TriMesh};

    let scales: &[u32] = &[100, 10_000, 100_000];

    for &scale in scales {
        // -- Creation: Sphere (Ball shape — parry doesn't produce mesh vertices,
        //    but we can create a TriMesh from icosphere subdivision) --
        let (mean, std, iters) = bench(|| {
            let _ = Ball::new(1.0);
        });
        results.push(BenchResult {
            framework: "parry3d",
            language: "rust",
            category: "creation",
            operation: "sphere",
            scale,
            mean_ms: mean,
            std_ms: std,
            iterations: iters,
        });

        // -- Creation: Box --
        let (mean, std, iters) = bench(|| {
            let _ = Cuboid::new(nalgebra::Vector3::new(0.5, 0.5, 0.5));
        });
        results.push(BenchResult {
            framework: "parry3d",
            language: "rust",
            category: "creation",
            operation: "box",
            scale,
            mean_ms: mean,
            std_ms: std,
            iterations: iters,
        });

        // -- Creation: Grid (as TriMesh) --
        let rc = grid_rc(scale) as usize;
        let (mean, std, iters) = bench(|| {
            let mut vertices = Vec::with_capacity(rc * rc);
            let mut indices = Vec::with_capacity((rc - 1) * (rc - 1) * 2);
            for z in 0..rc {
                for x in 0..rc {
                    vertices.push(Point::new(x as f32, 0.0, z as f32));
                }
            }
            for z in 0..(rc - 1) {
                for x in 0..(rc - 1) {
                    let i = (z * rc + x) as u32;
                    let w = rc as u32;
                    indices.push([i, i + 1, i + w]);
                    indices.push([i + 1, i + w + 1, i + w]);
                }
            }
            let _ = TriMesh::new(vertices, indices);
        });
        results.push(BenchResult {
            framework: "parry3d",
            language: "rust",
            category: "creation",
            operation: "grid",
            scale,
            mean_ms: mean,
            std_ms: std,
            iterations: iters,
        });
    }
}

// ---------------------------------------------------------------------------
// meshopt benchmarks
// ---------------------------------------------------------------------------

fn bench_meshopt(results: &mut Vec<BenchResult>) {
    let scales: &[u32] = &[100, 10_000, 100_000];

    for &scale in scales {
        let rc = grid_rc(scale) as usize;

        // Build a grid mesh for meshopt to work on
        let mut positions: Vec<f32> = Vec::with_capacity(rc * rc * 3);
        let mut indices: Vec<u32> = Vec::with_capacity((rc - 1) * (rc - 1) * 2 * 3);
        for z in 0..rc {
            for x in 0..rc {
                positions.push(x as f32);
                positions.push(0.0);
                positions.push(z as f32);
            }
        }
        for z in 0..(rc - 1) {
            for x in 0..(rc - 1) {
                let i = (z * rc + x) as u32;
                let w = rc as u32;
                indices.push(i);
                indices.push(i + 1);
                indices.push(i + w);
                indices.push(i + 1);
                indices.push(i + w + 1);
                indices.push(i + w);
            }
        }

        // -- Optimize vertex cache --
        let (mean, std, iters) = bench(|| {
            let mut idx = indices.clone();
            meshopt::optimize_vertex_cache(&mut idx, rc * rc);
        });
        results.push(BenchResult {
            framework: "meshopt",
            language: "rust",
            category: "topology",
            operation: "optimize_vertex_cache",
            scale,
            mean_ms: mean,
            std_ms: std,
            iterations: iters,
        });

        // -- Simplify (poly_reduce equivalent) --
        let vertex_bytes = meshopt::typed_to_bytes(&positions);
        let stride = 3 * std::mem::size_of::<f32>();
        let vertex_adapter = meshopt::VertexDataAdapter::new(vertex_bytes, stride, 0).unwrap();
        let target_count = indices.len() / 2;
        let (mean, std, iters) = bench(|| {
            let _ = meshopt::simplify(
                &indices,
                &vertex_adapter,
                target_count,
                0.01,
                meshopt::SimplifyOptions::None,
                None,
            );
        });
        results.push(BenchResult {
            framework: "meshopt",
            language: "rust",
            category: "topology",
            operation: "simplify",
            scale,
            mean_ms: mean,
            std_ms: std,
            iterations: iters,
        });
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let mut results: Vec<BenchResult> = Vec::new();

    eprintln!("Running procgeo benchmarks...");
    bench_procgeo(&mut results);

    eprintln!("Running deform benchmarks...");
    bench_deform(&mut results);

    eprintln!("Running boolean benchmarks...");
    bench_boolean(&mut results);

    eprintln!("Running parry3d benchmarks...");
    bench_parry3d(&mut results);

    eprintln!("Running meshopt benchmarks...");
    bench_meshopt(&mut results);

    let output = serde_json::to_string_pretty(&results).unwrap();
    println!("{output}");
}
