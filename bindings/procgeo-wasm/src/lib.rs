use std::sync::OnceLock;

use wasm_bindgen::prelude::*;
use procgeo_core::{AttribClass, PrimHandle, PointHandle};
use procgeo_sops::Sop;

/// Geometry wrapper exposed to JS via WASM.
#[wasm_bindgen]
pub struct Geometry {
    inner: procgeo_core::Geometry,
}

#[wasm_bindgen]
impl Geometry {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self { inner: procgeo_core::Geometry::new() }
    }

    #[wasm_bindgen(getter, js_name = "numPoints")]
    pub fn num_points(&self) -> u32 {
        self.inner.num_points() as u32
    }

    #[wasm_bindgen(getter, js_name = "numPrims")]
    pub fn num_prims(&self) -> u32 {
        self.inner.num_prims() as u32
    }

    #[wasm_bindgen(getter, js_name = "numVertices")]
    pub fn num_vertices(&self) -> u32 {
        self.inner.num_vertices() as u32
    }

    #[wasm_bindgen(js_name = "pointPos")]
    pub fn point_pos(&self, index: u32) -> Vec<f32> {
        let pos = self.inner.point_pos(PointHandle::from_index(index as usize));
        vec![pos.x, pos.y, pos.z]
    }

    #[wasm_bindgen(js_name = "boundingBox")]
    pub fn bounding_box(&self) -> JsValue {
        let bbox = self.inner.bounding_box();
        let obj = js_sys::Object::new();
        let min = js_sys::Float32Array::from(&[bbox.min.x, bbox.min.y, bbox.min.z][..]);
        let max = js_sys::Float32Array::from(&[bbox.max.x, bbox.max.y, bbox.max.z][..]);
        js_sys::Reflect::set(&obj, &"min".into(), &min).unwrap();
        js_sys::Reflect::set(&obj, &"max".into(), &max).unwrap();
        obj.into()
    }

    /// Get all point positions as a flat Float32Array [x0,y0,z0, x1,y1,z1, ...]
    /// Useful for feeding directly to WebGL/Three.js BufferGeometry.
    #[wasm_bindgen(js_name = "getPositions")]
    pub fn get_positions(&self) -> Vec<f32> {
        let n = self.inner.num_points();
        let mut buf = Vec::with_capacity(n * 3);
        // points() iterates Vec3 values directly
        for p in self.inner.points() {
            buf.push(p.x);
            buf.push(p.y);
            buf.push(p.z);
        }
        buf
    }

    /// Get triangle indices as a flat Uint32Array (fan-triangulated).
    /// Useful for WebGL/Three.js index buffers.
    #[wasm_bindgen(js_name = "getTriangleIndices")]
    pub fn get_triangle_indices(&self) -> Vec<u32> {
        let mut indices = Vec::new();
        for (i, _prim) in self.inner.prims().enumerate() {
            let ph = PrimHandle::from_index(i);
            let pts = self.inner.prim_points(ph);
            if pts.len() >= 3 {
                // Fan triangulate from first vertex
                for j in 1..pts.len() - 1 {
                    indices.push(pts[0].index() as u32);
                    indices.push(pts[j].index() as u32);
                    indices.push(pts[j + 1].index() as u32);
                }
            }
        }
        indices
    }

    /// Get normals as a flat Float32Array (if "N" attribute exists).
    #[wasm_bindgen(js_name = "getNormals")]
    pub fn get_normals(&self) -> Option<Vec<f32>> {
        let handle = self.inner.find_attrib::<[f32; 3]>(AttribClass::Point, "N").ok()?;
        let n = self.inner.num_points();
        let mut buf = Vec::with_capacity(n * 3);
        for i in 0..n {
            let normal = self.inner.get_attrib(&handle, i).ok()?;
            buf.push(normal[0]);
            buf.push(normal[1]);
            buf.push(normal[2]);
        }
        Some(buf)
    }

    /// Get colors as a flat Float32Array (if "Cd" attribute exists).
    #[wasm_bindgen(js_name = "getColors")]
    pub fn get_colors(&self) -> Option<Vec<f32>> {
        let handle = self.inner.find_attrib::<[f32; 3]>(AttribClass::Point, "Cd").ok()?;
        let n = self.inner.num_points();
        let mut buf = Vec::with_capacity(n * 3);
        for i in 0..n {
            let c = self.inner.get_attrib(&handle, i).ok()?;
            buf.push(c[0]);
            buf.push(c[1]);
            buf.push(c[2]);
        }
        Some(buf)
    }

    /// Write geometry as OBJ string.
    #[wasm_bindgen(js_name = "toObj")]
    pub fn to_obj(&self) -> Result<String, JsError> {
        let mut buf = Vec::new();
        use procgeo_io::GeometryWriter;
        procgeo_io::obj::ObjWriter.write(&self.inner, &mut buf)
            .map_err(|e| JsError::new(&e.to_string()))?;
        String::from_utf8(buf).map_err(|e| JsError::new(&e.to_string()))
    }

    /// Write geometry as GLB bytes (Uint8Array).
    #[wasm_bindgen(js_name = "toGlb")]
    pub fn to_glb(&self) -> Result<Vec<u8>, JsError> {
        let mut buf = Vec::new();
        use procgeo_io::GeometryWriter;
        procgeo_io::gltf::GlbWriter.write(&self.inner, &mut buf)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(buf)
    }
}

