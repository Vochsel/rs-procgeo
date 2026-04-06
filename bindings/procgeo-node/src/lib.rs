#![deny(clippy::all)]

use napi::bindgen_prelude::*;
use napi_derive::napi;

use procgeo_sops::Sop;

/// Wrapped Geometry object exposed to JavaScript.
#[napi]
pub struct Geometry {
    inner: procgeo_core::Geometry,
}

#[napi]
impl Geometry {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: procgeo_core::Geometry::new(),
        }
    }

    #[napi(getter)]
    pub fn num_points(&self) -> u32 {
        self.inner.num_points() as u32
    }

    #[napi(getter)]
    pub fn num_prims(&self) -> u32 {
        self.inner.num_prims() as u32
    }

    #[napi(getter)]
    pub fn num_vertices(&self) -> u32 {
        self.inner.num_vertices() as u32
    }

    #[napi]
    pub fn point_pos(&self, index: u32) -> Vec<f64> {
        let pos = self
            .inner
            .point_pos(procgeo_core::PointHandle::from_index(index as usize));
        vec![pos.x as f64, pos.y as f64, pos.z as f64]
    }

    #[napi]
    pub fn bounding_box(&self) -> serde_json::Value {
        let bbox = self.inner.bounding_box();
        serde_json::json!({
            "min": [bbox.min.x, bbox.min.y, bbox.min.z],
            "max": [bbox.max.x, bbox.max.y, bbox.max.z]
        })
    }
}

// ---------------------------------------------------------------------------
// Helper functions for reading params from serde_json::Value
// ---------------------------------------------------------------------------

fn get_f32(obj: &serde_json::Value, key: &str, default: f32) -> f32 {
    obj.get(key)
        .and_then(|v| v.as_f64())
        .map(|v| v as f32)
        .unwrap_or(default)
}

fn get_u32(obj: &serde_json::Value, key: &str, default: u32) -> u32 {
    obj.get(key)
        .and_then(|v| v.as_u64())
        .map(|v| v as u32)
        .unwrap_or(default)
}

fn get_vec3(obj: &serde_json::Value, key: &str, default: [f32; 3]) -> glam::Vec3 {
    obj.get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            glam::Vec3::new(
                arr.get(0)
                    .and_then(|v| v.as_f64())
                    .unwrap_or(default[0] as f64) as f32,
                arr.get(1)
                    .and_then(|v| v.as_f64())
                    .unwrap_or(default[1] as f64) as f32,
                arr.get(2)
                    .and_then(|v| v.as_f64())
                    .unwrap_or(default[2] as f64) as f32,
            )
        })
        .unwrap_or(glam::Vec3::new(default[0], default[1], default[2]))
}

fn get_bool(obj: &serde_json::Value, key: &str, default: bool) -> bool {
    obj.get(key)
        .and_then(|v| v.as_bool())
        .unwrap_or(default)
}

fn sop_err(e: procgeo_sops::SopError) -> napi::Error {
    napi::Error::from_reason(e.to_string())
}

// ---------------------------------------------------------------------------
// Creation SOPs
// ---------------------------------------------------------------------------

