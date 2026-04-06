use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use glam::Vec3;
use procgeo_core::attribute::{AttribClass, AttribDefault, AttribHandle, TypeQualifier};
use procgeo_core::geometry::Geometry;
use procgeo_core::handle::{PointHandle, PrimHandle};
// ---------------------------------------------------------------------------
// Helpers — build standard test geometries
// ---------------------------------------------------------------------------

/// Create a flat grid of `n x n` points with `(n-1)^2` quad faces.
fn make_grid(n: usize) -> Geometry {
    let mut geo = Geometry::with_capacity(n * n, (n - 1) * (n - 1));
    let mut handles = Vec::with_capacity(n * n);
    for z in 0..n {
        for x in 0..n {
            handles.push(geo.add_point(Vec3::new(x as f32, 0.0, z as f32)));
        }
    }
    for z in 0..(n - 1) {
        for x in 0..(n - 1) {
            let i = z * n + x;
            geo.add_face(&[handles[i], handles[i + 1], handles[i + n + 1], handles[i + n]]);
        }
    }
    geo
}

// ---------------------------------------------------------------------------
// Point benchmarks
// ---------------------------------------------------------------------------

fn bench_add_points(c: &mut Criterion) {
    let mut group = c.benchmark_group("points/add");
    for count in [100, 1_000, 10_000] {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &n| {
            b.iter(|| {
                let mut geo = Geometry::with_capacity(n, 0);
                for i in 0..n {
                    geo.add_point(Vec3::new(i as f32, 0.0, 0.0));
                }
                black_box(&geo);
            });
        });
    }
    group.finish();
}

fn bench_point_read(c: &mut Criterion) {
    let geo = make_grid(100); // 10k points
    let handles: Vec<PointHandle> = (0..geo.num_points())
        .map(PointHandle::from_index)
        .collect();

    c.bench_function("points/read_all_10k", |b| {
        b.iter(|| {
            let mut sum = Vec3::ZERO;
            for &h in &handles {
                sum += geo.point_pos(h);
            }
            black_box(sum);
        });
    });
}

fn bench_point_write(c: &mut Criterion) {
    let mut geo = make_grid(100); // 10k points

    c.bench_function("points/write_all_10k", |b| {
        b.iter(|| {
            for i in 0..geo.num_points() {
                let h = PointHandle::from_index(i);
                let p = geo.point_pos(h);
                geo.set_point_pos(h, p + Vec3::new(0.0, 0.001, 0.0));
            }
            black_box(&geo);
        });
    });
}

fn bench_point_soa_iteration(c: &mut Criterion) {
    let geo = make_grid(100); // 10k points
    let storage = geo.point_storage();

    c.bench_function("points/soa_iterate_10k", |b| {
        b.iter(|| {
            let xs = storage.x_slice();
            let ys = storage.y_slice();
            let zs = storage.z_slice();
            let mut sum = 0.0f32;
            for i in 0..xs.len() {
                sum += xs[i] + ys[i] + zs[i];
            }
            black_box(sum);
        });
    });
}

// ---------------------------------------------------------------------------
// Primitive benchmarks
// ---------------------------------------------------------------------------

fn bench_add_faces(c: &mut Criterion) {
    let mut group = c.benchmark_group("prims/add_faces");
    for count in [100, 1_000, 10_000] {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &n| {
            b.iter(|| {
                let mut geo = Geometry::with_capacity(n * 3, n);
                for i in 0..n {
                    let base = i as f32 * 2.0;
                    let p0 = geo.add_point(Vec3::new(base, 0.0, 0.0));
                    let p1 = geo.add_point(Vec3::new(base + 1.0, 0.0, 0.0));
                    let p2 = geo.add_point(Vec3::new(base + 0.5, 1.0, 0.0));
                    geo.add_face(&[p0, p1, p2]);
                }
                black_box(&geo);
            });
        });
    }
    group.finish();
}

fn bench_prim_points_lookup(c: &mut Criterion) {
    let geo = make_grid(50); // ~2.4k quads
    let num_prims = geo.num_prims();

    c.bench_function("prims/prim_points_lookup_2k", |b| {
        b.iter(|| {
            for i in 0..num_prims {
                black_box(geo.prim_points(PrimHandle::from_index(i)));
            }
        });
    });
}

// ---------------------------------------------------------------------------
// Attribute benchmarks
// ---------------------------------------------------------------------------