// ---------------------------------------------------------------------------
// Helper functions for parameter extraction from JS objects
// ---------------------------------------------------------------------------

fn get_f32(val: &JsValue, key: &str, default: f32) -> f32 {
    js_sys::Reflect::get(val, &key.into())
        .ok()
        .and_then(|v| v.as_f64())
        .map(|v| v as f32)
        .unwrap_or(default)
}

fn get_u32(val: &JsValue, key: &str, default: u32) -> u32 {
    js_sys::Reflect::get(val, &key.into())
        .ok()
        .and_then(|v| v.as_f64())
        .map(|v| v as u32)
        .unwrap_or(default)
}

fn get_u64(val: &JsValue, key: &str, default: u64) -> u64 {
    js_sys::Reflect::get(val, &key.into())
        .ok()
        .and_then(|v| v.as_f64())
        .map(|v| v as u64)
        .unwrap_or(default)
}

fn get_bool(val: &JsValue, key: &str, default: bool) -> bool {
    js_sys::Reflect::get(val, &key.into())
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(default)
}

fn get_vec3(val: &JsValue, key: &str, default: [f32; 3]) -> glam::Vec3 {
    let arr = js_sys::Reflect::get(val, &key.into()).ok();
    if let Some(arr) = arr {
        if let Some(arr) = arr.dyn_ref::<js_sys::Array>() {
            return glam::Vec3::new(
                arr.get(0).as_f64().unwrap_or(default[0] as f64) as f32,
                arr.get(1).as_f64().unwrap_or(default[1] as f64) as f32,
                arr.get(2).as_f64().unwrap_or(default[2] as f64) as f32,
            );
        }
    }
    glam::Vec3::from(default)
}

fn sop_err(e: procgeo_sops::SopError) -> JsError {
    JsError::new(&e.to_string())
}

fn empty_obj() -> JsValue {
    js_sys::Object::new().into()
}

// ---------------------------------------------------------------------------
// Creation SOPs
// ---------------------------------------------------------------------------

