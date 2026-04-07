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

    /// Add a point at position [x, y, z]. Returns the point index.
    #[wasm_bindgen(js_name = "addPoint")]
    pub fn add_point(&mut self, x: f32, y: f32, z: f32) -> u32 {
        self.inner.add_point(glam::Vec3::new(x, y, z)).index() as u32
    }

    /// Set the position of an existing point.
    #[wasm_bindgen(js_name = "setPointPos")]
    pub fn set_point_pos(&mut self, index: u32, x: f32, y: f32, z: f32) {
        self.inner.set_point_pos(
            PointHandle::from_index(index as usize),
            glam::Vec3::new(x, y, z),
        );
    }

    /// Create a closed face (polygon) from an array of point indices. Returns the primitive index.
    #[wasm_bindgen(js_name = "addFace")]
    pub fn add_face(&mut self, point_indices: &[u32]) -> u32 {
        let handles: Vec<PointHandle> = point_indices.iter().map(|&i| PointHandle::from_index(i as usize)).collect();
        self.inner.add_face(&handles).index() as u32
    }

    /// Create an open polyline from an array of point indices. Returns the primitive index.
    #[wasm_bindgen(js_name = "addPolyline")]
    pub fn add_polyline(&mut self, point_indices: &[u32]) -> u32 {
        let handles: Vec<PointHandle> = point_indices.iter().map(|&i| PointHandle::from_index(i as usize)).collect();
        self.inner.add_polyline(&handles).index() as u32
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

    // -----------------------------------------------------------------------
    // Attribute introspection (spreadsheet / debugging)
    // -----------------------------------------------------------------------

    fn parse_class(class: &str) -> AttribClass {
        match class {
            "vertex" | "Vertex" => AttribClass::Vertex,
            "primitive" | "Primitive" | "prim" => AttribClass::Primitive,
            "detail" | "Detail" => AttribClass::Detail,
            _ => AttribClass::Point,
        }
    }

    /// List attribute names for a class ("point", "vertex", "primitive", "detail").
    #[wasm_bindgen(js_name = "attribNames")]
    pub fn attrib_names(&self, class: &str) -> Vec<String> {
        self.inner
            .attrib_names(Self::parse_class(class))
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    /// Get the type name of an attribute ("Float", "Int", "Vector3", etc.).
    #[wasm_bindgen(js_name = "attribType")]
    pub fn attrib_type(&self, class: &str, name: &str) -> Option<String> {
        self.inner
            .attrib_type(Self::parse_class(class), name)
            .map(|t| format!("{t:?}"))
    }

    /// Get the component count of an attribute (1 for float, 3 for vec3, etc.).
    #[wasm_bindgen(js_name = "attribSize")]
    pub fn attrib_size(&self, class: &str, name: &str) -> Option<u32> {
        self.inner
            .attrib_size(Self::parse_class(class), name)
            .map(|s| s as u32)
    }

    /// Get all values of a numeric attribute as a flat Float64Array.
    /// Components interleaved: for vec3 → [x0,y0,z0, x1,y1,z1, ...].
    #[wasm_bindgen(js_name = "attribData")]
    pub fn attrib_data(&self, class: &str, name: &str) -> Option<Vec<f64>> {
        self.inner.attrib_data_f64(Self::parse_class(class), name)
    }

    /// Get all values of a string attribute.
    #[wasm_bindgen(js_name = "attribDataString")]
    pub fn attrib_data_string(&self, class: &str, name: &str) -> Option<Vec<String>> {
        self.inner
            .attrib_data_string(Self::parse_class(class), name)
    }

    /// Get the point indices for a specific primitive.
    #[wasm_bindgen(js_name = "primPointIndices")]
    pub fn prim_point_indices(&self, prim_index: u32) -> Vec<u32> {
        let ph = PrimHandle::from_index(prim_index as usize);
        self.inner
            .prim_points(ph)
            .iter()
            .map(|p| p.index() as u32)
            .collect()
    }

    /// Get the number of vertices in a specific primitive.
    #[wasm_bindgen(js_name = "primVertexCount")]
    pub fn prim_vertex_count(&self, prim_index: u32) -> u32 {
        let ph = PrimHandle::from_index(prim_index as usize);
        self.inner.prim_vertices(ph).len() as u32
    }

    /// Get which point a vertex maps to.
    #[wasm_bindgen(js_name = "vertexPoint")]
    pub fn vertex_point(&self, vertex_index: u32) -> u32 {
        self.inner
            .vertex_point(procgeo_core::VertexHandle::from_index(vertex_index as usize))
            .index() as u32
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
// Creation SOPs (additional)
// ---------------------------------------------------------------------------

#[wasm_bindgen]
pub fn revolve(geo: &Geometry, params: Option<JsValue>) -> Result<Geometry, JsError> {
    let p = params.unwrap_or_else(empty_obj);
    let params = procgeo_sops::creation::RevolveParams {
        origin: get_vec3(&p, "origin", [0.0, 0.0, 0.0]),
        axis: get_vec3(&p, "axis", [0.0, 1.0, 0.0]),
        divisions: get_u32(&p, "divisions", 24),
        start_angle: get_f32(&p, "startAngle", 0.0),
        end_angle: get_f32(&p, "endAngle", 360.0),
        end_caps: get_bool(&p, "endCaps", false),
    };
    let inner = procgeo_sops::creation::RevolveSop.execute(&[&geo.inner], &params).map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[wasm_bindgen(js_name = "createMetaball")]
pub fn create_metaball(params: Option<JsValue>) -> Result<Geometry, JsError> {
    let p = params.unwrap_or_else(empty_obj);
    let balls = js_sys::Reflect::get(&p, &"balls".into())
        .ok()
        .and_then(|v| v.dyn_ref::<js_sys::Array>().map(|arr| {
            arr.iter().map(|b| procgeo_sops::creation::MetaballDef {
                center: get_vec3(&b, "center", [0.0, 0.0, 0.0]),
                radius: get_f32(&b, "radius", 1.0),
                weight: get_f32(&b, "weight", 1.0),
            }).collect()
        }))
        .unwrap_or_else(|| vec![procgeo_sops::creation::MetaballDef::default()]);
    let kernel_str = get_str(&p, "kernel", "wyvill");
    let kernel = match kernel_str.as_str() {
        "blinn" => procgeo_sops::creation::MetaballKernel::Blinn,
        "hart" => procgeo_sops::creation::MetaballKernel::Hart,
        _ => procgeo_sops::creation::MetaballKernel::Wyvill,
    };
    let params = procgeo_sops::creation::MetaballParams {
        balls,
        threshold: get_f32(&p, "threshold", 1.0),
        kernel,
        resolution: get_u32(&p, "resolution", 32),
        padding: get_f32(&p, "padding", 0.2),
    };
    let inner = procgeo_sops::creation::MetaballSop.execute(&[], &params).map_err(sop_err)?;
    Ok(Geometry { inner })
}

// ---------------------------------------------------------------------------
// Merge SOP
// ---------------------------------------------------------------------------

/// Merge two geometries into one. Chain calls to merge more:
/// `merge(merge(a, b), c)`.
#[wasm_bindgen]
pub fn merge(a: &Geometry, b: &Geometry) -> Result<Geometry, JsError> {
    let inner = procgeo_sops::merge::MergeSop
        .execute(&[&a.inner, &b.inner], &procgeo_sops::merge::MergeParams)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

// ---------------------------------------------------------------------------
// Reshape SOPs (additional)
// ---------------------------------------------------------------------------

#[wasm_bindgen(js_name = "polyBevel")]
pub fn poly_bevel(geo: &Geometry, params: Option<JsValue>) -> Result<Geometry, JsError> {
    let p = params.unwrap_or_else(empty_obj);
    let params = procgeo_sops::reshape::PolyBevelParams {
        offset: get_f32(&p, "offset", 0.1),
        divisions: get_u32(&p, "divisions", 1),
    };
    let inner = procgeo_sops::reshape::PolyBevelSop.execute(&[&geo.inner], &params).map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[wasm_bindgen(js_name = "polyWire")]
pub fn poly_wire(geo: &Geometry, params: Option<JsValue>) -> Result<Geometry, JsError> {
    let p = params.unwrap_or_else(empty_obj);
    let params = procgeo_sops::reshape::PolyWireParams {
        radius: get_f32(&p, "radius", 0.1),
        divisions: get_u32(&p, "divisions", 8),
    };
    let inner = procgeo_sops::reshape::PolyWireSop.execute(&[&geo.inner], &params).map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[wasm_bindgen(js_name = "polyReduce")]
pub fn poly_reduce(geo: &Geometry, params: Option<JsValue>) -> Result<Geometry, JsError> {
    let p = params.unwrap_or_else(empty_obj);
    let params = procgeo_sops::reshape::PolyReduceParams {
        target_percent: get_f32(&p, "targetPercent", 0.5),
        preserve_boundaries: get_bool(&p, "preserveBoundaries", true),
    };
    let inner = procgeo_sops::reshape::PolyReduceSop.execute(&[&geo.inner], &params).map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[wasm_bindgen(js_name = "polyFill")]
pub fn poly_fill(geo: &Geometry, params: Option<JsValue>) -> Result<Geometry, JsError> {
    let p = params.unwrap_or_else(empty_obj);
    let mode = match get_str(&p, "mode", "single").as_str() {
        "fan" | "triangleFan" => procgeo_sops::reshape::PolyFillMode::TriangleFan,
        _ => procgeo_sops::reshape::PolyFillMode::SinglePolygon,
    };
    let params = procgeo_sops::reshape::PolyFillParams {
        mode,
        smooth: get_f32(&p, "smooth", 0.0),
    };
    let inner = procgeo_sops::reshape::PolyFillSop.execute(&[&geo.inner], &params).map_err(sop_err)?;
    Ok(Geometry { inner })
}

// ---------------------------------------------------------------------------
// Topology SOPs (additional)
// ---------------------------------------------------------------------------

#[wasm_bindgen]
pub fn sort(geo: &Geometry, params: Option<JsValue>) -> Result<Geometry, JsError> {
    let p = params.unwrap_or_else(empty_obj);
    let params = procgeo_sops::topology::SortParams {
        seed: get_u64(&p, "seed", 0),
        ..Default::default()
    };
    let inner = procgeo_sops::topology::SortSop.execute(&[&geo.inner], &params).map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[wasm_bindgen]
pub fn resample(geo: &Geometry, params: Option<JsValue>) -> Result<Geometry, JsError> {
    let p = params.unwrap_or_else(empty_obj);
    let params = procgeo_sops::topology::ResampleParams {
        length: get_f32(&p, "length", 0.1),
        max_segments: get_u32(&p, "maxSegments", 1000),
    };
    let inner = procgeo_sops::topology::ResampleSop.execute(&[&geo.inner], &params).map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[wasm_bindgen]
pub fn connectivity(geo: &Geometry, params: Option<JsValue>) -> Result<Geometry, JsError> {
    let p = params.unwrap_or_else(empty_obj);
    let params = procgeo_sops::topology::ConnectivityParams {
        attrib_name: get_str(&p, "attribName", "class"),
    };
    let inner = procgeo_sops::topology::ConnectivitySop.execute(&[&geo.inner], &params).map_err(sop_err)?;
    Ok(Geometry { inner })
}

// ---------------------------------------------------------------------------
// Utility / Measure SOPs
// ---------------------------------------------------------------------------

#[wasm_bindgen(js_name = "enumerateAttrib")]
pub fn enumerate_attrib(geo: &Geometry, params: Option<JsValue>) -> Result<Geometry, JsError> {
    let p = params.unwrap_or_else(empty_obj);
    let params = procgeo_sops::utility::EnumerateParams {
        name: get_str(&p, "name", "index"),
        start: get_f32(&p, "start", 0.0) as i32,
        ..Default::default()
    };
    let inner = procgeo_sops::utility::EnumerateSop.execute(&[&geo.inner], &params).map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[wasm_bindgen]
pub fn measure(geo: &Geometry, params: Option<JsValue>) -> Result<Geometry, JsError> {
    let p = params.unwrap_or_else(empty_obj);
    let params = procgeo_sops::measure::MeasureParams {
        attrib_name: get_str(&p, "attribName", ""),
        ..Default::default()
    };
    let inner = procgeo_sops::measure::MeasureSop.execute(&[&geo.inner], &params).map_err(sop_err)?;
    Ok(Geometry { inner })
}

// ---------------------------------------------------------------------------
// Delete SOPs
// ---------------------------------------------------------------------------

#[wasm_bindgen]
pub fn blast(geo: &Geometry, params: Option<JsValue>) -> Result<Geometry, JsError> {
    let p = params.unwrap_or_else(empty_obj);
    let entity = match get_str(&p, "entity", "primitives").as_str() {
        "points" => procgeo_sops::delete::BlastEntity::Points,
        _ => procgeo_sops::delete::BlastEntity::Primitives,
    };
    let params = procgeo_sops::delete::BlastParams {
        group_name: get_str(&p, "groupName", ""),
        entity,
        negate: get_bool(&p, "negate", false),
    };
    let inner = procgeo_sops::delete::BlastSop.execute(&[&geo.inner], &params).map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[wasm_bindgen(js_name = "deleteSop")]
pub fn delete_sop(geo: &Geometry, params: Option<JsValue>) -> Result<Geometry, JsError> {
    let p = params.unwrap_or_else(empty_obj);
    let entity = match get_str(&p, "entity", "primitives").as_str() {
        "points" => procgeo_sops::delete::DeleteEntity::Points,
        _ => procgeo_sops::delete::DeleteEntity::Primitives,
    };
    let params = procgeo_sops::delete::DeleteParams {
        entity,
        range_start: get_u32(&p, "rangeStart", 0) as usize,
        range_end: get_u32(&p, "rangeEnd", 0) as usize,
    };
    let inner = procgeo_sops::delete::DeleteSop.execute(&[&geo.inner], &params).map_err(sop_err)?;
    Ok(Geometry { inner })
}

// ---------------------------------------------------------------------------
// Attribute CRUD SOPs
// ---------------------------------------------------------------------------

#[wasm_bindgen(js_name = "attribCreate")]
pub fn attrib_create(geo: &Geometry, params: Option<JsValue>) -> Result<Geometry, JsError> {
    let p = params.unwrap_or_else(empty_obj);
    let value_vector3_arr = get_vec3(&p, "valueVector3", [0.0, 0.0, 0.0]);
    let params = procgeo_sops::attributes::AttribCreateParams {
        name: get_str(&p, "name", "attrib1"),
        class: parse_attrib_class(&get_str(&p, "class", "Point")),
        attrib_type: parse_attrib_type(&get_str(&p, "attribType", "Float")),
        value_int: get_f32(&p, "valueInt", 0.0) as i32,
        value_float: get_f32(&p, "valueFloat", 0.0),
        value_vector3: [value_vector3_arr.x, value_vector3_arr.y, value_vector3_arr.z],
        value_string: get_str(&p, "valueString", ""),
        qualifier: Default::default(),
    };
    let inner = procgeo_sops::attributes::AttribCreateSop.execute(&[&geo.inner], &params).map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[wasm_bindgen(js_name = "attribDelete")]
pub fn attrib_delete(geo: &Geometry, params: Option<JsValue>) -> Result<Geometry, JsError> {
    let p = params.unwrap_or_else(empty_obj);
    let params = procgeo_sops::attributes::AttribDeleteParams {
        name: get_str(&p, "name", "attrib1"),
        class: parse_attrib_class(&get_str(&p, "class", "Point")),
    };
    let inner = procgeo_sops::attributes::AttribDeleteSop.execute(&[&geo.inner], &params).map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[wasm_bindgen(js_name = "attribRename")]
pub fn attrib_rename(geo: &Geometry, params: Option<JsValue>) -> Result<Geometry, JsError> {
    let p = params.unwrap_or_else(empty_obj);
    let params = procgeo_sops::attributes::AttribRenameParams {
        from_name: get_str(&p, "fromName", "attrib1"),
        to_name: get_str(&p, "toName", "attrib2"),
        class: parse_attrib_class(&get_str(&p, "class", "Point")),
    };
    let inner = procgeo_sops::attributes::AttribRenameSop.execute(&[&geo.inner], &params).map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[wasm_bindgen(js_name = "attribPromote")]
pub fn attrib_promote(geo: &Geometry, params: Option<JsValue>) -> Result<Geometry, JsError> {
    let p = params.unwrap_or_else(empty_obj);
    let method = match get_str(&p, "method", "average").as_str() {
        "first" => procgeo_sops::attributes::PromoteMethod::First,
        "last" => procgeo_sops::attributes::PromoteMethod::Last,
        "min" => procgeo_sops::attributes::PromoteMethod::Min,
        "max" => procgeo_sops::attributes::PromoteMethod::Max,
        _ => procgeo_sops::attributes::PromoteMethod::Average,
    };
    let params = procgeo_sops::attributes::AttribPromoteParams {
        name: get_str(&p, "name", "attrib"),
        from_class: parse_attrib_class(&get_str(&p, "fromClass", "Point")),
        to_class: parse_attrib_class(&get_str(&p, "toClass", "Primitive")),
        method,
        delete_original: get_bool(&p, "deleteOriginal", true),
    };
    let inner = procgeo_sops::attributes::AttribPromoteSop.execute(&[&geo.inner], &params).map_err(sop_err)?;
    Ok(Geometry { inner })
}

// ---------------------------------------------------------------------------
// Group SOPs
// ---------------------------------------------------------------------------

#[wasm_bindgen(js_name = "groupCreate")]
pub fn group_create(geo: &Geometry, params: Option<JsValue>) -> Result<Geometry, JsError> {
    let p = params.unwrap_or_else(empty_obj);
    let group_type = match get_str(&p, "groupType", "points").as_str() {
        "primitives" | "prims" => procgeo_sops::groups::GroupType::Primitives,
        _ => procgeo_sops::groups::GroupType::Points,
    };
    let mode = match get_str(&p, "mode", "range").as_str() {
        "boundingBox" | "bbox" => procgeo_sops::groups::GroupCreateMode::BoundingBox,
        "normal" => procgeo_sops::groups::GroupCreateMode::Normal,
        _ => procgeo_sops::groups::GroupCreateMode::Range,
    };
    let params = procgeo_sops::groups::GroupCreateParams {
        name: get_str(&p, "name", "group1"),
        group_type,
        mode,
        range_start: get_u32(&p, "rangeStart", 0) as usize,
        range_end: get_u32(&p, "rangeEnd", u32::MAX) as usize,
        bbox_min: get_vec3(&p, "bboxMin", [f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY]),
        bbox_max: get_vec3(&p, "bboxMax", [f32::INFINITY, f32::INFINITY, f32::INFINITY]),
        normal_direction: get_vec3(&p, "normalDirection", [0.0, 1.0, 0.0]),
        normal_angle: get_f32(&p, "normalAngle", 45.0),
    };
    let inner = procgeo_sops::groups::GroupCreateSop.execute(&[&geo.inner], &params).map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[wasm_bindgen(js_name = "groupCombine")]
pub fn group_combine(geo: &Geometry, params: Option<JsValue>) -> Result<Geometry, JsError> {
    let p = params.unwrap_or_else(empty_obj);
    let operation = match get_str(&p, "operation", "union").as_str() {
        "intersect" => procgeo_sops::groups::GroupBooleanOp::Intersect,
        "subtract" => procgeo_sops::groups::GroupBooleanOp::Subtract,
        _ => procgeo_sops::groups::GroupBooleanOp::Union,
    };
    let group_type = match get_str(&p, "groupType", "points").as_str() {
        "primitives" | "prims" => procgeo_sops::groups::GroupType::Primitives,
        _ => procgeo_sops::groups::GroupType::Points,
    };
    let params = procgeo_sops::groups::GroupCombineParams {
        name_a: get_str(&p, "nameA", "group_a"),
        name_b: get_str(&p, "nameB", "group_b"),
        result: get_str(&p, "result", "group_result"),
        operation,
        group_type,
    };
    let inner = procgeo_sops::groups::GroupCombineSop.execute(&[&geo.inner], &params).map_err(sop_err)?;
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

// ---------------------------------------------------------------------------
// COP Registry
// ---------------------------------------------------------------------------

use procgeo_cops::registry::CopRegistry as WasmCopRegistry;
use procgeo_cops::context::GpuContext as WasmCopGpuContext;

static WASM_COP_REGISTRY: OnceLock<WasmCopRegistry> = OnceLock::new();
static WASM_GPU_CONTEXT: OnceLock<std::sync::Arc<WasmCopGpuContext>> = OnceLock::new();

fn get_wasm_cop_registry() -> &'static WasmCopRegistry {
    WASM_COP_REGISTRY.get_or_init(procgeo_cops::registry::default_cop_registry)
}

fn get_wasm_gpu_context() -> Result<std::sync::Arc<WasmCopGpuContext>, JsError> {
    WASM_GPU_CONTEXT
        .get()
        .map(std::sync::Arc::clone)
        .ok_or_else(|| JsError::new("GPU not initialized — call await pg.initCopGpu() first"))
}

/// Initialize the GPU context for COP image processing.
/// Must be called (and awaited) before using any cop* functions.
#[wasm_bindgen(js_name = "initCopGpu")]
pub async fn init_cop_gpu() -> Result<(), JsError> {
    if WASM_GPU_CONTEXT.get().is_some() {
        return Ok(());
    }
    let ctx = WasmCopGpuContext::new()
        .await
        .map(std::sync::Arc::new)
        .map_err(|e| JsError::new(&format!("GPU init failed: {e}")))?;
    let _ = WASM_GPU_CONTEXT.set(ctx);
    Ok(())
}

#[wasm_bindgen]
pub struct CopImage {
    inner: procgeo_cops::image::Image,
}

#[wasm_bindgen]
impl CopImage {
    #[wasm_bindgen(getter)]
    pub fn width(&self) -> u32 { self.inner.width() }

    #[wasm_bindgen(getter)]
    pub fn height(&self) -> u32 { self.inner.height() }

    #[wasm_bindgen(js_name = "getPixels")]
    pub async fn get_pixels(&self) -> Result<Vec<f32>, JsError> {
        self.inner.to_cpu_async().await.map_err(|e| JsError::new(&format!("{e}")))
    }
}

#[wasm_bindgen(js_name = "executeCopCreate")]
pub fn execute_cop_create(name: &str, params: Option<JsValue>) -> Result<CopImage, JsError> {
    let ctx = get_wasm_gpu_context()?;
    let registry = get_wasm_cop_registry();
    let params_json = match params {
        Some(v) => {
            let obj: serde_json::Value = serde_wasm_bindgen::from_value(v).unwrap_or(serde_json::Value::Object(Default::default()));
            serde_json::to_string(&obj).unwrap_or_default()
        }
        None => "{}".to_string(),
    };
    let inner = registry.execute(name, &ctx, &[], &params_json).map_err(|e| JsError::new(&format!("{e}")))?;
    Ok(CopImage { inner })
}

#[wasm_bindgen(js_name = "executeCop")]
pub fn wasm_execute_cop(name: &str, image: &CopImage, params: Option<JsValue>) -> Result<CopImage, JsError> {
    let ctx = get_wasm_gpu_context()?;
    let registry = get_wasm_cop_registry();
    let params_json = match params {
        Some(v) => {
            let obj: serde_json::Value = serde_wasm_bindgen::from_value(v).unwrap_or(serde_json::Value::Object(Default::default()));
            serde_json::to_string(&obj).unwrap_or_default()
        }
        None => "{}".to_string(),
    };
    let inner = registry.execute(name, &ctx, &[&image.inner], &params_json).map_err(|e| JsError::new(&format!("{e}")))?;
    Ok(CopImage { inner })
}

#[wasm_bindgen(js_name = "executeCopComposite")]
pub fn wasm_execute_cop_composite(name: &str, image_a: &CopImage, image_b: &CopImage, params: Option<JsValue>) -> Result<CopImage, JsError> {
    let ctx = get_wasm_gpu_context()?;
    let registry = get_wasm_cop_registry();
    let params_json = match params {
        Some(v) => {
            let obj: serde_json::Value = serde_wasm_bindgen::from_value(v).unwrap_or(serde_json::Value::Object(Default::default()));
            serde_json::to_string(&obj).unwrap_or_default()
        }
        None => "{}".to_string(),
    };
    let inner = registry.execute(name, &ctx, &[&image_a.inner, &image_b.inner], &params_json).map_err(|e| JsError::new(&format!("{e}")))?;
    Ok(CopImage { inner })
}

#[wasm_bindgen(js_name = "listCops")]
pub fn list_cops_wasm() -> Vec<String> {
    get_wasm_cop_registry().list().into_iter().map(|s| s.to_string()).collect()
}

fn cop_err(e: procgeo_cops::CopError) -> JsError {
    JsError::new(&format!("{e}"))
}

fn get_vec2_wasm(val: &JsValue, key: &str, default: [f32; 2]) -> [f32; 2] {
    let arr = js_sys::Reflect::get(val, &key.into()).ok();
    if let Some(arr) = arr {
        if let Some(arr) = arr.dyn_ref::<js_sys::Array>() {
            return [
                arr.get(0).as_f64().unwrap_or(default[0] as f64) as f32,
                arr.get(1).as_f64().unwrap_or(default[1] as f64) as f32,
            ];
        }
    }
    default
}

fn get_vec4_wasm(val: &JsValue, key: &str, default: [f32; 4]) -> [f32; 4] {
    let arr = js_sys::Reflect::get(val, &key.into()).ok();
    if let Some(arr) = arr {
        if let Some(arr) = arr.dyn_ref::<js_sys::Array>() {
            return [
                arr.get(0).as_f64().unwrap_or(default[0] as f64) as f32,
                arr.get(1).as_f64().unwrap_or(default[1] as f64) as f32,
                arr.get(2).as_f64().unwrap_or(default[2] as f64) as f32,
                arr.get(3).as_f64().unwrap_or(default[3] as f64) as f32,
            ];
        }
    }
    default
}

// ---------------------------------------------------------------------------
// Dedicated COP functions
// ---------------------------------------------------------------------------

// --- Generators (0 inputs) ---

#[wasm_bindgen(js_name = "copConstant")]
pub fn cop_constant_wasm(params: Option<JsValue>) -> Result<CopImage, JsError> {
    use procgeo_cops::Cop;
    use procgeo_cops::generator::{ConstantCop, ConstantParams};

    let p = params.unwrap_or_else(empty_obj);
    let cop_params = ConstantParams {
        color: get_vec4_wasm(&p, "color", [0.0, 0.0, 0.0, 1.0]),
        width: get_u32(&p, "width", 256),
        height: get_u32(&p, "height", 256),
    };

    let ctx = get_wasm_gpu_context()?;
    let carrier = procgeo_cops::image::Image::empty(ctx);
    let inner = ConstantCop.execute(&[&carrier], &cop_params).map_err(cop_err)?;
    Ok(CopImage { inner })
}

#[wasm_bindgen(js_name = "copCheckerboard")]
pub fn cop_checkerboard_wasm(params: Option<JsValue>) -> Result<CopImage, JsError> {
    use procgeo_cops::Cop;
    use procgeo_cops::generator::{CheckerboardCop, CheckerboardParams};

    let p = params.unwrap_or_else(empty_obj);
    let cop_params = CheckerboardParams {
        color_a: get_vec4_wasm(&p, "colorA", [0.0, 0.0, 0.0, 1.0]),
        color_b: get_vec4_wasm(&p, "colorB", [1.0, 1.0, 1.0, 1.0]),
        frequency: get_vec2_wasm(&p, "frequency", [8.0, 8.0]),
        width: get_u32(&p, "width", 256),
        height: get_u32(&p, "height", 256),
    };

    let ctx = get_wasm_gpu_context()?;
    let carrier = procgeo_cops::image::Image::empty(ctx);
    let inner = CheckerboardCop.execute(&[&carrier], &cop_params).map_err(cop_err)?;
    Ok(CopImage { inner })
}

#[wasm_bindgen(js_name = "copNoise")]
pub fn cop_noise_wasm(params: Option<JsValue>) -> Result<CopImage, JsError> {
    use procgeo_cops::Cop;
    use procgeo_cops::generator::{NoiseCop, NoiseParams, NoiseType};

    let p = params.unwrap_or_else(empty_obj);
    let noise_type_str = get_str(&p, "noiseType", "perlin");
    let noise_type = match noise_type_str.to_lowercase().as_str() {
        "simplex" => NoiseType::Simplex,
        "worley" => NoiseType::Worley,
        _ => NoiseType::Perlin,
    };

    let cop_params = NoiseParams {
        noise_type,
        frequency: get_f32(&p, "frequency", 4.0),
        octaves: get_u32(&p, "octaves", 4),
        lacunarity: get_f32(&p, "lacunarity", 2.0),
        gain: get_f32(&p, "gain", 0.5),
        amplitude: get_f32(&p, "amplitude", 1.0),
        offset: get_vec2_wasm(&p, "offset", [0.0, 0.0]),
        seed: get_u32(&p, "seed", 0),
        width: get_u32(&p, "width", 256),
        height: get_u32(&p, "height", 256),
    };

    let ctx = get_wasm_gpu_context()?;
    let carrier = procgeo_cops::image::Image::empty(ctx);
    let inner = NoiseCop.execute(&[&carrier], &cop_params).map_err(cop_err)?;
    Ok(CopImage { inner })
}

#[wasm_bindgen(js_name = "copRamp")]
pub fn cop_ramp_wasm(params: Option<JsValue>) -> Result<CopImage, JsError> {
    use procgeo_cops::Cop;
    use procgeo_cops::generator::{RampCop, RampParams, RampType, RampStop};

    let p = params.unwrap_or_else(empty_obj);
    let ramp_type_str = get_str(&p, "rampType", "linear");
    let ramp_type = match ramp_type_str.to_lowercase().as_str() {
        "radial" => RampType::Radial,
        "box" => RampType::Box,
        "diagonal" => RampType::Diagonal,
        _ => RampType::Linear,
    };

    // Parse stops array: each element is { position: f32, color: [f32; 4] }
    let stops: Vec<RampStop> = js_sys::Reflect::get(&p, &"stops".into())
        .ok()
        .and_then(|v| v.dyn_ref::<js_sys::Array>().map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    let pos = js_sys::Reflect::get(&item, &"position".into())
                        .ok()
                        .and_then(|v| v.as_f64())
                        .map(|v| v as f32)
                        .unwrap_or(0.0);
                    let color = get_vec4_wasm(&item, "color", [0.0, 0.0, 0.0, 1.0]);
                    Some((pos, color))
                })
                .collect()
        }))
        .unwrap_or_else(|| vec![
            (0.0, [0.0, 0.0, 0.0, 1.0]),
            (1.0, [1.0, 1.0, 1.0, 1.0]),
        ]);

    let cop_params = RampParams {
        ramp_type,
        stops,
        width: get_u32(&p, "width", 256),
        height: get_u32(&p, "height", 256),
    };

    let ctx = get_wasm_gpu_context()?;
    let carrier = procgeo_cops::image::Image::empty(ctx);
    let inner = RampCop.execute(&[&carrier], &cop_params).map_err(cop_err)?;
    Ok(CopImage { inner })
}

#[wasm_bindgen(js_name = "copLoadImage")]
pub fn cop_load_image_wasm(params: Option<JsValue>) -> Result<CopImage, JsError> {
    use procgeo_cops::Cop;
    use procgeo_cops::generator::{LoadImageCop, LoadImageParams};

    let p = params.unwrap_or_else(empty_obj);
    let cop_params = LoadImageParams {
        path: get_str(&p, "path", ""),
    };

    let ctx = get_wasm_gpu_context()?;
    let carrier = procgeo_cops::image::Image::empty(ctx);
    let inner = LoadImageCop.execute(&[&carrier], &cop_params).map_err(cop_err)?;
    Ok(CopImage { inner })
}

// --- Filters (1 input) ---

#[wasm_bindgen(js_name = "copBlur")]
pub fn cop_blur_wasm(image: &CopImage, params: Option<JsValue>) -> Result<CopImage, JsError> {
    use procgeo_cops::Cop;
    use procgeo_cops::filter::blur::{BlurCop, BlurParams, BlurType};

    let p = params.unwrap_or_else(empty_obj);
    let blur_type_str = get_str(&p, "blurType", "gaussian");
    let blur_type = match blur_type_str.to_lowercase().as_str() {
        "box" => BlurType::Box,
        _ => BlurType::Gaussian,
    };

    let cop_params = BlurParams {
        blur_type,
        radius_x: get_f32(&p, "radiusX", 4.0),
        radius_y: get_f32(&p, "radiusY", 4.0),
    };

    let inner = BlurCop.execute(&[&image.inner], &cop_params).map_err(cop_err)?;
    Ok(CopImage { inner })
}

#[wasm_bindgen(js_name = "copFlip")]
pub fn cop_flip_wasm(image: &CopImage, params: Option<JsValue>) -> Result<CopImage, JsError> {
    use procgeo_cops::Cop;
    use procgeo_cops::filter::flip::{FlipCop, FlipParams};

    let p = params.unwrap_or_else(empty_obj);
    let cop_params = FlipParams {
        horizontal: get_bool(&p, "horizontal", false),
        vertical: get_bool(&p, "vertical", true),
    };

    let inner = FlipCop.execute(&[&image.inner], &cop_params).map_err(cop_err)?;
    Ok(CopImage { inner })
}

#[wasm_bindgen(js_name = "copMirror")]
pub fn cop_mirror_wasm(image: &CopImage, params: Option<JsValue>) -> Result<CopImage, JsError> {
    use procgeo_cops::Cop;
    use procgeo_cops::filter::mirror::{MirrorCop, MirrorParams, MirrorAxis};

    let p = params.unwrap_or_else(empty_obj);
    let axis_str = get_str(&p, "axis", "x");
    let axis = match axis_str.to_lowercase().as_str() {
        "y" => MirrorAxis::Y,
        _ => MirrorAxis::X,
    };

    let cop_params = MirrorParams {
        axis,
        offset: get_f32(&p, "offset", 0.5),
    };

    let inner = MirrorCop.execute(&[&image.inner], &cop_params).map_err(cop_err)?;
    Ok(CopImage { inner })
}

#[wasm_bindgen(js_name = "copChannelSwap")]
pub fn cop_channel_swap_wasm(image: &CopImage, params: Option<JsValue>) -> Result<CopImage, JsError> {
    use procgeo_cops::Cop;
    use procgeo_cops::filter::channel_swap::{ChannelSwapCop, ChannelSwapParams, Channel};

    fn parse_channel(s: &str) -> Channel {
        match s.to_lowercase().as_str() {
            "r" => Channel::R,
            "g" => Channel::G,
            "b" => Channel::B,
            "a" => Channel::A,
            "one" | "1" => Channel::One,
            "zero" | "0" => Channel::Zero,
            _ => Channel::R,
        }
    }

    let p = params.unwrap_or_else(empty_obj);
    let cop_params = ChannelSwapParams {
        r: parse_channel(&get_str(&p, "r", "r")),
        g: parse_channel(&get_str(&p, "g", "g")),
        b: parse_channel(&get_str(&p, "b", "b")),
        a: parse_channel(&get_str(&p, "a", "a")),
    };

    let inner = ChannelSwapCop.execute(&[&image.inner], &cop_params).map_err(cop_err)?;
    Ok(CopImage { inner })
}

#[wasm_bindgen(js_name = "copResize")]
pub fn cop_resize_wasm(image: &CopImage, params: Option<JsValue>) -> Result<CopImage, JsError> {
    use procgeo_cops::Cop;
    use procgeo_cops::filter::resize::{ResizeCop, ResizeParams};
    use procgeo_cops::FilterMode;

    let p = params.unwrap_or_else(empty_obj);
    let filter_str = get_str(&p, "filter", "nearest");
    let filter = match filter_str.to_lowercase().as_str() {
        "bilinear" => FilterMode::Bilinear,
        _ => FilterMode::Nearest,
    };

    let cop_params = ResizeParams {
        width: get_u32(&p, "width", 256),
        height: get_u32(&p, "height", 256),
        filter,
    };

    let inner = ResizeCop.execute(&[&image.inner], &cop_params).map_err(cop_err)?;
    Ok(CopImage { inner })
}

#[wasm_bindgen(js_name = "copRotate")]
pub fn cop_rotate_wasm(image: &CopImage, params: Option<JsValue>) -> Result<CopImage, JsError> {
    use procgeo_cops::Cop;
    use procgeo_cops::filter::rotate::{RotateCop, RotateParams};
    use procgeo_cops::FilterMode;

    let p = params.unwrap_or_else(empty_obj);
    let filter_str = get_str(&p, "filter", "nearest");
    let filter = match filter_str.to_lowercase().as_str() {
        "bilinear" => FilterMode::Bilinear,
        _ => FilterMode::Nearest,
    };

    let cop_params = RotateParams {
        angle: get_f32(&p, "angle", 0.0),
        center: get_vec2_wasm(&p, "center", [0.5, 0.5]),
        filter,
    };

    let inner = RotateCop.execute(&[&image.inner], &cop_params).map_err(cop_err)?;
    Ok(CopImage { inner })
}

#[wasm_bindgen(js_name = "copSwirl")]
pub fn cop_swirl_wasm(image: &CopImage, params: Option<JsValue>) -> Result<CopImage, JsError> {
    use procgeo_cops::Cop;
    use procgeo_cops::filter::swirl::{SwirlCop, SwirlParams};

    let p = params.unwrap_or_else(empty_obj);
    let cop_params = SwirlParams {
        center: get_vec2_wasm(&p, "center", [0.5, 0.5]),
        angle: get_f32(&p, "angle", 90.0),
        radius: get_f32(&p, "radius", 0.5),
    };

    let inner = SwirlCop.execute(&[&image.inner], &cop_params).map_err(cop_err)?;
    Ok(CopImage { inner })
}

// --- Composite (2 inputs) ---

#[wasm_bindgen(js_name = "copComposite")]
pub fn cop_composite_wasm(a: &CopImage, b: &CopImage, params: Option<JsValue>) -> Result<CopImage, JsError> {
    use procgeo_cops::Cop;
    use procgeo_cops::composite::{CompositeCop, CompositeParams, CompOp};

    let p = params.unwrap_or_else(empty_obj);
    let op_str = get_str(&p, "operation", "over");
    let operation = match op_str.to_lowercase().as_str() {
        "add" => CompOp::Add,
        "multiply" => CompOp::Multiply,
        "screen" => CompOp::Screen,
        "subtract" => CompOp::Subtract,
        "difference" => CompOp::Difference,
        "min" => CompOp::Min,
        "max" => CompOp::Max,
        _ => CompOp::Over,
    };

    let cop_params = CompositeParams {
        operation,
        mix: get_f32(&p, "mix", 1.0),
    };

    let inner = CompositeCop.execute(&[&a.inner, &b.inner], &cop_params).map_err(cop_err)?;
    Ok(CopImage { inner })
}

// --- Custom ---

#[wasm_bindgen(js_name = "copCustomShader")]
pub fn cop_custom_shader_wasm(
    input_a: Option<CopImage>,
    input_b: Option<CopImage>,
    params: Option<JsValue>,
) -> Result<CopImage, JsError> {
    use procgeo_cops::Cop;
    use procgeo_cops::custom::{CustomShaderCop, CustomShaderParams, ShaderLang};

    let p = params.unwrap_or_else(empty_obj);
    let lang_str = get_str(&p, "language", "wgsl");
    let language = match lang_str.to_lowercase().as_str() {
        "glsl" => ShaderLang::Glsl,
        _ => ShaderLang::Wgsl,
    };

    let cop_params = CustomShaderParams {
        source: get_str(&p, "source", ""),
        language,
        uniforms: std::collections::HashMap::new(),
        width: get_u32(&p, "width", 256),
        height: get_u32(&p, "height", 256),
    };

    // Build the inputs slice from optional images, or fall back to a carrier.
    let ctx = get_wasm_gpu_context()?;
    let carrier = procgeo_cops::image::Image::empty(ctx);

    let mut input_refs: Vec<&procgeo_cops::image::Image> = Vec::new();
    match (&input_a, &input_b) {
        (Some(a), Some(b)) => {
            input_refs.push(&a.inner);
            input_refs.push(&b.inner);
        }
        (Some(a), None) => {
            input_refs.push(&a.inner);
        }
        _ => {
            // No image inputs — use carrier so the COP gets a GPU context.
            input_refs.push(&carrier);
        }
    }

    let inner = CustomShaderCop.execute(&input_refs, &cop_params).map_err(cop_err)?;
    Ok(CopImage { inner })
}