fn bench_attrib_create(c: &mut Criterion) {
    c.bench_function("attribs/create_10_on_10k_pts", |b| {
        b.iter(|| {
            let mut geo = make_grid(100);
            for i in 0..10 {
                geo.add_attrib(
                    AttribClass::Point,
                    format!("attr_{i}"),
                    AttribDefault::Float(0.0),
                    TypeQualifier::None,
                )
                .unwrap();
            }
            black_box(&geo);
        });
    });
}

fn bench_attrib_get_set(c: &mut Criterion) {
    let mut geo = make_grid(100); // 10k points
    geo.add_attrib(
        AttribClass::Point,
        "pscale",
        AttribDefault::Float(1.0),
        TypeQualifier::None,
    )
    .unwrap();
    let handle: AttribHandle<f32> = geo.find_attrib(AttribClass::Point, "pscale").unwrap();
    let n = geo.num_points();

    let mut group = c.benchmark_group("attribs");

    group.bench_function("get_f32_10k", |b| {
        b.iter(|| {
            let mut sum = 0.0f32;
            for i in 0..n {
                sum += geo.get_attrib(&handle, i).unwrap();
            }
            black_box(sum);
        });
    });

    group.bench_function("set_f32_10k", |b| {
        b.iter(|| {
            for i in 0..n {
                geo.set_attrib(&handle, i, i as f32 * 0.1).unwrap();
            }
            black_box(&geo);
        });
    });

    group.finish();
}

