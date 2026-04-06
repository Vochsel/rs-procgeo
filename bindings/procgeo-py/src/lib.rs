use std::sync::OnceLock;

use pyo3::prelude::*;
use procgeo_sops::Sop;

/// Geometry object wrapping the Rust Geometry struct.
#[pyclass(from_py_object)]
#[derive(Clone)]
struct Geometry {
    inner: procgeo_core::Geometry,
}

#[pymethods]
impl Geometry {
    #[new]
    fn new() -> Self {
        Self {
            inner: procgeo_core::Geometry::new(),
        }
    }

    #[getter]
    fn num_points(&self) -> usize {
        self.inner.num_points()
    }

    #[getter]
    fn num_prims(&self) -> usize {
        self.inner.num_prims()
    }

    #[getter]
    fn num_vertices(&self) -> usize {
        self.inner.num_vertices()
    }

    fn point_pos(&self, index: usize) -> (f32, f32, f32) {
        let pos = self
            .inner
            .point_pos(procgeo_core::PointHandle::from_index(index));
        (pos.x, pos.y, pos.z)
    }

    fn bounding_box(&self) -> ((f32, f32, f32), (f32, f32, f32)) {
        let bbox = self.inner.bounding_box();
        (
            (bbox.min.x, bbox.min.y, bbox.min.z),
            (bbox.max.x, bbox.max.y, bbox.max.z),
        )
    }

    fn __repr__(&self) -> String {
        format!(
            "Geometry(points={}, prims={}, vertices={})",
            self.inner.num_points(),
            self.inner.num_prims(),
            self.inner.num_vertices()
        )
    }
}

fn sop_err(e: procgeo_sops::SopError) -> PyErr {
    pyo3::exceptions::PyRuntimeError::new_err(e.to_string())
}

// ---- Creation SOPs ----

