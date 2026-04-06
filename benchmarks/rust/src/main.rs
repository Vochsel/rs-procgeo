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

    eprintln!("Running parry3d benchmarks...");
    bench_parry3d(&mut results);

    eprintln!("Running meshopt benchmarks...");
    bench_meshopt(&mut results);

    let output = serde_json::to_string_pretty(&results).unwrap();
    println!("{output}");
}