#[napi]
pub fn create_box(params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let params = procgeo_sops::creation::BoxParams {
        size: get_vec3(&p, "size", [1.0, 1.0, 1.0]),
        center: get_vec3(&p, "center", [0.0, 0.0, 0.0]),
    };
    let inner = procgeo_sops::creation::BoxSop
        .execute(&[], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[napi]
pub fn create_grid(params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let params = procgeo_sops::creation::GridParams {
        size: [get_f32(&p, "size_x", 10.0), get_f32(&p, "size_y", 10.0)],
        rows: get_u32(&p, "rows", 10),
        cols: get_u32(&p, "cols", 10),
        center: get_vec3(&p, "center", [0.0, 0.0, 0.0]),
        ..Default::default()
    };
    let inner = procgeo_sops::creation::GridSop
        .execute(&[], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[napi]
pub fn create_sphere(params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let r = get_f32(&p, "radius", 0.5);
    let params = procgeo_sops::creation::SphereParams {
        radius: glam::Vec3::splat(r),
        center: get_vec3(&p, "center", [0.0, 0.0, 0.0]),
        rows: get_u32(&p, "rows", 12),
        cols: get_u32(&p, "cols", 24),
    };
    let inner = procgeo_sops::creation::SphereSop
        .execute(&[], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[napi]
pub fn create_line(params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let params = procgeo_sops::creation::LineParams {
        origin: get_vec3(&p, "origin", [0.0, 0.0, 0.0]),
        direction: get_vec3(&p, "direction", [0.0, 1.0, 0.0]),
        length: get_f32(&p, "length", 1.0),
        points: get_u32(&p, "points", 2),
    };
    let inner = procgeo_sops::creation::LineSop
        .execute(&[], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[napi]
pub fn create_circle(params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let params = procgeo_sops::creation::CircleParams {
        radius: get_f32(&p, "radius", 1.0),
        center: get_vec3(&p, "center", [0.0, 0.0, 0.0]),
        divisions: get_u32(&p, "divisions", 40),
        ..Default::default()
    };
    let inner = procgeo_sops::creation::CircleSop
        .execute(&[], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[napi]
pub fn create_tube(params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let params = procgeo_sops::creation::TubeParams {
        radius_bottom: get_f32(&p, "radius_bottom", 0.5),
        radius_top: get_f32(&p, "radius_top", 0.5),
        height: get_f32(&p, "height", 1.0),
        center: get_vec3(&p, "center", [0.0, 0.0, 0.0]),
        cols: get_u32(&p, "cols", 24),
        rows: get_u32(&p, "rows", 2),
        ..Default::default()
    };
    let inner = procgeo_sops::creation::TubeSop
        .execute(&[], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[napi]
pub fn create_torus(params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let params = procgeo_sops::creation::TorusParams {
        radius_outer: get_f32(&p, "radius_outer", 1.0),
        radius_inner: get_f32(&p, "radius_inner", 0.3),
        center: get_vec3(&p, "center", [0.0, 0.0, 0.0]),
        rows: get_u32(&p, "rows", 12),
        cols: get_u32(&p, "cols", 24),
    };
    let inner = procgeo_sops::creation::TorusSop
        .execute(&[], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

// ---------------------------------------------------------------------------
// Manipulation SOPs
// ---------------------------------------------------------------------------

#[napi]
pub fn transform(geo: &Geometry, params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let params = procgeo_sops::transform::TransformParams {
        translate: get_vec3(&p, "translate", [0.0, 0.0, 0.0]),
        rotate: get_vec3(&p, "rotate", [0.0, 0.0, 0.0]),
        scale: get_vec3(&p, "scale", [1.0, 1.0, 1.0]),
        pivot: get_vec3(&p, "pivot", [0.0, 0.0, 0.0]),
    };
    let inner = procgeo_sops::transform::TransformSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[napi]
pub fn compute_normals(geo: &Geometry) -> Result<Geometry> {
    let inner = procgeo_sops::normals::NormalSop
        .execute(
            &[&geo.inner],
            &procgeo_sops::normals::NormalParams::default(),
        )
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

/// Merge multiple Geometry objects. Accepts an array of Geometry instances.
/// Because napi-rs does not support Vec<&T> for napi classes, this function
/// takes a raw JS value and extracts the stored geometry via a workaround:
/// callers should pass an array of Geometry objects created by this module.
///
/// Implementation: each Geometry is cloned internally when passed through napi.
/// We accept Vec<ClassInstance<Geometry>> which napi-rs does support.
#[napi]
pub fn merge(geometries: Vec<ClassInstance<Geometry>>) -> Result<Geometry> {
    let refs: Vec<&procgeo_core::Geometry> = geometries.iter().map(|g| &g.inner).collect();
    let inner = procgeo_sops::merge::MergeSop
        .execute(&refs, &procgeo_sops::merge::MergeParams)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[napi]
pub fn subdivide(geo: &Geometry, params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let params = procgeo_sops::reshape::SubdivideParams {
        depth: get_u32(&p, "depth", 1),
    };
    let inner = procgeo_sops::reshape::SubdivideSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[napi]
pub fn scatter(geo: &Geometry, params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let params = procgeo_sops::scatter::ScatterParams {
        count: get_u32(&p, "count", 100),
        seed: p.get("seed").and_then(|v| v.as_u64()).unwrap_or(0),
    };
    let inner = procgeo_sops::scatter::ScatterSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[napi]
pub fn copy_to_points(source: &Geometry, target: &Geometry) -> Result<Geometry> {
    let inner = procgeo_sops::copy::CopyToPointsSop
        .execute(
            &[&source.inner, &target.inner],
            &procgeo_sops::copy::CopyToPointsParams,
        )
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[napi]
pub fn poly_extrude(geo: &Geometry, params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let params = procgeo_sops::reshape::PolyExtrudeParams {
        distance: get_f32(&p, "distance", 1.0),
        inset: get_f32(&p, "inset", 0.0),
        output_front: get_bool(&p, "output_front", true),
        output_side: get_bool(&p, "output_side", true),
    };
    let inner = procgeo_sops::reshape::PolyExtrudeSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[napi]
pub fn smooth(geo: &Geometry, params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let params = procgeo_sops::reshape::SmoothParams {
        iterations: get_u32(&p, "iterations", 1),
        strength: get_f32(&p, "strength", 0.5),
    };
    let inner = procgeo_sops::reshape::SmoothSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[napi]
pub fn clip(geo: &Geometry, params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let params = procgeo_sops::reshape::ClipParams {
        origin: get_vec3(&p, "origin", [0.0, 0.0, 0.0]),
        normal: get_vec3(&p, "normal", [0.0, 1.0, 0.0]),
        keep_above: get_bool(&p, "keep_above", true),
    };
    let inner = procgeo_sops::reshape::ClipSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[napi]
pub fn reverse(geo: &Geometry) -> Result<Geometry> {
    let inner = procgeo_sops::topology::ReverseSop
        .execute(
            &[&geo.inner],
            &procgeo_sops::topology::ReverseParams::default(),
        )
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[napi]
pub fn resample(geo: &Geometry, params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let params = procgeo_sops::topology::ResampleParams {
        length: get_f32(&p, "length", 0.1),
        max_segments: get_u32(&p, "max_segments", 1000),
    };
    let inner = procgeo_sops::topology::ResampleSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[napi]
pub fn sort(geo: &Geometry, params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let params = procgeo_sops::topology::SortParams {
        seed: p.get("seed").and_then(|v| v.as_u64()).unwrap_or(0),
        ..Default::default()
    };
    let inner = procgeo_sops::topology::SortSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[napi]
pub fn fuse(geo: &Geometry, params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let params = procgeo_sops::topology::FuseParams {
        distance: get_f32(&p, "distance", 0.001),
    };
    let inner = procgeo_sops::topology::FuseSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[napi]
pub fn connectivity(geo: &Geometry, params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let params = procgeo_sops::topology::ConnectivityParams {
        attrib_name: p
            .get("attrib_name")
            .and_then(|v| v.as_str())
            .unwrap_or("class")
            .to_string(),
    };
    let inner = procgeo_sops::topology::ConnectivitySop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[napi]
pub fn color(geo: &Geometry, params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let color_arr = p
        .get("color")
        .and_then(|v| v.as_array())
        .map(|arr| {
            [
                arr.get(0).and_then(|v| v.as_f64()).unwrap_or(1.0) as f32,
                arr.get(1).and_then(|v| v.as_f64()).unwrap_or(1.0) as f32,
                arr.get(2).and_then(|v| v.as_f64()).unwrap_or(1.0) as f32,
            ]
        })
        .unwrap_or([1.0, 1.0, 1.0]);
    let params = procgeo_sops::color::ColorParams { color: color_arr };
    let inner = procgeo_sops::color::ColorSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[napi]
pub fn enumerate_attrib(geo: &Geometry, params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let params = procgeo_sops::utility::EnumerateParams {
        name: p
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("index")
            .to_string(),
        start: p
            .get("start")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32,
        ..Default::default()
    };
    let inner = procgeo_sops::utility::EnumerateSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[napi]
pub fn measure(geo: &Geometry, params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let params = procgeo_sops::measure::MeasureParams {
        attrib_name: p
            .get("attrib_name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        ..Default::default()
    };
    let inner = procgeo_sops::measure::MeasureSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

// ---------------------------------------------------------------------------
// I/O
// ---------------------------------------------------------------------------

#[napi]
pub fn write_obj(geo: &Geometry, path: String) -> Result<()> {
    procgeo_io::write_file(&geo.inner, std::path::Path::new(&path))
        .map_err(|e| napi::Error::from_reason(e.to_string()))
}

#[napi]
pub fn write_glb(geo: &Geometry, path: String) -> Result<()> {
    procgeo_io::write_file(&geo.inner, std::path::Path::new(&path))
        .map_err(|e| napi::Error::from_reason(e.to_string()))
}