#[wasm_bindgen(js_name = "createBox")]
pub fn create_box(params: Option<JsValue>) -> Result<Geometry, JsError> {
    let p = params.unwrap_or_else(empty_obj);
    let params = procgeo_sops::creation::BoxParams {
        size: get_vec3(&p, "size", [1.0, 1.0, 1.0]),
        center: get_vec3(&p, "center", [0.0, 0.0, 0.0]),
    };
    let inner = procgeo_sops::creation::BoxSop.execute(&[], &params).map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[wasm_bindgen(js_name = "createGrid")]
pub fn create_grid(params: Option<JsValue>) -> Result<Geometry, JsError> {
    let p = params.unwrap_or_else(empty_obj);
    let params = procgeo_sops::creation::GridParams {
        size: [get_f32(&p, "sizeX", 10.0), get_f32(&p, "sizeY", 10.0)],
        rows: get_u32(&p, "rows", 10),
        cols: get_u32(&p, "cols", 10),
        center: get_vec3(&p, "center", [0.0, 0.0, 0.0]),
        ..Default::default()
    };
    let inner = procgeo_sops::creation::GridSop.execute(&[], &params).map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[wasm_bindgen(js_name = "createSphere")]
pub fn create_sphere(params: Option<JsValue>) -> Result<Geometry, JsError> {
    let p = params.unwrap_or_else(empty_obj);
    let r = get_f32(&p, "radius", 0.5);
    let params = procgeo_sops::creation::SphereParams {
        radius: glam::Vec3::splat(r),
        center: get_vec3(&p, "center", [0.0, 0.0, 0.0]),
        rows: get_u32(&p, "rows", 12),
        cols: get_u32(&p, "cols", 24),
    };
    let inner = procgeo_sops::creation::SphereSop.execute(&[], &params).map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[wasm_bindgen(js_name = "createLine")]
pub fn create_line(params: Option<JsValue>) -> Result<Geometry, JsError> {
    let p = params.unwrap_or_else(empty_obj);
    let params = procgeo_sops::creation::LineParams {
        origin: get_vec3(&p, "origin", [0.0, 0.0, 0.0]),
        direction: get_vec3(&p, "direction", [0.0, 1.0, 0.0]),
        length: get_f32(&p, "length", 1.0),
        points: get_u32(&p, "points", 2),
    };
    let inner = procgeo_sops::creation::LineSop.execute(&[], &params).map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[wasm_bindgen(js_name = "createCircle")]
pub fn create_circle(params: Option<JsValue>) -> Result<Geometry, JsError> {
    let p = params.unwrap_or_else(empty_obj);
    let params = procgeo_sops::creation::CircleParams {
        radius: get_f32(&p, "radius", 1.0),
        center: get_vec3(&p, "center", [0.0, 0.0, 0.0]),
        divisions: get_u32(&p, "divisions", 40),
        ..Default::default()
    };
    let inner = procgeo_sops::creation::CircleSop.execute(&[], &params).map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[wasm_bindgen(js_name = "createTube")]
pub fn create_tube(params: Option<JsValue>) -> Result<Geometry, JsError> {
    let p = params.unwrap_or_else(empty_obj);
    let params = procgeo_sops::creation::TubeParams {
        radius_bottom: get_f32(&p, "radiusBottom", 0.5),
        radius_top: get_f32(&p, "radiusTop", 0.5),
        height: get_f32(&p, "height", 1.0),
        center: get_vec3(&p, "center", [0.0, 0.0, 0.0]),
        cols: get_u32(&p, "cols", 24),
        rows: get_u32(&p, "rows", 2),
        ..Default::default()
    };
    let inner = procgeo_sops::creation::TubeSop.execute(&[], &params).map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[wasm_bindgen(js_name = "createTorus")]
pub fn create_torus(params: Option<JsValue>) -> Result<Geometry, JsError> {
    let p = params.unwrap_or_else(empty_obj);
    let params = procgeo_sops::creation::TorusParams {
        radius_outer: get_f32(&p, "radiusOuter", 1.0),
        radius_inner: get_f32(&p, "radiusInner", 0.3),
        center: get_vec3(&p, "center", [0.0, 0.0, 0.0]),
        rows: get_u32(&p, "rows", 12),
        cols: get_u32(&p, "cols", 24),
    };
    let inner = procgeo_sops::creation::TorusSop.execute(&[], &params).map_err(sop_err)?;
    Ok(Geometry { inner })
}

// ---------------------------------------------------------------------------
// Manipulation SOPs
// ---------------------------------------------------------------------------

#[wasm_bindgen]
pub fn transform(geo: &Geometry, params: Option<JsValue>) -> Result<Geometry, JsError> {
    let p = params.unwrap_or_else(empty_obj);
    let params = procgeo_sops::transform::TransformParams {
        translate: get_vec3(&p, "translate", [0.0, 0.0, 0.0]),
        rotate: get_vec3(&p, "rotate", [0.0, 0.0, 0.0]),
        scale: get_vec3(&p, "scale", [1.0, 1.0, 1.0]),
        pivot: get_vec3(&p, "pivot", [0.0, 0.0, 0.0]),
    };
    let inner = procgeo_sops::transform::TransformSop.execute(&[&geo.inner], &params).map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[wasm_bindgen(js_name = "computeNormals")]
pub fn compute_normals(geo: &Geometry) -> Result<Geometry, JsError> {
    let inner = procgeo_sops::normals::NormalSop.execute(&[&geo.inner], &Default::default()).map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[wasm_bindgen]
pub fn subdivide(geo: &Geometry, params: Option<JsValue>) -> Result<Geometry, JsError> {
    let p = params.unwrap_or_else(empty_obj);
    let mode_str = js_sys::Reflect::get(&p, &"mode".into())
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_default();
    let mode = match mode_str.as_str() {
        "catmullClark" | "catmull-clark" | "cc" => procgeo_sops::reshape::SubdivideMode::CatmullClark,
        _ => procgeo_sops::reshape::SubdivideMode::Linear,
    };
    let params = procgeo_sops::reshape::SubdivideParams {
        depth: get_u32(&p, "depth", 1),
        mode,
    };
    let inner = procgeo_sops::reshape::SubdivideSop.execute(&[&geo.inner], &params).map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[wasm_bindgen]
pub fn scatter(geo: &Geometry, params: Option<JsValue>) -> Result<Geometry, JsError> {
    let p = params.unwrap_or_else(empty_obj);
    let params = procgeo_sops::scatter::ScatterParams {
        count: get_u32(&p, "count", 100),
        seed: get_u64(&p, "seed", 0),
    };
    let inner = procgeo_sops::scatter::ScatterSop.execute(&[&geo.inner], &params).map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[wasm_bindgen(js_name = "copyToPoints")]
pub fn copy_to_points(source: &Geometry, target: &Geometry) -> Result<Geometry, JsError> {
    let inner = procgeo_sops::copy::CopyToPointsSop.execute(&[&source.inner, &target.inner], &Default::default()).map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[wasm_bindgen(js_name = "polyExtrude")]
pub fn poly_extrude(geo: &Geometry, params: Option<JsValue>) -> Result<Geometry, JsError> {
    let p = params.unwrap_or_else(empty_obj);
    let params = procgeo_sops::reshape::PolyExtrudeParams {
        distance: get_f32(&p, "distance", 1.0),
        inset: get_f32(&p, "inset", 0.0),
        output_front: get_bool(&p, "outputFront", true),
        output_side: get_bool(&p, "outputSide", true),
    };
    let inner = procgeo_sops::reshape::PolyExtrudeSop.execute(&[&geo.inner], &params).map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[wasm_bindgen]
pub fn smooth(geo: &Geometry, params: Option<JsValue>) -> Result<Geometry, JsError> {
    let p = params.unwrap_or_else(empty_obj);
    let params = procgeo_sops::reshape::SmoothParams {
        iterations: get_u32(&p, "iterations", 1),
        strength: get_f32(&p, "strength", 0.5),
    };
    let inner = procgeo_sops::reshape::SmoothSop.execute(&[&geo.inner], &params).map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[wasm_bindgen]
pub fn clip(geo: &Geometry, params: Option<JsValue>) -> Result<Geometry, JsError> {
    let p = params.unwrap_or_else(empty_obj);
    let params = procgeo_sops::reshape::ClipParams {
        origin: get_vec3(&p, "origin", [0.0, 0.0, 0.0]),
        normal: get_vec3(&p, "normal", [0.0, 1.0, 0.0]),
        keep_above: get_bool(&p, "keepAbove", true),
    };
    let inner = procgeo_sops::reshape::ClipSop.execute(&[&geo.inner], &params).map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[wasm_bindgen]
pub fn reverse(geo: &Geometry) -> Result<Geometry, JsError> {
    let inner = procgeo_sops::topology::ReverseSop.execute(&[&geo.inner], &Default::default()).map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[wasm_bindgen]
pub fn color(geo: &Geometry, params: Option<JsValue>) -> Result<Geometry, JsError> {
    let p = params.unwrap_or_else(empty_obj);
    let color_arr = get_vec3(&p, "color", [1.0, 1.0, 1.0]);
    let params = procgeo_sops::color::ColorParams { color: [color_arr.x, color_arr.y, color_arr.z] };
    let inner = procgeo_sops::color::ColorSop.execute(&[&geo.inner], &params).map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[wasm_bindgen]
pub fn fuse(geo: &Geometry, params: Option<JsValue>) -> Result<Geometry, JsError> {
    let p = params.unwrap_or_else(empty_obj);
    let params = procgeo_sops::topology::FuseParams {
        distance: get_f32(&p, "distance", 0.001),
    };
    let inner = procgeo_sops::topology::FuseSop.execute(&[&geo.inner], &params).map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[wasm_bindgen(js_name = "voronoiFracture")]
pub fn voronoi_fracture(geo: &Geometry, points: &Geometry, params: Option<JsValue>) -> Result<Geometry, JsError> {
    let p = params.unwrap_or_else(empty_obj);
    let params = procgeo_sops::voronoi::VoronoiFractureParams {
        cut_plane_offset: get_f32(&p, "cutPlaneOffset", 0.0),
        create_inside_faces: get_bool(&p, "createInsideFaces", true),
    };
    let inner = procgeo_sops::voronoi::VoronoiFractureSop.execute(&[&geo.inner, &points.inner], &params).map_err(sop_err)?;
    Ok(Geometry { inner })
}

// ---------------------------------------------------------------------------
// Attribute SOPs
// ---------------------------------------------------------------------------

fn get_str(val: &JsValue, key: &str, default: &str) -> String {
    js_sys::Reflect::get(val, &key.into())
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_else(|| default.to_string())
}

fn parse_attrib_class(s: &str) -> procgeo_core::AttribClass {
    serde_json::from_str(&format!("\"{}\"", s)).unwrap_or(procgeo_core::AttribClass::Point)
}

fn parse_attrib_type(s: &str) -> procgeo_core::AttribType {
    serde_json::from_str(&format!("\"{}\"", s)).unwrap_or(procgeo_core::AttribType::Float)
}

#[wasm_bindgen(js_name = "attribTransfer")]
pub fn attrib_transfer(dest: &Geometry, source: &Geometry, params: Option<JsValue>) -> Result<Geometry, JsError> {
    let p = params.unwrap_or_else(empty_obj);
    let params = procgeo_sops::attributes::AttribTransferParams {
        attrib_name: get_str(&p, "attribName", "attrib"),
        class: parse_attrib_class(&get_str(&p, "class", "Point")),
        attrib_type: parse_attrib_type(&get_str(&p, "attribType", "Float")),
        max_samples: get_u32(&p, "maxSamples", 1),
        distance_threshold: get_f32(&p, "distanceThreshold", f32::MAX),
    };
    let inner = procgeo_sops::attributes::AttribTransferSop
        .execute(&[&dest.inner, &source.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[wasm_bindgen(js_name = "attribCopy")]
pub fn attrib_copy(dest: &Geometry, source: Option<Geometry>, params: Option<JsValue>) -> Result<Geometry, JsError> {
    let p = params.unwrap_or_else(empty_obj);
    let params = procgeo_sops::attributes::AttribCopyParams {
        attrib_name: get_str(&p, "attribName", "attrib"),
        class: parse_attrib_class(&get_str(&p, "class", "Point")),
        attrib_type: parse_attrib_type(&get_str(&p, "attribType", "Float")),
        new_name: get_str(&p, "newName", ""),
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

#[wasm_bindgen(js_name = "attribRandomize")]
pub fn attrib_randomize(geo: &Geometry, params: Option<JsValue>) -> Result<Geometry, JsError> {
    let p = params.unwrap_or_else(empty_obj);
    let distribution = serde_json::from_str::<procgeo_sops::attributes::RandomDistribution>(
        &format!("\"{}\"", get_str(&p, "distribution", "Uniform")),
    ).unwrap_or(procgeo_sops::attributes::RandomDistribution::Uniform);
    let operation = serde_json::from_str::<procgeo_sops::attributes::RandomOperation>(
        &format!("\"{}\"", get_str(&p, "operation", "Set")),
    ).unwrap_or(procgeo_sops::attributes::RandomOperation::Set);
    let params = procgeo_sops::attributes::AttribRandomizeParams {
        attrib_name: get_str(&p, "attribName", "randomize"),
        class: parse_attrib_class(&get_str(&p, "class", "Point")),
        attrib_type: parse_attrib_type(&get_str(&p, "attribType", "Float")),
        distribution,
        operation,
        seed: get_u64(&p, "seed", 0),
        min_value: get_f32(&p, "minValue", 0.0),
        max_value: get_f32(&p, "maxValue", 1.0),
        mean: get_f32(&p, "mean", 0.0),
        stddev: get_f32(&p, "stddev", 1.0),
        value_a: get_f32(&p, "valueA", 0.0),
        value_b: get_f32(&p, "valueB", 1.0),
        probability: get_f32(&p, "probability", 0.5),
        dimensions: get_u32(&p, "dimensions", 1),
        global_scale: get_f32(&p, "globalScale", 1.0),
    };
    let inner = procgeo_sops::attributes::AttribRandomizeSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[wasm_bindgen(js_name = "attribSort")]
pub fn attrib_sort(geo: &Geometry, params: Option<JsValue>) -> Result<Geometry, JsError> {
    let p = params.unwrap_or_else(empty_obj);
    let order = serde_json::from_str::<procgeo_sops::attributes::AttribSortOrder>(
        &format!("\"{}\"", get_str(&p, "order", "Ascending")),
    ).unwrap_or(procgeo_sops::attributes::AttribSortOrder::Ascending);
    let params = procgeo_sops::attributes::AttribSortParams {
        attrib_name: get_str(&p, "attribName", "attrib"),
        class: parse_attrib_class(&get_str(&p, "class", "Point")),
        attrib_type: parse_attrib_type(&get_str(&p, "attribType", "Float")),
        order,
        component: get_u32(&p, "component", 0) as usize,
    };
    let inner = procgeo_sops::attributes::AttribSortSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[wasm_bindgen(js_name = "attribBlur")]
pub fn attrib_blur(geo: &Geometry, params: Option<JsValue>) -> Result<Geometry, JsError> {
    let p = params.unwrap_or_else(empty_obj);
    let params = procgeo_sops::attributes::AttribBlurParams {
        attrib_name: get_str(&p, "attribName", "attrib"),
        attrib_type: parse_attrib_type(&get_str(&p, "attribType", "Float")),
        iterations: get_u32(&p, "iterations", 1),
        step_size: get_f32(&p, "stepSize", 1.0),
    };
    let inner = procgeo_sops::attributes::AttribBlurSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[wasm_bindgen(js_name = "attribFill")]
pub fn attrib_fill(geo: &Geometry, params: Option<JsValue>) -> Result<Geometry, JsError> {
    let p = params.unwrap_or_else(empty_obj);
    let params = procgeo_sops::attributes::AttribFillParams {
        attrib_name: get_str(&p, "attribName", "attrib"),
        attrib_type: parse_attrib_type(&get_str(&p, "attribType", "Float")),
        boundary_group: get_str(&p, "boundaryGroup", ""),
        iterations: get_u32(&p, "iterations", 10),
        step_size: get_f32(&p, "stepSize", 0.5),
    };
    let inner = procgeo_sops::attributes::AttribFillSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[wasm_bindgen(js_name = "attribNoise")]
pub fn attrib_noise(geo: &Geometry, params: Option<JsValue>) -> Result<Geometry, JsError> {
    let p = params.unwrap_or_else(empty_obj);

    let noise_type_str = get_str(&p, "noiseType", "perlin");
    let noise_type = match noise_type_str.as_str() {
        "simplex" => procgeo_sops::attributes::NoiseType::Simplex,
        "worley" => procgeo_sops::attributes::NoiseType::Worley,
        "worleyF2F1" => procgeo_sops::attributes::NoiseType::WorleyF2F1,
        _ => procgeo_sops::attributes::NoiseType::Perlin,
    };
    let operation_str = get_str(&p, "operation", "add");
    let operation = match operation_str.as_str() {
        "setInitial" => procgeo_sops::attributes::NoiseOperation::SetInitial,
        "set" => procgeo_sops::attributes::NoiseOperation::Set,
        "subtract" => procgeo_sops::attributes::NoiseOperation::Subtract,
        "multiply" => procgeo_sops::attributes::NoiseOperation::Multiply,
        "min" | "minimum" => procgeo_sops::attributes::NoiseOperation::Minimum,
        "max" | "maximum" => procgeo_sops::attributes::NoiseOperation::Maximum,
        _ => procgeo_sops::attributes::NoiseOperation::Add,
    };
    let range_str = get_str(&p, "range", "positive");
    let range = match range_str.as_str() {
        "zeroCentered" | "ZeroCentered" => procgeo_sops::attributes::NoiseRange::ZeroCentered,
        "minMax" | "MinMax" => procgeo_sops::attributes::NoiseRange::MinMax,
        _ => procgeo_sops::attributes::NoiseRange::Positive,
    };
    let fractal_str = get_str(&p, "fractal", "none");
    let fractal = match fractal_str.as_str() {
        "standard" => procgeo_sops::attributes::FractalType::Standard,
        "terrain" => procgeo_sops::attributes::FractalType::Terrain,
        _ => procgeo_sops::attributes::FractalType::None,
    };
    let offset = get_vec3(&p, "offset", [0.0, 0.0, 0.0]);
    let params = procgeo_sops::attributes::AttribNoiseParams {
        attrib_name: get_str(&p, "attribName", "noise"),
        class: parse_attrib_class(&get_str(&p, "class", "Point")),
        dimensions: get_u32(&p, "dimensions", 1),
        noise_type,
        operation,
        element_size: get_f32(&p, "elementSize", 1.0),
        offset: [offset.x, offset.y, offset.z],
        seed: get_u64(&p, "seed", 0),
        range,
        amplitude: get_f32(&p, "amplitude", 1.0),
        min_value: get_f32(&p, "minValue", 0.0),
        max_value: get_f32(&p, "maxValue", 1.0),
        fractal,
        octaves: get_u32(&p, "octaves", 8),
        lacunarity: get_f32(&p, "lacunarity", 2.0),
        roughness: get_f32(&p, "roughness", 0.5),
        gain: get_f32(&p, "gain", 0.5),
        bias: get_f32(&p, "bias", 0.5),
    };
    let inner = procgeo_sops::attributes::AttribNoiseSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

// ---------------------------------------------------------------------------
// SOP Registry — generic execute
// ---------------------------------------------------------------------------

static REGISTRY: OnceLock<procgeo_sops::SopRegistry> = OnceLock::new();

fn get_registry() -> &'static procgeo_sops::SopRegistry {
    REGISTRY.get_or_init(procgeo_sops::default_registry)
}

/// Execute any registered SOP by name. Params are a JSON-compatible JS object.
/// Uses Rust/snake_case field names for params (matching serde serialization).
#[wasm_bindgen(js_name = "executeSop")]
pub fn execute_sop(
    name: &str,
    geo: &Geometry,
    params: Option<JsValue>,
) -> Result<Geometry, JsError> {
    let registry = get_registry();

    let params_json = match params {
        Some(p) if !p.is_undefined() && !p.is_null() => {
            js_sys::JSON::stringify(&p)
                .map_err(|_| JsError::new("Failed to serialize params"))?
                .as_string()
                .unwrap_or_default()
        }
        _ => "{}".to_string(),
    };

    let inputs = vec![&geo.inner];
    let inner = registry
        .execute(name, &inputs, &params_json)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

/// Execute a creation SOP (no input geometry required).
#[wasm_bindgen(js_name = "executeSopCreate")]
pub fn execute_sop_create(
    name: &str,
    params: Option<JsValue>,
) -> Result<Geometry, JsError> {
    let registry = get_registry();

    let params_json = match params {
        Some(p) if !p.is_undefined() && !p.is_null() => {
            js_sys::JSON::stringify(&p)
                .map_err(|_| JsError::new("Failed to serialize params"))?
                .as_string()
                .unwrap_or_default()
        }
        _ => "{}".to_string(),
    };

    let inner = registry
        .execute(name, &[], &params_json)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

/// List all registered SOP names.
#[wasm_bindgen(js_name = "listSops")]
pub fn list_sops() -> Vec<String> {
    get_registry()
        .list()
        .into_iter()
        .map(|s| s.to_string())
        .collect()
}
