use std::sync::OnceLock;

use procgeo_sops::Sop;
use pyo3::prelude::*;

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

    fn add_point(&mut self, x: f32, y: f32, z: f32) -> usize {
        self.inner.add_point(glam::Vec3::new(x, y, z)).index()
    }

    fn set_point_pos(&mut self, index: usize, x: f32, y: f32, z: f32) {
        self.inner.set_point_pos(
            procgeo_core::PointHandle::from_index(index),
            glam::Vec3::new(x, y, z),
        );
    }

    fn add_face(&mut self, point_indices: Vec<usize>) -> usize {
        let handles: Vec<procgeo_core::PointHandle> = point_indices
            .iter()
            .map(|&i| procgeo_core::PointHandle::from_index(i))
            .collect();
        self.inner.add_face(&handles).index()
    }

    fn add_polyline(&mut self, point_indices: Vec<usize>) -> usize {
        let handles: Vec<procgeo_core::PointHandle> = point_indices
            .iter()
            .map(|&i| procgeo_core::PointHandle::from_index(i))
            .collect();
        self.inner.add_polyline(&handles).index()
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

    // -- Attribute introspection (spreadsheet / debugging) --

    fn attrib_names(&self, class: &str) -> Vec<String> {
        self.inner
            .attrib_names(py_parse_class(class))
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    fn attrib_type(&self, class: &str, name: &str) -> Option<String> {
        self.inner
            .attrib_type(py_parse_class(class), name)
            .map(|t| format!("{t:?}"))
    }

    fn attrib_size(&self, class: &str, name: &str) -> Option<usize> {
        self.inner.attrib_size(py_parse_class(class), name)
    }

    fn attrib_data(&self, class: &str, name: &str) -> Option<Vec<f64>> {
        self.inner.attrib_data_f64(py_parse_class(class), name)
    }

    fn attrib_data_string(&self, class: &str, name: &str) -> Option<Vec<String>> {
        self.inner.attrib_data_string(py_parse_class(class), name)
    }

    fn prim_point_indices(&self, prim_index: usize) -> Vec<usize> {
        let ph = procgeo_core::PrimHandle::from_index(prim_index);
        self.inner
            .prim_points(ph)
            .iter()
            .map(|p| p.index())
            .collect()
    }

    fn prim_is_closed(&self, prim_index: usize) -> bool {
        let ph = procgeo_core::PrimHandle::from_index(prim_index);
        match self.inner.prim(ph) {
            procgeo_core::Primitive::Polygon(poly) => {
                poly.poly_type == procgeo_core::PolyType::Closed
            }
        }
    }

    fn prim_vertex_count(&self, prim_index: usize) -> usize {
        let ph = procgeo_core::PrimHandle::from_index(prim_index);
        self.inner.prim_vertices(ph).len()
    }

    fn vertex_point(&self, vertex_index: usize) -> usize {
        self.inner
            .vertex_point(procgeo_core::VertexHandle::from_index(vertex_index))
            .index()
    }
}

fn py_parse_class(class: &str) -> procgeo_core::AttribClass {
    match class {
        "vertex" | "Vertex" => procgeo_core::AttribClass::Vertex,
        "primitive" | "Primitive" | "prim" => procgeo_core::AttribClass::Primitive,
        "detail" | "Detail" => procgeo_core::AttribClass::Detail,
        _ => procgeo_core::AttribClass::Point,
    }
}

fn py_parse_normal_group_type(group_type: &str) -> procgeo_sops::normals::NormalGroupType {
    match group_type {
        "points" | "point" | "Points" => procgeo_sops::normals::NormalGroupType::Points,
        "vertices" | "vertex" | "Vertex" => procgeo_sops::normals::NormalGroupType::Vertices,
        "primitives" | "primitive" | "prim" | "Primitive" => {
            procgeo_sops::normals::NormalGroupType::Primitives
        }
        "edges" | "edge" | "Edge" => procgeo_sops::normals::NormalGroupType::Edges,
        _ => procgeo_sops::normals::NormalGroupType::GuessFromGroup,
    }
}

fn py_parse_normal_target(target: &str) -> procgeo_sops::normals::NormalTarget {
    match target {
        "vertices" | "vertex" | "Vertex" => procgeo_sops::normals::NormalTarget::Vertices,
        "primitives" | "primitive" | "prim" | "Primitive" => {
            procgeo_sops::normals::NormalTarget::Primitives
        }
        "detail" | "Detail" => procgeo_sops::normals::NormalTarget::Detail,
        _ => procgeo_sops::normals::NormalTarget::Points,
    }
}

fn py_parse_normal_weighting_method(method: &str) -> procgeo_sops::normals::NormalWeightingMethod {
    match method {
        "each_vertex_equally" | "eachVertexEqually" | "equal" => {
            procgeo_sops::normals::NormalWeightingMethod::EachVertexEqually
        }
        "face_area" | "faceArea" | "area" => {
            procgeo_sops::normals::NormalWeightingMethod::ByFaceArea
        }
        _ => procgeo_sops::normals::NormalWeightingMethod::ByVertexAngle,
    }
}

fn py_parse_displace_direction(direction: &str) -> procgeo_sops::deform::DisplaceDirection {
    match direction {
        "x" | "X" => procgeo_sops::deform::DisplaceDirection::X,
        "y" | "Y" => procgeo_sops::deform::DisplaceDirection::Y,
        "z" | "Z" => procgeo_sops::deform::DisplaceDirection::Z,
        "rgb_to_xyz" | "rgbToXyz" | "rgbtoxyz" => procgeo_sops::deform::DisplaceDirection::RGBToXYZ,
        "custom_vector" | "customVector" | "custom" => {
            procgeo_sops::deform::DisplaceDirection::CustomVector
        }
        _ => procgeo_sops::deform::DisplaceDirection::Normal,
    }
}

fn py_parse_displace_coordinates(coords: &str) -> procgeo_sops::deform::DisplaceCoordinates {
    match coords {
        "uv" | "UV" => procgeo_sops::deform::DisplaceCoordinates::UV,
        "bounding_box" | "boundingBox" | "bbox" => {
            procgeo_sops::deform::DisplaceCoordinates::BoundingBox
        }
        "position" | "local" => procgeo_sops::deform::DisplaceCoordinates::Position,
        _ => procgeo_sops::deform::DisplaceCoordinates::Auto,
    }
}

fn py_parse_displace_projection(projection: &str) -> procgeo_sops::deform::DisplaceProjection {
    match projection {
        "xy" | "XY" => procgeo_sops::deform::DisplaceProjection::XY,
        "yz" | "YZ" => procgeo_sops::deform::DisplaceProjection::YZ,
        _ => procgeo_sops::deform::DisplaceProjection::XZ,
    }
}

fn py_parse_displace_channel(channel: &str) -> procgeo_sops::deform::DisplaceSampleChannel {
    match channel {
        "red" | "r" => procgeo_sops::deform::DisplaceSampleChannel::Red,
        "green" | "g" => procgeo_sops::deform::DisplaceSampleChannel::Green,
        "blue" | "b" => procgeo_sops::deform::DisplaceSampleChannel::Blue,
        "alpha" | "a" => procgeo_sops::deform::DisplaceSampleChannel::Alpha,
        "average" | "avg" => procgeo_sops::deform::DisplaceSampleChannel::Average,
        _ => procgeo_sops::deform::DisplaceSampleChannel::Luminance,
    }
}

fn py_parse_displace_sampler(sampler: &str) -> procgeo_sops::deform::DisplaceSampler {
    match sampler {
        "nearest" => procgeo_sops::deform::DisplaceSampler::Nearest,
        _ => procgeo_sops::deform::DisplaceSampler::Bilinear,
    }
}

fn py_parse_displace_wrap(wrap: &str) -> procgeo_sops::deform::DisplaceWrapMode {
    match wrap {
        "clamp" => procgeo_sops::deform::DisplaceWrapMode::Clamp,
        _ => procgeo_sops::deform::DisplaceWrapMode::Repeat,
    }
}

fn py_parse_displace_noise_type(noise_type: &str) -> procgeo_sops::deform::DisplaceNoiseType {
    match noise_type {
        "simplex" => procgeo_sops::deform::DisplaceNoiseType::Simplex,
        "worley" => procgeo_sops::deform::DisplaceNoiseType::Worley,
        "worley_f2f1" | "worleyF2F1" => procgeo_sops::deform::DisplaceNoiseType::WorleyF2F1,
        _ => procgeo_sops::deform::DisplaceNoiseType::Perlin,
    }
}

fn py_parse_displace_noise_fractal(fractal: &str) -> procgeo_sops::deform::DisplaceNoiseFractal {
    match fractal {
        "standard" | "fbm" => procgeo_sops::deform::DisplaceNoiseFractal::Standard,
        "terrain" | "ridged" => procgeo_sops::deform::DisplaceNoiseFractal::Terrain,
        _ => procgeo_sops::deform::DisplaceNoiseFractal::None,
    }
}

fn sop_err(e: procgeo_sops::SopError) -> PyErr {
    pyo3::exceptions::PyRuntimeError::new_err(e.to_string())
}

// ---- Creation SOPs ----

#[pyfunction]
#[pyo3(signature = (source=None, points=None, polygons=None, polylines=None))]
fn add(
    source: Option<&Geometry>,
    points: Option<Vec<(f32, f32, f32)>>,
    polygons: Option<Vec<Vec<usize>>>,
    polylines: Option<Vec<Vec<usize>>>,
) -> PyResult<Geometry> {
    let params = procgeo_sops::creation::AddParams {
        points: points
            .unwrap_or_default()
            .into_iter()
            .map(|(x, y, z)| [x, y, z])
            .collect(),
        polygons: polygons.unwrap_or_default(),
        polylines: polylines.unwrap_or_default(),
    };

    let inner = match source {
        Some(src) => procgeo_sops::creation::AddSop
            .execute(&[&src.inner], &params)
            .map_err(sop_err)?,
        None => procgeo_sops::creation::AddSop
            .execute(&[], &params)
            .map_err(sop_err)?,
    };

    Ok(Geometry { inner })
}

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
#[pyo3(signature = (start_radius=0.0, end_radius=1.0, height=0.0, turns=3.0, points=96, center_x=0.0, center_y=0.0, center_z=0.0))]
fn create_spiral(
    start_radius: f32,
    end_radius: f32,
    height: f32,
    turns: f32,
    points: u32,
    center_x: f32,
    center_y: f32,
    center_z: f32,
) -> PyResult<Geometry> {
    let params = procgeo_sops::creation::SpiralParams {
        start_radius,
        end_radius,
        height,
        turns,
        points,
        center: glam::Vec3::new(center_x, center_y, center_z),
    };
    let inner = procgeo_sops::creation::SpiralSop
        .execute(&[], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[pyfunction]
#[pyo3(signature = (radius=0.5, height=1.0, turns=3.0, points=96, center_x=0.0, center_y=0.0, center_z=0.0))]
fn create_helix(
    radius: f32,
    height: f32,
    turns: f32,
    points: u32,
    center_x: f32,
    center_y: f32,
    center_z: f32,
) -> PyResult<Geometry> {
    let params = procgeo_sops::creation::HelixParams {
        radius,
        height,
        turns,
        points,
        center: glam::Vec3::new(center_x, center_y, center_z),
    };
    let inner = procgeo_sops::creation::HelixSop
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
#[pyo3(signature = (radius=0.5, subdivisions=2, center_x=0.0, center_y=0.0, center_z=0.0))]
fn create_icosphere(
    radius: f32,
    subdivisions: u32,
    center_x: f32,
    center_y: f32,
    center_z: f32,
) -> PyResult<Geometry> {
    let params = procgeo_sops::creation::IcosphereParams {
        radius,
        center: glam::Vec3::new(center_x, center_y, center_z),
        subdivisions,
    };
    let inner = procgeo_sops::creation::IcosphereSop
        .execute(&[], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[pyfunction]
#[pyo3(signature = (resolution=6, size_x=1.0, size_y=1.0, size_z=1.0, center_x=0.0, center_y=0.0, center_z=0.0))]
fn create_teapot(
    resolution: u32,
    size_x: f32,
    size_y: f32,
    size_z: f32,
    center_x: f32,
    center_y: f32,
    center_z: f32,
) -> PyResult<Geometry> {
    let params = procgeo_sops::creation::TeapotParams {
        size: glam::Vec3::new(size_x, size_y, size_z),
        center: glam::Vec3::new(center_x, center_y, center_z),
        resolution,
    };
    let inner = procgeo_sops::creation::TeapotSop
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
#[pyo3(signature = (
    geo,
    group="",
    group_type="guess",
    override_normal="N",
    compute_normals=true,
    add_normals_to="points",
    cusp_angle=60.0,
    weighting_method="vertex_angle",
    keep_original_zero=false,
    make_unit_length=false,
    reverse_normals=false
))]
fn compute_normals(
    geo: &Geometry,
    group: &str,
    group_type: &str,
    override_normal: &str,
    compute_normals: bool,
    add_normals_to: &str,
    cusp_angle: f32,
    weighting_method: &str,
    keep_original_zero: bool,
    make_unit_length: bool,
    reverse_normals: bool,
) -> PyResult<Geometry> {
    let params = procgeo_sops::normals::NormalParams {
        group: group.to_string(),
        group_type: py_parse_normal_group_type(group_type),
        override_normal: override_normal.to_string(),
        compute_normals,
        add_normals_to: py_parse_normal_target(add_normals_to),
        cusp_angle,
        weighting_method: py_parse_normal_weighting_method(weighting_method),
        keep_original_zero,
        make_unit_length,
        reverse_normals,
    };
    let inner = procgeo_sops::normals::NormalSop
        .execute(&[&geo.inner], &params)
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
#[pyo3(signature = (geo, origin_x=0.0, origin_y=0.0, origin_z=0.0, normal_x=0.0, normal_y=1.0, normal_z=0.0, keep_above=true, create_cap=false))]
fn clip(
    geo: &Geometry,
    origin_x: f32,
    origin_y: f32,
    origin_z: f32,
    normal_x: f32,
    normal_y: f32,
    normal_z: f32,
    keep_above: bool,
    create_cap: bool,
) -> PyResult<Geometry> {
    let params = procgeo_sops::reshape::ClipParams {
        origin: glam::Vec3::new(origin_x, origin_y, origin_z),
        normal: glam::Vec3::new(normal_x, normal_y, normal_z),
        keep_above,
        create_cap,
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

#[pyfunction]
#[pyo3(signature = (geo, seed=0))]
fn sort(geo: &Geometry, seed: u64) -> PyResult<Geometry> {
    let params = procgeo_sops::topology::SortParams {
        seed,
        ..Default::default()
    };
    let inner = procgeo_sops::topology::SortSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[pyfunction]
#[pyo3(signature = (geo, points, cut_plane_offset=0.0, create_inside_faces=true))]
fn voronoi_fracture(
    geo: &Geometry,
    points: &Geometry,
    cut_plane_offset: f32,
    create_inside_faces: bool,
) -> PyResult<Geometry> {
    let params = procgeo_sops::voronoi::VoronoiFractureParams {
        cut_plane_offset,
        create_inside_faces,
    };
    let inner = procgeo_sops::voronoi::VoronoiFractureSop
        .execute(&[&geo.inner, &points.inner], &params)
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
fn poly_reduce(
    geo: &Geometry,
    target_percent: f32,
    preserve_boundaries: bool,
) -> PyResult<Geometry> {
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

#[pyfunction]
#[pyo3(signature = (geo, target_mode="face_count", target_count=1000, target_edge_length=0.1, seed=None, mode="intrinsic"))]
fn quad_remesh(
    geo: &Geometry,
    target_mode: &str,
    target_count: u32,
    target_edge_length: f64,
    seed: Option<u64>,
    mode: &str,
) -> PyResult<Geometry> {
    let tm = match target_mode {
        "vertex_count" => procgeo_sops::reshape::QuadRemeshTarget::VertexCount,
        "edge_length" => procgeo_sops::reshape::QuadRemeshTarget::EdgeLength,
        _ => procgeo_sops::reshape::QuadRemeshTarget::FaceCount,
    };
    let m = match mode {
        "extrinsic" => procgeo_sops::reshape::QuadRemeshMode::Extrinsic,
        _ => procgeo_sops::reshape::QuadRemeshMode::Intrinsic,
    };
    let params = procgeo_sops::reshape::QuadRemeshParams {
        target_mode: tm,
        target_count,
        target_edge_length,
        seed,
        mode: m,
    };
    let inner = procgeo_sops::reshape::QuadRemeshSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

// ---- Delete SOPs ----

#[pyfunction]
#[pyo3(signature = (geo, group_name="", entity="primitives", negate=false))]
fn blast(geo: &Geometry, group_name: &str, entity: &str, negate: bool) -> PyResult<Geometry> {
    let entity = match entity {
        "points" => procgeo_sops::delete::BlastEntity::Points,
        _ => procgeo_sops::delete::BlastEntity::Primitives,
    };
    let params = procgeo_sops::delete::BlastParams {
        group_name: group_name.to_string(),
        entity,
        negate,
    };
    let inner = procgeo_sops::delete::BlastSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[pyfunction]
#[pyo3(signature = (geo, entity="primitives", range_start=0, range_end=0))]
fn delete_geo(
    geo: &Geometry,
    entity: &str,
    range_start: usize,
    range_end: usize,
) -> PyResult<Geometry> {
    let entity = match entity {
        "points" => procgeo_sops::delete::DeleteEntity::Points,
        _ => procgeo_sops::delete::DeleteEntity::Primitives,
    };
    let params = procgeo_sops::delete::DeleteParams {
        entity,
        range_start,
        range_end,
    };
    let inner = procgeo_sops::delete::DeleteSop
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
    )
    .unwrap_or(procgeo_sops::attributes::RandomDistribution::Uniform);
    let operation = serde_json::from_str::<procgeo_sops::attributes::RandomOperation>(&format!(
        "\"{}\"",
        operation
    ))
    .unwrap_or(procgeo_sops::attributes::RandomOperation::Set);
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
    let order = serde_json::from_str::<procgeo_sops::attributes::AttribSortOrder>(&format!(
        "\"{}\"",
        order
    ))
    .unwrap_or(procgeo_sops::attributes::AttribSortOrder::Ascending);
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

#[pyfunction]
#[pyo3(signature = (geo, name="attrib1", class="Point", attrib_type="Float", value_int=0, value_float=0.0, value_vector3_x=0.0, value_vector3_y=0.0, value_vector3_z=0.0, value_string=""))]
fn attrib_create(
    geo: &Geometry,
    name: &str,
    class: &str,
    attrib_type: &str,
    value_int: i32,
    value_float: f32,
    value_vector3_x: f32,
    value_vector3_y: f32,
    value_vector3_z: f32,
    value_string: &str,
) -> PyResult<Geometry> {
    let params = procgeo_sops::attributes::AttribCreateParams {
        name: name.to_string(),
        class: parse_attrib_class(class),
        attrib_type: parse_attrib_type(attrib_type),
        value_int,
        value_float,
        value_vector3: [value_vector3_x, value_vector3_y, value_vector3_z],
        value_string: value_string.to_string(),
        qualifier: Default::default(),
    };
    let inner = procgeo_sops::attributes::AttribCreateSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[pyfunction]
#[pyo3(signature = (geo, name="attrib1", class="Point"))]
fn attrib_delete(geo: &Geometry, name: &str, class: &str) -> PyResult<Geometry> {
    let params = procgeo_sops::attributes::AttribDeleteParams {
        name: name.to_string(),
        class: parse_attrib_class(class),
    };
    let inner = procgeo_sops::attributes::AttribDeleteSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[pyfunction]
#[pyo3(signature = (geo, from_name="attrib1", to_name="attrib2", class="Point"))]
fn attrib_rename(
    geo: &Geometry,
    from_name: &str,
    to_name: &str,
    class: &str,
) -> PyResult<Geometry> {
    let params = procgeo_sops::attributes::AttribRenameParams {
        from_name: from_name.to_string(),
        to_name: to_name.to_string(),
        class: parse_attrib_class(class),
    };
    let inner = procgeo_sops::attributes::AttribRenameSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[pyfunction]
#[pyo3(signature = (geo, name="attrib", from_class="Point", to_class="Primitive", method="average", delete_original=true))]
fn attrib_promote(
    geo: &Geometry,
    name: &str,
    from_class: &str,
    to_class: &str,
    method: &str,
    delete_original: bool,
) -> PyResult<Geometry> {
    let method = match method {
        "first" => procgeo_sops::attributes::PromoteMethod::First,
        "last" => procgeo_sops::attributes::PromoteMethod::Last,
        "min" => procgeo_sops::attributes::PromoteMethod::Min,
        "max" => procgeo_sops::attributes::PromoteMethod::Max,
        _ => procgeo_sops::attributes::PromoteMethod::Average,
    };
    let params = procgeo_sops::attributes::AttribPromoteParams {
        name: name.to_string(),
        from_class: parse_attrib_class(from_class),
        to_class: parse_attrib_class(to_class),
        method,
        delete_original,
    };
    let inner = procgeo_sops::attributes::AttribPromoteSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

// ---- Group SOPs ----

#[pyfunction]
#[pyo3(signature = (geo, name="group1", group_type="points", mode="range", range_start=0, range_end=usize::MAX, bbox_min_x=f32::NEG_INFINITY, bbox_min_y=f32::NEG_INFINITY, bbox_min_z=f32::NEG_INFINITY, bbox_max_x=f32::INFINITY, bbox_max_y=f32::INFINITY, bbox_max_z=f32::INFINITY, normal_dir_x=0.0, normal_dir_y=1.0, normal_dir_z=0.0, normal_angle=45.0))]
fn group_create(
    geo: &Geometry,
    name: &str,
    group_type: &str,
    mode: &str,
    range_start: usize,
    range_end: usize,
    bbox_min_x: f32,
    bbox_min_y: f32,
    bbox_min_z: f32,
    bbox_max_x: f32,
    bbox_max_y: f32,
    bbox_max_z: f32,
    normal_dir_x: f32,
    normal_dir_y: f32,
    normal_dir_z: f32,
    normal_angle: f32,
) -> PyResult<Geometry> {
    let group_type = match group_type {
        "primitives" | "prims" => procgeo_sops::groups::GroupType::Primitives,
        _ => procgeo_sops::groups::GroupType::Points,
    };
    let mode = match mode {
        "bounding_box" | "bbox" => procgeo_sops::groups::GroupCreateMode::BoundingBox,
        "normal" => procgeo_sops::groups::GroupCreateMode::Normal,
        _ => procgeo_sops::groups::GroupCreateMode::Range,
    };
    let params = procgeo_sops::groups::GroupCreateParams {
        name: name.to_string(),
        group_type,
        mode,
        range_start,
        range_end,
        bbox_min: glam::Vec3::new(bbox_min_x, bbox_min_y, bbox_min_z),
        bbox_max: glam::Vec3::new(bbox_max_x, bbox_max_y, bbox_max_z),
        normal_direction: glam::Vec3::new(normal_dir_x, normal_dir_y, normal_dir_z),
        normal_angle,
    };
    let inner = procgeo_sops::groups::GroupCreateSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[pyfunction]
#[pyo3(signature = (geo, name_a="group_a", name_b="group_b", result="group_result", operation="union", group_type="points"))]
fn group_combine(
    geo: &Geometry,
    name_a: &str,
    name_b: &str,
    result: &str,
    operation: &str,
    group_type: &str,
) -> PyResult<Geometry> {
    let operation = match operation {
        "intersect" => procgeo_sops::groups::GroupBooleanOp::Intersect,
        "subtract" => procgeo_sops::groups::GroupBooleanOp::Subtract,
        _ => procgeo_sops::groups::GroupBooleanOp::Union,
    };
    let group_type = match group_type {
        "primitives" | "prims" => procgeo_sops::groups::GroupType::Primitives,
        _ => procgeo_sops::groups::GroupType::Points,
    };
    let params = procgeo_sops::groups::GroupCombineParams {
        name_a: name_a.to_string(),
        name_b: name_b.to_string(),
        result: result.to_string(),
        operation,
        group_type,
    };
    let inner = procgeo_sops::groups::GroupCombineSop
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
    let inner = registry.execute(name, &[], params_json).map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[pyfunction]
#[pyo3(signature = (geo, sharp_angle=35.0, curvature_weight=0.3, smooth_iterations=20, scale_factor=1.0, alpha=0.02, post_smooth_iterations=30))]
fn quad_wild(
    geo: &Geometry,
    sharp_angle: f32,
    curvature_weight: f32,
    smooth_iterations: u32,
    scale_factor: f32,
    alpha: f32,
    post_smooth_iterations: u32,
) -> PyResult<Geometry> {
    let params = procgeo_sops::quadwild::QuadWildParams {
        sharp_angle,
        curvature_weight,
        smooth_iterations,
        scale_factor,
        alpha,
        post_smooth_iterations,
    };
    let inner = procgeo_sops::quadwild::QuadWildSop
        .execute(&[&geo.inner], &params)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("{e}")))?;
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

// ---------------------------------------------------------------------------
// COP Registry
// ---------------------------------------------------------------------------

use procgeo_cops::context::GpuContext as PyCopGpuContext;
use procgeo_cops::registry::CopRegistry as PyCopRegistry;

static COP_REGISTRY: OnceLock<PyCopRegistry> = OnceLock::new();
static PY_GPU_CONTEXT: OnceLock<std::sync::Arc<PyCopGpuContext>> = OnceLock::new();

fn get_cop_registry() -> &'static PyCopRegistry {
    COP_REGISTRY.get_or_init(procgeo_cops::registry::default_cop_registry)
}

fn get_py_gpu_context() -> PyResult<std::sync::Arc<PyCopGpuContext>> {
    if let Some(ctx) = PY_GPU_CONTEXT.get() {
        return Ok(std::sync::Arc::clone(ctx));
    }
    let ctx = PyCopGpuContext::new_blocking()
        .map(std::sync::Arc::new)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("GPU init: {e}")))?;
    let _ = PY_GPU_CONTEXT.set(std::sync::Arc::clone(&ctx));
    Ok(ctx)
}

fn cop_err(e: procgeo_cops::CopError) -> PyErr {
    pyo3::exceptions::PyRuntimeError::new_err(format!("{e}"))
}

#[pyclass]
struct CopImage {
    inner: procgeo_cops::image::Image,
}

#[pymethods]
impl CopImage {
    #[getter]
    fn width(&self) -> u32 {
        self.inner.width()
    }
    #[getter]
    fn height(&self) -> u32 {
        self.inner.height()
    }
    fn to_list(&self) -> PyResult<Vec<f32>> {
        self.inner.to_cpu().map_err(cop_err)
    }
}

#[pyfunction]
#[pyo3(signature = (name, params_json="{}"))]
fn execute_cop_create(name: &str, params_json: &str) -> PyResult<CopImage> {
    let ctx = get_py_gpu_context()?;
    let registry = get_cop_registry();
    let inner = registry
        .execute(name, &ctx, &[], params_json)
        .map_err(cop_err)?;
    Ok(CopImage { inner })
}

#[pyfunction]
#[pyo3(signature = (name, image, params_json="{}"))]
fn execute_cop(name: &str, image: &CopImage, params_json: &str) -> PyResult<CopImage> {
    let ctx = get_py_gpu_context()?;
    let registry = get_cop_registry();
    let inner = registry
        .execute(name, &ctx, &[&image.inner], params_json)
        .map_err(cop_err)?;
    Ok(CopImage { inner })
}

#[pyfunction]
#[pyo3(signature = (name, image_a, image_b, params_json="{}"))]
fn execute_cop_composite(
    name: &str,
    image_a: &CopImage,
    image_b: &CopImage,
    params_json: &str,
) -> PyResult<CopImage> {
    let ctx = get_py_gpu_context()?;
    let registry = get_cop_registry();
    let inner = registry
        .execute(name, &ctx, &[&image_a.inner, &image_b.inner], params_json)
        .map_err(cop_err)?;
    Ok(CopImage { inner })
}

#[pyfunction]
fn list_cops() -> Vec<String> {
    get_cop_registry()
        .list()
        .into_iter()
        .map(|s| s.to_string())
        .collect()
}

#[pyfunction]
#[pyo3(signature = (image, path))]
fn save_cop_image(image: &CopImage, path: &str) -> PyResult<()> {
    let params = procgeo_cops::io::SaveImageParams {
        path: path.to_string(),
        ..Default::default()
    };
    procgeo_cops::io::save_image(&image.inner, &params).map_err(cop_err)
}

// ---- Deform SOPs ----

#[pyfunction]
#[pyo3(signature = (
    geo,
    group=None,
    enable_deformation=true,
    limit_to_capture_region=true,
    deform_both_directions=false,
    bend_enable=false,
    bend_mode="angle",
    bend_angle=0.0,
    twist_enable=false,
    twist_angle=0.0,
    length_scale_enable=false,
    length_scale=1.0,
    preserve_volume=false,
    taper_enable=false,
    taper_value=1.0,
    squish=1.0,
    squish_pivot=0.5,
    taper_mode="linear",
    up_vector_x=0.0,
    up_vector_y=1.0,
    up_vector_z=0.0,
    up_vector_angle=0.0,
    capture_origin_x=0.0,
    capture_origin_y=0.0,
    capture_origin_z=0.0,
    capture_direction_x=0.0,
    capture_direction_y=1.0,
    capture_direction_z=0.0,
    capture_length=1.0,
    mask_attrib=None,
    output_attrib=None,
))]
#[allow(clippy::too_many_arguments)]
fn bend(
    geo: &Geometry,
    group: Option<String>,
    enable_deformation: bool,
    limit_to_capture_region: bool,
    deform_both_directions: bool,
    bend_enable: bool,
    bend_mode: &str,
    bend_angle: f32,
    twist_enable: bool,
    twist_angle: f32,
    length_scale_enable: bool,
    length_scale: f32,
    preserve_volume: bool,
    taper_enable: bool,
    taper_value: f32,
    squish: f32,
    squish_pivot: f32,
    taper_mode: &str,
    up_vector_x: f32,
    up_vector_y: f32,
    up_vector_z: f32,
    up_vector_angle: f32,
    capture_origin_x: f32,
    capture_origin_y: f32,
    capture_origin_z: f32,
    capture_direction_x: f32,
    capture_direction_y: f32,
    capture_direction_z: f32,
    capture_length: f32,
    mask_attrib: Option<String>,
    output_attrib: Option<String>,
) -> PyResult<Geometry> {
    let bm = match bend_mode {
        "direction" => procgeo_sops::deform::BendMode::Direction,
        _ => procgeo_sops::deform::BendMode::Angle,
    };
    let tm = match taper_mode {
        "smooth" => procgeo_sops::deform::TaperMode::Smooth,
        _ => procgeo_sops::deform::TaperMode::Linear,
    };
    let params = procgeo_sops::deform::BendParams {
        group,
        mask_attrib,
        enable_deformation,
        limit_to_capture_region,
        deform_both_directions,
        bend_enable,
        bend_mode: bm,
        bend_angle,
        bend_goal_direction: glam::Vec3::Z,
        twist_enable,
        twist_angle,
        twist_continuous_both: false,
        length_scale_enable,
        length_scale,
        preserve_volume,
        taper_enable,
        taper_along: [true, true],
        taper_mode: tm,
        taper_value,
        squish,
        squish_pivot,
        taper_ramp_enable: false,
        taper_ramp: vec![(0.0, 0.5), (1.0, 0.5)],
        up_vector: glam::Vec3::new(up_vector_x, up_vector_y, up_vector_z),
        up_vector_angle,
        capture_origin: glam::Vec3::new(capture_origin_x, capture_origin_y, capture_origin_z),
        capture_direction: glam::Vec3::new(
            capture_direction_x,
            capture_direction_y,
            capture_direction_z,
        ),
        capture_length,
        output_attrib,
        attribs_to_transform: String::from("*"),
        recompute_normals: true,
        preserve_normal_length: false,
    };
    let inner = procgeo_sops::deform::BendSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[pyfunction]
#[pyo3(signature = (
    geo,
    image=None,
    strength=1.0,
    midlevel=0.5,
    direction="normal",
    coordinates="auto",
    projection="xz",
    uv_attrib="uv",
    normal_attrib="N",
    sample_channel="luminance",
    sampler="bilinear",
    wrap="repeat",
    coord_scale_u=1.0,
    coord_scale_v=1.0,
    coord_offset_u=0.0,
    coord_offset_v=0.0,
    custom_vector_x=0.0,
    custom_vector_y=1.0,
    custom_vector_z=0.0,
    noise_type=None,
    noise_scale_x=1.0,
    noise_scale_y=1.0,
    noise_scale_z=1.0,
    noise_offset_x=0.0,
    noise_offset_y=0.0,
    noise_offset_z=0.0,
    noise_seed=0,
    noise_octaves=4,
    noise_lacunarity=2.0,
    noise_roughness=0.5,
    noise_fractal="none",
))]
#[allow(clippy::too_many_arguments)]
fn displace(
    geo: &Geometry,
    image: Option<&CopImage>,
    strength: f32,
    midlevel: f32,
    direction: &str,
    coordinates: &str,
    projection: &str,
    uv_attrib: &str,
    normal_attrib: &str,
    sample_channel: &str,
    sampler: &str,
    wrap: &str,
    coord_scale_u: f32,
    coord_scale_v: f32,
    coord_offset_u: f32,
    coord_offset_v: f32,
    custom_vector_x: f32,
    custom_vector_y: f32,
    custom_vector_z: f32,
    noise_type: Option<&str>,
    noise_scale_x: f32,
    noise_scale_y: f32,
    noise_scale_z: f32,
    noise_offset_x: f32,
    noise_offset_y: f32,
    noise_offset_z: f32,
    noise_seed: u64,
    noise_octaves: u32,
    noise_lacunarity: f32,
    noise_roughness: f32,
    noise_fractal: &str,
) -> PyResult<Geometry> {
    let texture = match image {
        Some(img) => Some(procgeo_sops::deform::DisplaceTexture {
            width: img.inner.width(),
            height: img.inner.height(),
            pixels: img.inner.to_cpu().map_err(cop_err)?,
        }),
        None => None,
    };

    let noise = noise_type.map(|kind| procgeo_sops::deform::DisplaceNoiseParams {
        noise_type: py_parse_displace_noise_type(kind),
        fractal: py_parse_displace_noise_fractal(noise_fractal),
        scale: [noise_scale_x, noise_scale_y, noise_scale_z],
        offset: [noise_offset_x, noise_offset_y, noise_offset_z],
        seed: noise_seed,
        octaves: noise_octaves,
        lacunarity: noise_lacunarity,
        roughness: noise_roughness,
    });

    let params = procgeo_sops::deform::DisplaceParams {
        strength,
        midlevel,
        direction: py_parse_displace_direction(direction),
        coordinates: py_parse_displace_coordinates(coordinates),
        projection: py_parse_displace_projection(projection),
        uv_attrib: uv_attrib.to_string(),
        normal_attrib: normal_attrib.to_string(),
        sample_channel: py_parse_displace_channel(sample_channel),
        sampler: py_parse_displace_sampler(sampler),
        wrap: py_parse_displace_wrap(wrap),
        coord_scale: [coord_scale_u, coord_scale_v],
        coord_offset: [coord_offset_u, coord_offset_v],
        custom_vector: [custom_vector_x, custom_vector_y, custom_vector_z],
        texture,
        noise,
    };

    let inner = procgeo_sops::deform::DisplaceSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[pyfunction]
#[pyo3(signature = (geo, rest_lattice, deformed_lattice, radius=1.0, min_points=1, max_points=10, rigid_projection=true, mask=1.0))]
fn point_deform(
    geo: &Geometry,
    rest_lattice: &Geometry,
    deformed_lattice: &Geometry,
    radius: f32,
    min_points: u32,
    max_points: u32,
    rigid_projection: bool,
    mask: f32,
) -> PyResult<Geometry> {
    let params = procgeo_sops::deform::PointDeformParams {
        group: None,
        mode: procgeo_sops::deform::PointDeformMode::CaptureAndDeform,
        radius,
        min_points,
        max_points,
        piece_attrib: None,
        rigid_projection,
        mask,
        mask_attrib: None,
        recompute_normals: true,
        attribs_to_transform: "*".into(),
        delete_capture_attribs: true,
    };
    let inner = procgeo_sops::deform::PointDeformSop
        .execute(
            &[&geo.inner, &rest_lattice.inner, &deformed_lattice.inner],
            &params,
        )
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[pyfunction]
#[pyo3(signature = (a, b, operation="union", treat_a_as="solid", treat_b_as="solid", collapse_tiny_edges=true))]
fn boolean_op(
    a: &Geometry,
    b: &Geometry,
    operation: &str,
    treat_a_as: &str,
    treat_b_as: &str,
    collapse_tiny_edges: bool,
) -> PyResult<Geometry> {
    let op = match operation {
        "intersect" => procgeo_sops::boolean::BooleanOp::Intersect,
        "subtract" => procgeo_sops::boolean::BooleanOp::Subtract,
        "shatter" => procgeo_sops::boolean::BooleanOp::Shatter,
        "seam" => procgeo_sops::boolean::BooleanOp::Seam,
        "detect" => procgeo_sops::boolean::BooleanOp::Detect,
        "resolve" => procgeo_sops::boolean::BooleanOp::Resolve,
        "custom" => procgeo_sops::boolean::BooleanOp::Custom,
        _ => procgeo_sops::boolean::BooleanOp::Union,
    };
    let ta = match treat_a_as {
        "surface" => procgeo_sops::boolean::BooleanTreatAs::Surface,
        _ => procgeo_sops::boolean::BooleanTreatAs::Solid,
    };
    let tb = match treat_b_as {
        "surface" => procgeo_sops::boolean::BooleanTreatAs::Surface,
        _ => procgeo_sops::boolean::BooleanTreatAs::Solid,
    };
    let params = procgeo_sops::boolean::BooleanParams {
        operation: op,
        treat_a_as: ta,
        treat_b_as: tb,
        collapse_tiny_edges,
        ..Default::default()
    };
    let inner = procgeo_sops::boolean::BooleanSop
        .execute(&[&a.inner, &b.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

/// The procgeo Python module.
#[pymodule]
fn procgeo(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Geometry>()?;
    // Creation
    m.add_function(wrap_pyfunction!(add, m)?)?;
    m.add_function(wrap_pyfunction!(create_box, m)?)?;
    m.add_function(wrap_pyfunction!(create_grid, m)?)?;
    m.add_function(wrap_pyfunction!(create_sphere, m)?)?;
    m.add_function(wrap_pyfunction!(create_line, m)?)?;
    m.add_function(wrap_pyfunction!(create_spiral, m)?)?;
    m.add_function(wrap_pyfunction!(create_helix, m)?)?;
    m.add_function(wrap_pyfunction!(create_circle, m)?)?;
    m.add_function(wrap_pyfunction!(create_tube, m)?)?;
    m.add_function(wrap_pyfunction!(create_torus, m)?)?;
    m.add_function(wrap_pyfunction!(create_icosphere, m)?)?;
    m.add_function(wrap_pyfunction!(create_teapot, m)?)?;
    m.add_function(wrap_pyfunction!(revolve, m)?)?;
    m.add_function(wrap_pyfunction!(create_metaball, m)?)?;
    // Manipulation
    m.add_function(wrap_pyfunction!(transform, m)?)?;
    m.add_function(wrap_pyfunction!(bend, m)?)?;
    m.add_function(wrap_pyfunction!(displace, m)?)?;
    m.add_function(wrap_pyfunction!(point_deform, m)?)?;
    m.add_function(wrap_pyfunction!(boolean_op, m)?)?;
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
    m.add_function(wrap_pyfunction!(sort, m)?)?;
    m.add_function(wrap_pyfunction!(voronoi_fracture, m)?)?;
    // Reshape SOPs
    m.add_function(wrap_pyfunction!(poly_bevel, m)?)?;
    m.add_function(wrap_pyfunction!(poly_wire, m)?)?;
    m.add_function(wrap_pyfunction!(poly_reduce, m)?)?;
    m.add_function(wrap_pyfunction!(poly_fill, m)?)?;
    m.add_function(wrap_pyfunction!(quad_remesh, m)?)?;
    // Delete SOPs
    m.add_function(wrap_pyfunction!(blast, m)?)?;
    m.add_function(wrap_pyfunction!(delete_geo, m)?)?;
    // Attribute SOPs
    m.add_function(wrap_pyfunction!(attrib_transfer, m)?)?;
    m.add_function(wrap_pyfunction!(attrib_copy, m)?)?;
    m.add_function(wrap_pyfunction!(attrib_randomize, m)?)?;
    m.add_function(wrap_pyfunction!(attrib_sort, m)?)?;
    m.add_function(wrap_pyfunction!(attrib_blur, m)?)?;
    m.add_function(wrap_pyfunction!(attrib_fill, m)?)?;
    m.add_function(wrap_pyfunction!(attrib_noise, m)?)?;
    m.add_function(wrap_pyfunction!(attrib_create, m)?)?;
    m.add_function(wrap_pyfunction!(attrib_delete, m)?)?;
    m.add_function(wrap_pyfunction!(attrib_rename, m)?)?;
    m.add_function(wrap_pyfunction!(attrib_promote, m)?)?;
    // Group SOPs
    m.add_function(wrap_pyfunction!(group_create, m)?)?;
    m.add_function(wrap_pyfunction!(group_combine, m)?)?;
    // QuadWild
    m.add_function(wrap_pyfunction!(quad_wild, m)?)?;
    // SOP Registry
    m.add_function(wrap_pyfunction!(execute_sop, m)?)?;
    m.add_function(wrap_pyfunction!(execute_sop_create, m)?)?;
    m.add_function(wrap_pyfunction!(list_sops, m)?)?;
    // I/O
    m.add_function(wrap_pyfunction!(write_obj, m)?)?;
    m.add_function(wrap_pyfunction!(write_glb, m)?)?;
    // COP Registry
    m.add_class::<CopImage>()?;
    m.add_function(wrap_pyfunction!(execute_cop_create, m)?)?;
    m.add_function(wrap_pyfunction!(execute_cop, m)?)?;
    m.add_function(wrap_pyfunction!(execute_cop_composite, m)?)?;
    m.add_function(wrap_pyfunction!(list_cops, m)?)?;
    m.add_function(wrap_pyfunction!(save_cop_image, m)?)?;
    Ok(())
}