fn bench_attrib_vec3_get_set(c: &mut Criterion) {
    let mut geo = make_grid(100);
    geo.add_attrib(
        AttribClass::Point,
        "Cd",
        AttribDefault::Vector3([1.0, 1.0, 1.0]),
        TypeQualifier::Color,
    )
    .unwrap();
    let handle: AttribHandle<[f32; 3]> = geo.find_attrib(AttribClass::Point, "Cd").unwrap();
    let n = geo.num_points();

    let mut group = c.benchmark_group("attribs");

    group.bench_function("get_vec3_10k", |b| {
        b.iter(|| {
            let mut sum = [0.0f32; 3];
            for i in 0..n {
                let v = geo.get_attrib(&handle, i).unwrap();
                sum[0] += v[0];
                sum[1] += v[1];
                sum[2] += v[2];
            }
            black_box(sum);
        });
    });

    group.bench_function("set_vec3_10k", |b| {
        b.iter(|| {
            for i in 0..n {
                let f = i as f32 * 0.001;
                geo.set_attrib(&handle, i, [f, f, f]).unwrap();
            }
            black_box(&geo);
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Group benchmarks
// ---------------------------------------------------------------------------

fn bench_group_operations(c: &mut Criterion) {
    let mut geo = make_grid(100); // 10k points
    geo.create_point_group("A");
    geo.create_point_group("B");

    // Populate groups: A = even indices, B = indices divisible by 3
    let n = geo.num_points();
    {
        let grp_a = geo.groups_mut().point_group_mut("A").unwrap();
        for i in (0..n).step_by(2) {
            grp_a.add(i);
        }
        let grp_b = geo.groups_mut().point_group_mut("B").unwrap();
        for i in (0..n).step_by(3) {
            grp_b.add(i);
        }
    }

    let mut group = c.benchmark_group("groups");

    group.bench_function("add_10k", |b| {
        b.iter(|| {
            geo.create_point_group("bench_temp");
            let grp = geo.groups_mut().point_group_mut("bench_temp").unwrap();
            for i in 0..n {
                grp.add(i);
            }
            geo.groups_mut().delete_point_group("bench_temp");
            black_box(&geo);
        });
    });

    group.bench_function("contains_10k", |b| {
        b.iter(|| {
            let grp = geo.groups().point_group("A").unwrap();
            let mut count = 0usize;
            for i in 0..n {
                if grp.contains(i) {
                    count += 1;
                }
            }
            black_box(count);
        });
    });

    group.bench_function("count_10k", |b| {
        b.iter(|| {
            black_box(geo.groups().point_group("A").unwrap().count());
        });
    });

    group.bench_function("iter_set_10k", |b| {
        b.iter(|| {
            let grp = geo.groups().point_group("A").unwrap();
            let mut sum = 0usize;
            for idx in grp.iter_set() {
                sum += idx;
            }
            black_box(sum);
        });
    });

    group.bench_function("union_10k", |b| {
        b.iter_batched(
            || geo.groups().point_group("A").unwrap().clone(),
            |mut a| {
                let b_ref = geo.groups().point_group("B").unwrap();
                a.union(b_ref);
                black_box(a);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function("intersect_10k", |b| {
        b.iter_batched(
            || geo.groups().point_group("A").unwrap().clone(),
            |mut a| {
                let b_ref = geo.groups().point_group("B").unwrap();
                a.intersect(b_ref);
                black_box(a);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Spatial benchmarks
// ---------------------------------------------------------------------------

fn bench_bounding_box(c: &mut Criterion) {
    let mut group = c.benchmark_group("spatial/bbox");
    for size in [100, 1_000, 10_000, 100_000] {
        let mut geo = Geometry::with_capacity(size, 0);
        for i in 0..size {
            let f = i as f32;
            geo.add_point(Vec3::new(f.sin(), f.cos(), f * 0.01));
        }
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| black_box(geo.bounding_box()));
        });
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Rebuild benchmarks
// ---------------------------------------------------------------------------

fn bench_rebuild_keeping_prims(c: &mut Criterion) {
    let geo = make_grid(50); // ~2.4k quads, 2.5k points
    let num_prims = geo.num_prims();
    // Keep every other prim
    let keep: Vec<bool> = (0..num_prims).map(|i| i % 2 == 0).collect();

    c.bench_function("rebuild/keep_half_prims_2k", |b| {
        b.iter(|| black_box(geo.rebuild_keeping_prims(&keep)));
    });
}

fn bench_rebuild_keeping_points(c: &mut Criterion) {
    let geo = make_grid(50);
    let num_pts = geo.num_points();
    let keep: Vec<bool> = (0..num_pts).map(|i| i % 2 == 0).collect();

    c.bench_function("rebuild/keep_half_points_2k", |b| {
        b.iter(|| black_box(geo.rebuild_keeping_points(&keep)));
    });
}

// ---------------------------------------------------------------------------
// Full pipeline benchmark — build geometry + attributes + groups
// ---------------------------------------------------------------------------

fn bench_full_pipeline(c: &mut Criterion) {
    c.bench_function("pipeline/grid_100x100_with_attribs_and_groups", |b| {
        b.iter(|| {
            let n = 100;
            let mut geo = Geometry::with_capacity(n * n, (n - 1) * (n - 1));

            // Points
            let mut handles = Vec::with_capacity(n * n);
            for z in 0..n {
                for x in 0..n {
                    handles.push(geo.add_point(Vec3::new(x as f32, 0.0, z as f32)));
                }
            }

            // Faces
            for z in 0..(n - 1) {
                for x in 0..(n - 1) {
                    let i = z * n + x;
                    geo.add_face(&[handles[i], handles[i + 1], handles[i + n + 1], handles[i + n]]);
                }
            }

            // Attribute
            geo.add_attrib(
                AttribClass::Point,
                "Cd",
                AttribDefault::Vector3([1.0, 1.0, 1.0]),
                TypeQualifier::Color,
            )
            .unwrap();
            let cd: AttribHandle<[f32; 3]> =
                geo.find_attrib(AttribClass::Point, "Cd").unwrap();
            for i in 0..geo.num_points() {
                let f = i as f32 / geo.num_points() as f32;
                geo.set_attrib(&cd, i, [f, 1.0 - f, 0.5]).unwrap();
            }

            // Group
            geo.create_point_group("border");
            let np = geo.num_points();
            let grp = geo.groups_mut().point_group_mut("border").unwrap();
            for i in 0..np {
                let x = i % n;
                let z = i / n;
                if x == 0 || x == n - 1 || z == 0 || z == n - 1 {
                    grp.add(i);
                }
            }

            // BBox
            black_box(geo.bounding_box());
            black_box(&geo);
        });
    });
}

// ---------------------------------------------------------------------------
// Criterion setup
// ---------------------------------------------------------------------------

criterion_group!(
    benches,
    bench_add_points,
    bench_point_read,
    bench_point_write,
    bench_point_soa_iteration,
    bench_add_faces,
    bench_prim_points_lookup,
    bench_attrib_create,
    bench_attrib_get_set,
    bench_attrib_vec3_get_set,
    bench_group_operations,
    bench_bounding_box,
    bench_rebuild_keeping_prims,
    bench_rebuild_keeping_points,
    bench_full_pipeline,
);

criterion_main!(benches);