#[pyfunction]
#[pyo3(signature = (size_x=1.0, size_y=1.0, size_z=1.0, center_x=0.0, center_y=0.0, center_z=0.0))]
fn create_box(
    size_x: f32,
    size_y: f32,
    size_z: f32,
    center_x: f32,
    center_y: f32,
    center_z: f32,
) -> PyResult<Geometry> {
    let params = procgeo_sops::creation::BoxParams {
        size: glam::Vec3::new(size_x, size_y, size_z),
        center: glam::Vec3::new(center_x, center_y, center_z),
    };
    let inner = procgeo_sops::creation::BoxSop
        .execute(&[], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[pyfunction]
#[pyo3(signature = (rows=10, cols=10, size_x=10.0, size_y=10.0, center_x=0.0, center_y=0.0, center_z=0.0))]
fn create_grid(
    rows: u32,
    cols: u32,
    size_x: f32,
    size_y: f32,
    center_x: f32,
    center_y: f32,
    center_z: f32,
) -> PyResult<Geometry> {
    let params = procgeo_sops::creation::GridParams {
        size: [size_x, size_y],
        rows,
        cols,
        center: glam::Vec3::new(center_x, center_y, center_z),
        ..Default::default()
    };
    let inner = procgeo_sops::creation::GridSop
        .execute(&[], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[pyfunction]
#[pyo3(signature = (radius=0.5, rows=12, cols=24, center_x=0.0, center_y=0.0, center_z=0.0))]
fn create_sphere(
    radius: f32,
    rows: u32,
    cols: u32,
    center_x: f32,
    center_y: f32,
    center_z: f32,
) -> PyResult<Geometry> {
    let params = procgeo_sops::creation::SphereParams {
        radius: glam::Vec3::splat(radius),
        center: glam::Vec3::new(center_x, center_y, center_z),
        rows,
        cols,
    };
    let inner = procgeo_sops::creation::SphereSop
        .execute(&[], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[pyfunction]
#[pyo3(signature = (length=1.0, points=2, origin_x=0.0, origin_y=0.0, origin_z=0.0, dir_x=0.0, dir_y=1.0, dir_z=0.0))]
fn create_line(
    length: f32,
    points: u32,
    origin_x: f32,
    origin_y: f32,
    origin_z: f32,
    dir_x: f32,
    dir_y: f32,
    dir_z: f32,
) -> PyResult<Geometry> {
    let params = procgeo_sops::creation::LineParams {
        origin: glam::Vec3::new(origin_x, origin_y, origin_z),
        direction: glam::Vec3::new(dir_x, dir_y, dir_z),
        length,
        points,
    };
    let inner = procgeo_sops::creation::LineSop
        .execute(&[], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[pyfunction]
#[pyo3(signature = (radius=1.0, divisions=40, center_x=0.0, center_y=0.0, center_z=0.0))]
fn create_circle(
    radius: f32,
    divisions: u32,
    center_x: f32,
    center_y: f32,
    center_z: f32,
) -> PyResult<Geometry> {
    let params = procgeo_sops::creation::CircleParams {
        radius,
        center: glam::Vec3::new(center_x, center_y, center_z),
        divisions,
        ..Default::default()
    };
    let inner = procgeo_sops::creation::CircleSop
        .execute(&[], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[pyfunction]
#[pyo3(signature = (radius_bottom=0.5, radius_top=0.5, height=1.0, cols=24, rows=2, center_x=0.0, center_y=0.0, center_z=0.0))]
fn create_tube(
    radius_bottom: f32,
    radius_top: f32,
    height: f32,
    cols: u32,
    rows: u32,
    center_x: f32,
    center_y: f32,
    center_z: f32,
) -> PyResult<Geometry> {
    let params = procgeo_sops::creation::TubeParams {
        radius_bottom,
        radius_top,
        height,
        center: glam::Vec3::new(center_x, center_y, center_z),
        cols,
        rows,
        ..Default::default()
    };
    let inner = procgeo_sops::creation::TubeSop
        .execute(&[], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[pyfunction]
#[pyo3(signature = (radius_outer=1.0, radius_inner=0.3, rows=12, cols=24, center_x=0.0, center_y=0.0, center_z=0.0))]
fn create_torus(
    radius_outer: f32,
    radius_inner: f32,
    rows: u32,
    cols: u32,
    center_x: f32,
    center_y: f32,
    center_z: f32,
) -> PyResult<Geometry> {
    let params = procgeo_sops::creation::TorusParams {
        radius_outer,
        radius_inner,
        center: glam::Vec3::new(center_x, center_y, center_z),
        rows,
        cols,
    };
    let inner = procgeo_sops::creation::TorusSop
        .execute(&[], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[pyfunction]
#[pyo3(signature = (geo, origin_x=0.0, origin_y=0.0, origin_z=0.0, axis_x=0.0, axis_y=1.0, axis_z=0.0, divisions=24, start_angle=0.0, end_angle=360.0, end_caps=false))]
fn revolve(
    geo: &Geometry,
    origin_x: f32,
    origin_y: f32,
    origin_z: f32,
    axis_x: f32,
    axis_y: f32,
    axis_z: f32,
    divisions: u32,
    start_angle: f32,
    end_angle: f32,
    end_caps: bool,
) -> PyResult<Geometry> {
    let params = procgeo_sops::creation::RevolveParams {
        origin: glam::Vec3::new(origin_x, origin_y, origin_z),
        axis: glam::Vec3::new(axis_x, axis_y, axis_z),
        divisions,
        start_angle,
        end_angle,
        end_caps,
    };
    let inner = procgeo_sops::creation::RevolveSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[pyfunction]
#[pyo3(signature = (center_x=0.0, center_y=0.0, center_z=0.0, radius=1.0, weight=1.0, threshold=1.0, resolution=32, kernel="wyvill"))]
fn create_metaball(
    center_x: f32,
    center_y: f32,
    center_z: f32,
    radius: f32,
    weight: f32,
    threshold: f32,
    resolution: u32,
    kernel: &str,
) -> PyResult<Geometry> {
    let k = match kernel {
        "blinn" => procgeo_sops::creation::MetaballKernel::Blinn,
        "hart" => procgeo_sops::creation::MetaballKernel::Hart,
        _ => procgeo_sops::creation::MetaballKernel::Wyvill,
    };
    let params = procgeo_sops::creation::MetaballParams {
        balls: vec![procgeo_sops::creation::MetaballDef {
            center: glam::Vec3::new(center_x, center_y, center_z),
            radius,
            weight,
        }],
        threshold,
        kernel: k,
        resolution,
        ..Default::default()
    };
    let inner = procgeo_sops::creation::MetaballSop
        .execute(&[], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

// ---- Manipulation SOPs ----

#[pyfunction]
#[pyo3(signature = (geo, translate_x=0.0, translate_y=0.0, translate_z=0.0, rotate_x=0.0, rotate_y=0.0, rotate_z=0.0, scale_x=1.0, scale_y=1.0, scale_z=1.0, pivot_x=0.0, pivot_y=0.0, pivot_z=0.0))]
fn transform(
    geo: &Geometry,
    translate_x: f32,
    translate_y: f32,
    translate_z: f32,
    rotate_x: f32,
    rotate_y: f32,
    rotate_z: f32,
    scale_x: f32,
    scale_y: f32,
    scale_z: f32,
    pivot_x: f32,
    pivot_y: f32,
    pivot_z: f32,
) -> PyResult<Geometry> {
    let params = procgeo_sops::transform::TransformParams {
        translate: glam::Vec3::new(translate_x, translate_y, translate_z),
        rotate: glam::Vec3::new(rotate_x, rotate_y, rotate_z),
        scale: glam::Vec3::new(scale_x, scale_y, scale_z),
        pivot: glam::Vec3::new(pivot_x, pivot_y, pivot_z),
    };
    let inner = procgeo_sops::transform::TransformSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[pyfunction]
fn compute_normals(geo: &Geometry) -> PyResult<Geometry> {
    let inner = procgeo_sops::normals::NormalSop
        .execute(&[&geo.inner], &Default::default())
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[pyfunction]
fn merge(geometries: Vec<Geometry>) -> PyResult<Geometry> {
    let refs: Vec<&procgeo_core::Geometry> = geometries.iter().map(|g| &g.inner).collect();
    let inner = procgeo_sops::merge::MergeSop
        .execute(&refs, &Default::default())
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[pyfunction]
#[pyo3(signature = (geo, depth=1))]
fn subdivide(geo: &Geometry, depth: u32) -> PyResult<Geometry> {
    let params = procgeo_sops::reshape::SubdivideParams {
        depth,
        mode: procgeo_sops::reshape::SubdivideMode::default(),
    };
    let inner = procgeo_sops::reshape::SubdivideSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[pyfunction]
#[pyo3(signature = (geo, count=100, seed=0))]
fn scatter(geo: &Geometry, count: u32, seed: u64) -> PyResult<Geometry> {
    let params = procgeo_sops::scatter::ScatterParams { count, seed };
    let inner = procgeo_sops::scatter::ScatterSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[pyfunction]
fn copy_to_points(source: &Geometry, target: &Geometry) -> PyResult<Geometry> {
    let inner = procgeo_sops::copy::CopyToPointsSop
        .execute(&[&source.inner, &target.inner], &Default::default())
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[pyfunction]
#[pyo3(signature = (geo, distance=1.0, inset=0.0, output_front=true, output_side=true))]
fn poly_extrude(
    geo: &Geometry,
    distance: f32,
    inset: f32,
    output_front: bool,
    output_side: bool,
) -> PyResult<Geometry> {
    let params = procgeo_sops::reshape::PolyExtrudeParams {
        distance,
        inset,
        output_front,
        output_side,
    };
    let inner = procgeo_sops::reshape::PolyExtrudeSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[pyfunction]
#[pyo3(signature = (geo, iterations=1, strength=0.5))]
fn smooth(geo: &Geometry, iterations: u32, strength: f32) -> PyResult<Geometry> {
    let params = procgeo_sops::reshape::SmoothParams {
        iterations,
        strength,
    };
    let inner = procgeo_sops::reshape::SmoothSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[pyfunction]
#[pyo3(signature = (geo, origin_x=0.0, origin_y=0.0, origin_z=0.0, normal_x=0.0, normal_y=1.0, normal_z=0.0, keep_above=true))]
fn clip(
    geo: &Geometry,
    origin_x: f32,
    origin_y: f32,
    origin_z: f32,
    normal_x: f32,
    normal_y: f32,
    normal_z: f32,
    keep_above: bool,
) -> PyResult<Geometry> {
    let params = procgeo_sops::reshape::ClipParams {
        origin: glam::Vec3::new(origin_x, origin_y, origin_z),
        normal: glam::Vec3::new(normal_x, normal_y, normal_z),
        keep_above,
    };
    let inner = procgeo_sops::reshape::ClipSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[pyfunction]
fn reverse(geo: &Geometry) -> PyResult<Geometry> {
    let inner = procgeo_sops::topology::ReverseSop
        .execute(&[&geo.inner], &Default::default())
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[pyfunction]
#[pyo3(signature = (geo, length=0.1, max_segments=1000))]
fn resample(geo: &Geometry, length: f32, max_segments: u32) -> PyResult<Geometry> {
    let params = procgeo_sops::topology::ResampleParams {
        length,
        max_segments,
    };
    let inner = procgeo_sops::topology::ResampleSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[pyfunction]
#[pyo3(signature = (geo, distance=0.001))]
fn fuse(geo: &Geometry, distance: f32) -> PyResult<Geometry> {
    let params = procgeo_sops::topology::FuseParams { distance };
    let inner = procgeo_sops::topology::FuseSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[pyfunction]
#[pyo3(signature = (geo, attrib_name="class"))]
fn connectivity(geo: &Geometry, attrib_name: &str) -> PyResult<Geometry> {
    let params = procgeo_sops::topology::ConnectivityParams {
        attrib_name: attrib_name.to_string(),
    };
    let inner = procgeo_sops::topology::ConnectivitySop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[pyfunction]
#[pyo3(signature = (geo, r=1.0, g=1.0, b=1.0))]
fn color(geo: &Geometry, r: f32, g: f32, b: f32) -> PyResult<Geometry> {
    let params = procgeo_sops::color::ColorParams { color: [r, g, b] };
    let inner = procgeo_sops::color::ColorSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[pyfunction]
#[pyo3(signature = (geo, name="index", start=0))]
fn enumerate_attrib(geo: &Geometry, name: &str, start: i32) -> PyResult<Geometry> {
    let params = procgeo_sops::utility::EnumerateParams {
        name: name.to_string(),
        start,
        ..Default::default()
    };
    let inner = procgeo_sops::utility::EnumerateSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[pyfunction]
fn measure_area(geo: &Geometry) -> PyResult<Geometry> {
    let inner = procgeo_sops::measure::MeasureSop
        .execute(&[&geo.inner], &Default::default())
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

// ---- Reshape SOPs (new) ----

#[pyfunction]
#[pyo3(signature = (geo, offset=0.1, divisions=1))]
fn poly_bevel(geo: &Geometry, offset: f32, divisions: u32) -> PyResult<Geometry> {
    let params = procgeo_sops::reshape::PolyBevelParams { offset, divisions };
    let inner = procgeo_sops::reshape::PolyBevelSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[pyfunction]
#[pyo3(signature = (geo, radius=0.1, divisions=8))]
fn poly_wire(geo: &Geometry, radius: f32, divisions: u32) -> PyResult<Geometry> {
    let params = procgeo_sops::reshape::PolyWireParams { radius, divisions };
    let inner = procgeo_sops::reshape::PolyWireSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[pyfunction]
#[pyo3(signature = (geo, target_percent=0.5, preserve_boundaries=true))]
fn poly_reduce(geo: &Geometry, target_percent: f32, preserve_boundaries: bool) -> PyResult<Geometry> {
    let params = procgeo_sops::reshape::PolyReduceParams {
        target_percent,
        preserve_boundaries,
    };
    let inner = procgeo_sops::reshape::PolyReduceSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[pyfunction]
#[pyo3(signature = (geo, mode="single", smooth=0.0))]
fn poly_fill(geo: &Geometry, mode: &str, smooth: f32) -> PyResult<Geometry> {
    let m = match mode {
        "fan" | "triangle_fan" => procgeo_sops::reshape::PolyFillMode::TriangleFan,
        _ => procgeo_sops::reshape::PolyFillMode::SinglePolygon,
    };
    let params = procgeo_sops::reshape::PolyFillParams { mode: m, smooth };
    let inner = procgeo_sops::reshape::PolyFillSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

// ---- Attribute SOPs ----

fn parse_attrib_class(s: &str) -> procgeo_core::AttribClass {
    serde_json::from_str(&format!("\"{}\"", s)).unwrap_or(procgeo_core::AttribClass::Point)
}

fn parse_attrib_type(s: &str) -> procgeo_core::AttribType {
    serde_json::from_str(&format!("\"{}\"", s)).unwrap_or(procgeo_core::AttribType::Float)
}

#[pyfunction]
#[pyo3(signature = (dest, source, attrib_name="attrib", class="Point", attrib_type="Float", max_samples=1, distance_threshold=f32::MAX))]
fn attrib_transfer(
    dest: &Geometry,
    source: &Geometry,
    attrib_name: &str,
    class: &str,
    attrib_type: &str,
    max_samples: u32,
    distance_threshold: f32,
) -> PyResult<Geometry> {
    let params = procgeo_sops::attributes::AttribTransferParams {
        attrib_name: attrib_name.to_string(),
        class: parse_attrib_class(class),
        attrib_type: parse_attrib_type(attrib_type),
        max_samples,
        distance_threshold,
    };
    let inner = procgeo_sops::attributes::AttribTransferSop
        .execute(&[&dest.inner, &source.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[pyfunction]
#[pyo3(signature = (dest, source=None, attrib_name="attrib", class="Point", attrib_type="Float", new_name=""))]
fn attrib_copy(
    dest: &Geometry,
    source: Option<&Geometry>,
    attrib_name: &str,
    class: &str,
    attrib_type: &str,
    new_name: &str,
) -> PyResult<Geometry> {
    let params = procgeo_sops::attributes::AttribCopyParams {
        attrib_name: attrib_name.to_string(),
        class: parse_attrib_class(class),
        attrib_type: parse_attrib_type(attrib_type),
        new_name: new_name.to_string(),
    };
    let inner = match source {
        Some(src) => procgeo_sops::attributes::AttribCopySop
            .execute(&[&dest.inner, &src.inner], &params)
            .map_err(sop_err)?,
        None => procgeo_sops::attributes::AttribCopySop
            .execute(&[&dest.inner], &params)
            .map_err(sop_err)?,
    };
    Ok(Geometry { inner })
}

#[pyfunction]
#[pyo3(signature = (geo, attrib_name="randomize", class="Point", attrib_type="Float", distribution="Uniform", operation="Set", seed=0, min_value=0.0, max_value=1.0, mean=0.0, stddev=1.0, value_a=0.0, value_b=1.0, probability=0.5, dimensions=1, global_scale=1.0))]
fn attrib_randomize(
    geo: &Geometry,
    attrib_name: &str,
    class: &str,
    attrib_type: &str,
    distribution: &str,
    operation: &str,
    seed: u64,
    min_value: f32,
    max_value: f32,
    mean: f32,
    stddev: f32,
    value_a: f32,
    value_b: f32,
    probability: f32,
    dimensions: u32,
    global_scale: f32,
) -> PyResult<Geometry> {
    let distribution = serde_json::from_str::<procgeo_sops::attributes::RandomDistribution>(
        &format!("\"{}\"", distribution),
    ).unwrap_or(procgeo_sops::attributes::RandomDistribution::Uniform);
    let operation = serde_json::from_str::<procgeo_sops::attributes::RandomOperation>(
        &format!("\"{}\"", operation),
    ).unwrap_or(procgeo_sops::attributes::RandomOperation::Set);
    let params = procgeo_sops::attributes::AttribRandomizeParams {
        attrib_name: attrib_name.to_string(),
        class: parse_attrib_class(class),
        attrib_type: parse_attrib_type(attrib_type),
        distribution,
        operation,
        seed,
        min_value,
        max_value,
        mean,
        stddev,
        value_a,
        value_b,
        probability,
        dimensions,
        global_scale,
    };
    let inner = procgeo_sops::attributes::AttribRandomizeSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[pyfunction]
#[pyo3(signature = (geo, attrib_name="attrib", class="Point", attrib_type="Float", order="Ascending", component=0))]
fn attrib_sort(
    geo: &Geometry,
    attrib_name: &str,
    class: &str,
    attrib_type: &str,
    order: &str,
    component: usize,
) -> PyResult<Geometry> {
    let order = serde_json::from_str::<procgeo_sops::attributes::AttribSortOrder>(
        &format!("\"{}\"", order),
    ).unwrap_or(procgeo_sops::attributes::AttribSortOrder::Ascending);
    let params = procgeo_sops::attributes::AttribSortParams {
        attrib_name: attrib_name.to_string(),
        class: parse_attrib_class(class),
        attrib_type: parse_attrib_type(attrib_type),
        order,
        component,
    };
    let inner = procgeo_sops::attributes::AttribSortSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[pyfunction]
#[pyo3(signature = (geo, attrib_name="attrib", attrib_type="Float", iterations=1, step_size=1.0))]
fn attrib_blur(
    geo: &Geometry,
    attrib_name: &str,
    attrib_type: &str,
    iterations: u32,
    step_size: f32,
) -> PyResult<Geometry> {
    let params = procgeo_sops::attributes::AttribBlurParams {
        attrib_name: attrib_name.to_string(),
        attrib_type: parse_attrib_type(attrib_type),
        iterations,
        step_size,
    };
    let inner = procgeo_sops::attributes::AttribBlurSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[pyfunction]
#[pyo3(signature = (geo, attrib_name="attrib", attrib_type="Float", boundary_group="", iterations=10, step_size=0.5))]
fn attrib_fill(
    geo: &Geometry,
    attrib_name: &str,
    attrib_type: &str,
    boundary_group: &str,
    iterations: u32,
    step_size: f32,
) -> PyResult<Geometry> {
    let params = procgeo_sops::attributes::AttribFillParams {
        attrib_name: attrib_name.to_string(),
        attrib_type: parse_attrib_type(attrib_type),
        boundary_group: boundary_group.to_string(),
        iterations,
        step_size,
    };
    let inner = procgeo_sops::attributes::AttribFillSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[pyfunction]
#[pyo3(signature = (geo, attrib_name="noise", noise_type="perlin", element_size=1.0, amplitude=1.0, seed=0, dimensions=1, fractal="none", octaves=8, lacunarity=2.0, roughness=0.5, gain=0.5, bias=0.5, offset_x=0.0, offset_y=0.0, offset_z=0.0))]
fn attrib_noise(
    geo: &Geometry,
    attrib_name: &str,
    noise_type: &str,
    element_size: f32,
    amplitude: f32,
    seed: u64,
    dimensions: u32,
    fractal: &str,
    octaves: u32,
    lacunarity: f32,
    roughness: f32,
    gain: f32,
    bias: f32,
    offset_x: f32,
    offset_y: f32,
    offset_z: f32,
) -> PyResult<Geometry> {
    let noise_type = match noise_type {
        "simplex" => procgeo_sops::attributes::NoiseType::Simplex,
        "worley" => procgeo_sops::attributes::NoiseType::Worley,
        "worleyF2F1" => procgeo_sops::attributes::NoiseType::WorleyF2F1,
        _ => procgeo_sops::attributes::NoiseType::Perlin,
    };
    let fractal = match fractal {
        "standard" => procgeo_sops::attributes::FractalType::Standard,
        "terrain" => procgeo_sops::attributes::FractalType::Terrain,
        _ => procgeo_sops::attributes::FractalType::None,
    };
    let params = procgeo_sops::attributes::AttribNoiseParams {
        attrib_name: attrib_name.to_string(),
        class: procgeo_core::AttribClass::Point,
        dimensions,
        noise_type,
        operation: procgeo_sops::attributes::NoiseOperation::Set,
        element_size,
        offset: [offset_x, offset_y, offset_z],
        seed,
        range: procgeo_sops::attributes::NoiseRange::Positive,
        amplitude,
        min_value: 0.0,
        max_value: 1.0,
        fractal,
        octaves,
        lacunarity,
        roughness,
        gain,
        bias,
    };
    let inner = procgeo_sops::attributes::AttribNoiseSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

// ---- SOP Registry — generic execute ----

static REGISTRY: OnceLock<procgeo_sops::SopRegistry> = OnceLock::new();

fn get_registry() -> &'static procgeo_sops::SopRegistry {
    REGISTRY.get_or_init(procgeo_sops::default_registry)
}

/// Execute any registered SOP by name with a JSON params string.
/// Uses Rust/snake_case field names for params (matching serde serialization).
#[pyfunction]
#[pyo3(signature = (name, geo, params_json="{}"))]
fn execute_sop(name: &str, geo: &Geometry, params_json: &str) -> PyResult<Geometry> {
    let registry = get_registry();
    let inner = registry
        .execute(name, &[&geo.inner], params_json)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

/// Execute a creation SOP (no input geometry required).
#[pyfunction]
#[pyo3(signature = (name, params_json="{}"))]
fn execute_sop_create(name: &str, params_json: &str) -> PyResult<Geometry> {
    let registry = get_registry();
    let inner = registry
        .execute(name, &[], params_json)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

/// List all registered SOP names.
#[pyfunction]
fn list_sops() -> Vec<String> {
    get_registry()
        .list()
        .into_iter()
        .map(|s| s.to_string())
        .collect()
}

// ---- I/O ----

#[pyfunction]
fn write_obj(geo: &Geometry, path: &str) -> PyResult<()> {
    procgeo_io::write_file(&geo.inner, std::path::Path::new(path))
        .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))
}

#[pyfunction]
fn write_glb(geo: &Geometry, path: &str) -> PyResult<()> {
    procgeo_io::write_file(&geo.inner, std::path::Path::new(path))
        .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))
}

/// The procgeo Python module.
#[pymodule]
fn procgeo(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Geometry>()?;
    // Creation
    m.add_function(wrap_pyfunction!(create_box, m)?)?;
    m.add_function(wrap_pyfunction!(create_grid, m)?)?;
    m.add_function(wrap_pyfunction!(create_sphere, m)?)?;
    m.add_function(wrap_pyfunction!(create_line, m)?)?;
    m.add_function(wrap_pyfunction!(create_circle, m)?)?;
    m.add_function(wrap_pyfunction!(create_tube, m)?)?;
    m.add_function(wrap_pyfunction!(create_torus, m)?)?;
    m.add_function(wrap_pyfunction!(revolve, m)?)?;
    m.add_function(wrap_pyfunction!(create_metaball, m)?)?;
    // Manipulation
    m.add_function(wrap_pyfunction!(transform, m)?)?;
    m.add_function(wrap_pyfunction!(compute_normals, m)?)?;
    m.add_function(wrap_pyfunction!(merge, m)?)?;
    m.add_function(wrap_pyfunction!(subdivide, m)?)?;
    m.add_function(wrap_pyfunction!(scatter, m)?)?;
    m.add_function(wrap_pyfunction!(copy_to_points, m)?)?;
    m.add_function(wrap_pyfunction!(poly_extrude, m)?)?;
    m.add_function(wrap_pyfunction!(smooth, m)?)?;
    m.add_function(wrap_pyfunction!(clip, m)?)?;
    m.add_function(wrap_pyfunction!(reverse, m)?)?;
    m.add_function(wrap_pyfunction!(resample, m)?)?;
    m.add_function(wrap_pyfunction!(fuse, m)?)?;
    m.add_function(wrap_pyfunction!(connectivity, m)?)?;
    m.add_function(wrap_pyfunction!(color, m)?)?;
    m.add_function(wrap_pyfunction!(enumerate_attrib, m)?)?;
    m.add_function(wrap_pyfunction!(measure_area, m)?)?;
    // Reshape SOPs
    m.add_function(wrap_pyfunction!(poly_bevel, m)?)?;
    m.add_function(wrap_pyfunction!(poly_wire, m)?)?;
    m.add_function(wrap_pyfunction!(poly_reduce, m)?)?;
    m.add_function(wrap_pyfunction!(poly_fill, m)?)?;
    // Attribute SOPs
    m.add_function(wrap_pyfunction!(attrib_transfer, m)?)?;
    m.add_function(wrap_pyfunction!(attrib_copy, m)?)?;
    m.add_function(wrap_pyfunction!(attrib_randomize, m)?)?;
    m.add_function(wrap_pyfunction!(attrib_sort, m)?)?;
    m.add_function(wrap_pyfunction!(attrib_blur, m)?)?;
    m.add_function(wrap_pyfunction!(attrib_fill, m)?)?;
    m.add_function(wrap_pyfunction!(attrib_noise, m)?)?;
    // SOP Registry
    m.add_function(wrap_pyfunction!(execute_sop, m)?)?;
    m.add_function(wrap_pyfunction!(execute_sop_create, m)?)?;
    m.add_function(wrap_pyfunction!(list_sops, m)?)?;
    // I/O
    m.add_function(wrap_pyfunction!(write_obj, m)?)?;
    m.add_function(wrap_pyfunction!(write_glb, m)?)?;
    Ok(())
}
