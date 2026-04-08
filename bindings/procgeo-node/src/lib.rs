#![deny(clippy::all)]

use std::sync::OnceLock;

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

    #[napi]
    pub fn add_point(&mut self, x: f64, y: f64, z: f64) -> u32 {
        self.inner.add_point(glam::Vec3::new(x as f32, y as f32, z as f32)).index() as u32
    }

    #[napi]
    pub fn set_point_pos(&mut self, index: u32, x: f64, y: f64, z: f64) {
        self.inner.set_point_pos(
            procgeo_core::PointHandle::from_index(index as usize),
            glam::Vec3::new(x as f32, y as f32, z as f32),
        );
    }

    #[napi]
    pub fn add_face(&mut self, point_indices: Vec<u32>) -> u32 {
        let handles: Vec<procgeo_core::PointHandle> = point_indices.iter().map(|&i| procgeo_core::PointHandle::from_index(i as usize)).collect();
        self.inner.add_face(&handles).index() as u32
    }

    #[napi]
    pub fn add_polyline(&mut self, point_indices: Vec<u32>) -> u32 {
        let handles: Vec<procgeo_core::PointHandle> = point_indices.iter().map(|&i| procgeo_core::PointHandle::from_index(i as usize)).collect();
        self.inner.add_polyline(&handles).index() as u32
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

    // -- Attribute introspection (spreadsheet / debugging) --

    #[napi]
    pub fn attrib_names(&self, class: String) -> Vec<String> {
        self.inner
            .attrib_names(parse_attrib_class(&class))
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    #[napi]
    pub fn attrib_type(&self, class: String, name: String) -> Option<String> {
        self.inner
            .attrib_type(parse_attrib_class(&class), &name)
            .map(|t| format!("{t:?}"))
    }

    #[napi]
    pub fn attrib_size(&self, class: String, name: String) -> Option<u32> {
        self.inner
            .attrib_size(parse_attrib_class(&class), &name)
            .map(|s| s as u32)
    }

    #[napi]
    pub fn attrib_data(&self, class: String, name: String) -> Option<Vec<f64>> {
        self.inner
            .attrib_data_f64(parse_attrib_class(&class), &name)
    }

    #[napi]
    pub fn attrib_data_string(&self, class: String, name: String) -> Option<Vec<String>> {
        self.inner
            .attrib_data_string(parse_attrib_class(&class), &name)
    }

    #[napi]
    pub fn prim_point_indices(&self, prim_index: u32) -> Vec<u32> {
        let ph = procgeo_core::PrimHandle::from_index(prim_index as usize);
        self.inner
            .prim_points(ph)
            .iter()
            .map(|p| p.index() as u32)
            .collect()
    }

    #[napi]
    pub fn prim_vertex_count(&self, prim_index: u32) -> u32 {
        let ph = procgeo_core::PrimHandle::from_index(prim_index as usize);
        self.inner.prim_vertices(ph).len() as u32
    }

    #[napi]
    pub fn vertex_point(&self, vertex_index: u32) -> u32 {
        self.inner
            .vertex_point(procgeo_core::VertexHandle::from_index(vertex_index as usize))
            .index() as u32
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

#[napi]
pub fn revolve(geo: &Geometry, params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let params = procgeo_sops::creation::RevolveParams {
        origin: get_vec3(&p, "origin", [0.0, 0.0, 0.0]),
        axis: get_vec3(&p, "axis", [0.0, 1.0, 0.0]),
        divisions: get_u32(&p, "divisions", 24),
        start_angle: get_f32(&p, "start_angle", 0.0),
        end_angle: get_f32(&p, "end_angle", 360.0),
        end_caps: get_bool(&p, "end_caps", false),
    };
    let inner = procgeo_sops::creation::RevolveSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[napi]
pub fn create_metaball(params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let balls = p
        .get("balls")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .map(|b| procgeo_sops::creation::MetaballDef {
                    center: get_vec3(b, "center", [0.0, 0.0, 0.0]),
                    radius: get_f32(b, "radius", 1.0),
                    weight: get_f32(b, "weight", 1.0),
                })
                .collect()
        })
        .unwrap_or_else(|| vec![procgeo_sops::creation::MetaballDef::default()]);
    let kernel_str = p
        .get("kernel")
        .and_then(|v| v.as_str())
        .unwrap_or("wyvill");
    let kernel = match kernel_str {
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
    let inner = procgeo_sops::creation::MetaballSop
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
pub fn bend(geo: &Geometry, params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let bend_mode = match p.get("bend_mode").and_then(|v| v.as_str()).unwrap_or("angle") {
        "direction" => procgeo_sops::deform::BendMode::Direction,
        _ => procgeo_sops::deform::BendMode::Angle,
    };
    let taper_mode = match p.get("taper_mode").and_then(|v| v.as_str()).unwrap_or("linear") {
        "smooth" => procgeo_sops::deform::TaperMode::Smooth,
        _ => procgeo_sops::deform::TaperMode::Linear,
    };
    let taper_ramp = p
        .get("taper_ramp")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|pair| {
                    let pair = pair.as_array()?;
                    let pos = pair.get(0)?.as_f64()? as f32;
                    let val = pair.get(1)?.as_f64()? as f32;
                    Some((pos, val))
                })
                .collect()
        })
        .unwrap_or_else(|| vec![(0.0, 0.5), (1.0, 0.5)]);
    let params = procgeo_sops::deform::BendParams {
        group: p.get("group").and_then(|v| v.as_str()).map(String::from),
        mask_attrib: p.get("mask_attrib").and_then(|v| v.as_str()).map(String::from),
        enable_deformation: get_bool(&p, "enable_deformation", true),
        limit_to_capture_region: get_bool(&p, "limit_to_capture_region", true),
        deform_both_directions: get_bool(&p, "deform_both_directions", false),
        bend_enable: get_bool(&p, "bend_enable", false),
        bend_mode,
        bend_angle: get_f32(&p, "bend_angle", 0.0),
        bend_goal_direction: get_vec3(&p, "bend_goal_direction", [0.0, 0.0, 1.0]),
        twist_enable: get_bool(&p, "twist_enable", false),
        twist_angle: get_f32(&p, "twist_angle", 0.0),
        twist_continuous_both: get_bool(&p, "twist_continuous_both", false),
        length_scale_enable: get_bool(&p, "length_scale_enable", false),
        length_scale: get_f32(&p, "length_scale", 1.0),
        preserve_volume: get_bool(&p, "preserve_volume", false),
        taper_enable: get_bool(&p, "taper_enable", false),
        taper_along: [
            get_bool(&p, "taper_along_x", true),
            get_bool(&p, "taper_along_z", true),
        ],
        taper_mode,
        taper_value: get_f32(&p, "taper_value", 1.0),
        squish: get_f32(&p, "squish", 1.0),
        squish_pivot: get_f32(&p, "squish_pivot", 0.5),
        taper_ramp_enable: get_bool(&p, "taper_ramp_enable", false),
        taper_ramp,
        up_vector: get_vec3(&p, "up_vector", [0.0, 1.0, 0.0]),
        up_vector_angle: get_f32(&p, "up_vector_angle", 0.0),
        capture_origin: get_vec3(&p, "capture_origin", [0.0, 0.0, 0.0]),
        capture_direction: get_vec3(&p, "capture_direction", [0.0, 1.0, 0.0]),
        capture_length: get_f32(&p, "capture_length", 1.0),
        output_attrib: p.get("output_attrib").and_then(|v| v.as_str()).map(String::from),
        attribs_to_transform: p
            .get("attribs_to_transform")
            .and_then(|v| v.as_str())
            .unwrap_or("*")
            .to_string(),
        recompute_normals: get_bool(&p, "recompute_normals", true),
        preserve_normal_length: get_bool(&p, "preserve_normal_length", false),
    };
    let inner = procgeo_sops::deform::BendSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[napi]
pub fn point_deform(
    geo: &Geometry,
    rest_lattice: &Geometry,
    deformed_lattice: &Geometry,
    params: Option<serde_json::Value>,
) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let params = procgeo_sops::deform::PointDeformParams {
        group: p.get("group").and_then(|v| v.as_str()).map(String::from),
        mode: match get_str(&p, "mode", "capture_and_deform") {
            "capture" => procgeo_sops::deform::PointDeformMode::Capture,
            "deform" => procgeo_sops::deform::PointDeformMode::Deform,
            _ => procgeo_sops::deform::PointDeformMode::CaptureAndDeform,
        },
        radius: get_f32(&p, "radius", 1.0),
        min_points: get_u32(&p, "min_points", 1),
        max_points: get_u32(&p, "max_points", 10),
        piece_attrib: p.get("piece_attrib").and_then(|v| v.as_str()).map(String::from),
        rigid_projection: get_bool(&p, "rigid_projection", true),
        mask: get_f32(&p, "mask", 1.0),
        mask_attrib: p.get("mask_attrib").and_then(|v| v.as_str()).map(String::from),
        recompute_normals: get_bool(&p, "recompute_normals", true),
        attribs_to_transform: get_str(&p, "attribs_to_transform", "*").to_string(),
        delete_capture_attribs: get_bool(&p, "delete_capture_attribs", true),
    };
    let inner = procgeo_sops::deform::PointDeformSop
        .execute(&[&geo.inner, &rest_lattice.inner, &deformed_lattice.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[napi]
pub fn boolean_op(
    a: &Geometry,
    b: &Geometry,
    params: Option<serde_json::Value>,
) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let operation = match get_str(&p, "operation", "union") {
        "intersect" => procgeo_sops::boolean::BooleanOp::Intersect,
        "subtract" => procgeo_sops::boolean::BooleanOp::Subtract,
        "shatter" => procgeo_sops::boolean::BooleanOp::Shatter,
        "seam" => procgeo_sops::boolean::BooleanOp::Seam,
        "detect" => procgeo_sops::boolean::BooleanOp::Detect,
        "resolve" => procgeo_sops::boolean::BooleanOp::Resolve,
        "custom" => procgeo_sops::boolean::BooleanOp::Custom,
        _ => procgeo_sops::boolean::BooleanOp::Union,
    };
    let treat_a_as = match get_str(&p, "treat_a_as", "solid") {
        "surface" => procgeo_sops::boolean::BooleanTreatAs::Surface,
        _ => procgeo_sops::boolean::BooleanTreatAs::Solid,
    };
    let treat_b_as = match get_str(&p, "treat_b_as", "solid") {
        "surface" => procgeo_sops::boolean::BooleanTreatAs::Surface,
        _ => procgeo_sops::boolean::BooleanTreatAs::Solid,
    };
    let detriangulate = match get_str(&p, "detriangulate", "all") {
        "only_unchanged" => procgeo_sops::boolean::Detriangulate::OnlyUnchanged,
        "none" => procgeo_sops::boolean::Detriangulate::None,
        _ => procgeo_sops::boolean::Detriangulate::All,
    };
    let custom_match = match get_str(&p, "custom_match", "both") {
        "a" => procgeo_sops::boolean::CustomMatch::A,
        "b" => procgeo_sops::boolean::CustomMatch::B,
        "exactly_one" => procgeo_sops::boolean::CustomMatch::ExactlyOne,
        _ => procgeo_sops::boolean::CustomMatch::Both,
    };
    let a_depth_min = p.get("a_depth_min").and_then(|v| v.as_i64()).unwrap_or(1) as i32;
    let a_depth_max = p.get("a_depth_max").and_then(|v| v.as_i64()).unwrap_or(9999) as i32;
    let b_depth_min = p.get("b_depth_min").and_then(|v| v.as_i64()).unwrap_or(1) as i32;
    let b_depth_max = p.get("b_depth_max").and_then(|v| v.as_i64()).unwrap_or(9999) as i32;
    let params = procgeo_sops::boolean::BooleanParams {
        group_a: p.get("group_a").and_then(|v| v.as_str()).map(String::from),
        group_b: p.get("group_b").and_then(|v| v.as_str()).map(String::from),
        treat_a_as,
        treat_b_as,
        operation,
        detriangulate,
        custom_match,
        collapse_tiny_edges: get_bool(&p, "collapse_tiny_edges", true),
        resolve_self_a: get_bool(&p, "resolve_self_a", false),
        resolve_self_b: get_bool(&p, "resolve_self_b", false),
        assume_seam_flat: get_bool(&p, "assume_seam_flat", true),
        unique_seam_points: get_bool(&p, "unique_seam_points", false),
        edge_length_threshold: get_f32(&p, "edge_length_threshold", 1e-5),
        a_depth_range: [a_depth_min, a_depth_max],
        b_depth_range: [b_depth_min, b_depth_max],
        merge_adjacent: get_bool(&p, "merge_adjacent", true),
        generate_aa_seams: get_bool(&p, "generate_aa_seams", false),
        generate_bb_seams: get_bool(&p, "generate_bb_seams", false),
        generate_ab_seams: get_bool(&p, "generate_ab_seams", true),
        a_inside_b_group: p.get("a_inside_b_group").and_then(|v| v.as_str()).map(String::from),
        a_outside_b_group: p.get("a_outside_b_group").and_then(|v| v.as_str()).map(String::from),
        b_inside_a_group: p.get("b_inside_a_group").and_then(|v| v.as_str()).map(String::from),
        b_outside_a_group: p.get("b_outside_a_group").and_then(|v| v.as_str()).map(String::from),
        aa_seam_edge_group: p.get("aa_seam_edge_group").and_then(|v| v.as_str()).map(String::from),
        bb_seam_edge_group: p.get("bb_seam_edge_group").and_then(|v| v.as_str()).map(String::from),
        ab_seam_edge_group: p.get("ab_seam_edge_group").and_then(|v| v.as_str()).map(String::from),
    };
    let inner = procgeo_sops::boolean::BooleanSop
        .execute(&[&a.inner, &b.inner], &params)
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
        mode: procgeo_sops::reshape::SubdivideMode::default(),
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
            &procgeo_sops::copy::CopyToPointsParams::default(),
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
pub fn poly_bevel(geo: &Geometry, params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let params = procgeo_sops::reshape::PolyBevelParams {
        offset: get_f32(&p, "offset", 0.1),
        divisions: get_u32(&p, "divisions", 1),
    };
    let inner = procgeo_sops::reshape::PolyBevelSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[napi]
pub fn poly_wire(geo: &Geometry, params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let params = procgeo_sops::reshape::PolyWireParams {
        radius: get_f32(&p, "radius", 0.1),
        divisions: get_u32(&p, "divisions", 8),
    };
    let inner = procgeo_sops::reshape::PolyWireSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[napi]
pub fn poly_reduce(geo: &Geometry, params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let params = procgeo_sops::reshape::PolyReduceParams {
        target_percent: get_f32(&p, "target_percent", 0.5),
        preserve_boundaries: get_bool(&p, "preserve_boundaries", true),
    };
    let inner = procgeo_sops::reshape::PolyReduceSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[napi]
pub fn poly_fill(geo: &Geometry, params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let mode = match p.get("mode").and_then(|v| v.as_str()).unwrap_or("single") {
        "fan" | "triangle_fan" => procgeo_sops::reshape::PolyFillMode::TriangleFan,
        _ => procgeo_sops::reshape::PolyFillMode::SinglePolygon,
    };
    let params = procgeo_sops::reshape::PolyFillParams {
        mode,
        smooth: get_f32(&p, "smooth", 0.0),
    };
    let inner = procgeo_sops::reshape::PolyFillSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[napi]
pub fn quad_remesh(geo: &Geometry, params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let target_mode = match p.get("target_mode").and_then(|v| v.as_str()).unwrap_or("face_count") {
        "vertex_count" => procgeo_sops::reshape::QuadRemeshTarget::VertexCount,
        "edge_length" => procgeo_sops::reshape::QuadRemeshTarget::EdgeLength,
        _ => procgeo_sops::reshape::QuadRemeshTarget::FaceCount,
    };
    let mode = match p.get("mode").and_then(|v| v.as_str()).unwrap_or("intrinsic") {
        "extrinsic" => procgeo_sops::reshape::QuadRemeshMode::Extrinsic,
        _ => procgeo_sops::reshape::QuadRemeshMode::Intrinsic,
    };
    let params = procgeo_sops::reshape::QuadRemeshParams {
        target_mode,
        target_count: get_u32(&p, "target_count", 1000),
        target_edge_length: p.get("target_edge_length").and_then(|v| v.as_f64()).unwrap_or(0.1),
        seed: p.get("seed").and_then(|v| v.as_u64()),
        mode,
    };
    let inner = procgeo_sops::reshape::QuadRemeshSop
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
// Delete SOPs
// ---------------------------------------------------------------------------

#[napi]
pub fn blast(geo: &Geometry, params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let entity = match get_str(&p, "entity", "primitives") {
        "points" => procgeo_sops::delete::BlastEntity::Points,
        _ => procgeo_sops::delete::BlastEntity::Primitives,
    };
    let params = procgeo_sops::delete::BlastParams {
        group_name: get_str(&p, "group_name", "").to_string(),
        entity,
        negate: get_bool(&p, "negate", false),
    };
    let inner = procgeo_sops::delete::BlastSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[napi]
pub fn delete_geo(geo: &Geometry, params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let entity = match get_str(&p, "entity", "primitives") {
        "points" => procgeo_sops::delete::DeleteEntity::Points,
        _ => procgeo_sops::delete::DeleteEntity::Primitives,
    };
    let params = procgeo_sops::delete::DeleteParams {
        entity,
        range_start: get_u32(&p, "range_start", 0) as usize,
        range_end: get_u32(&p, "range_end", 0) as usize,
    };
    let inner = procgeo_sops::delete::DeleteSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

// ---------------------------------------------------------------------------
// Voronoi Fracture
// ---------------------------------------------------------------------------

#[napi]
pub fn voronoi_fracture(geo: &Geometry, points: &Geometry, params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let params = procgeo_sops::voronoi::VoronoiFractureParams {
        cut_plane_offset: get_f32(&p, "cut_plane_offset", 0.0),
        create_inside_faces: get_bool(&p, "create_inside_faces", true),
    };
    let inner = procgeo_sops::voronoi::VoronoiFractureSop
        .execute(&[&geo.inner, &points.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

// ---------------------------------------------------------------------------
// Attribute SOPs
// ---------------------------------------------------------------------------

fn get_str<'a>(obj: &'a serde_json::Value, key: &str, default: &'a str) -> &'a str {
    obj.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or(default)
}

fn parse_attrib_class(s: &str) -> procgeo_core::AttribClass {
    serde_json::from_str(&format!("\"{}\"", s)).unwrap_or(procgeo_core::AttribClass::Point)
}

fn parse_attrib_type(s: &str) -> procgeo_core::AttribType {
    serde_json::from_str(&format!("\"{}\"", s)).unwrap_or(procgeo_core::AttribType::Float)
}

#[napi]
pub fn attrib_transfer(dest: &Geometry, source: &Geometry, params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let params = procgeo_sops::attributes::AttribTransferParams {
        attrib_name: get_str(&p, "attrib_name", "attrib").to_string(),
        class: parse_attrib_class(get_str(&p, "class", "Point")),
        attrib_type: parse_attrib_type(get_str(&p, "attrib_type", "Float")),
        max_samples: get_u32(&p, "max_samples", 1),
        distance_threshold: get_f32(&p, "distance_threshold", f32::MAX),
    };
    let inner = procgeo_sops::attributes::AttribTransferSop
        .execute(&[&dest.inner, &source.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[napi]
pub fn attrib_copy(dest: &Geometry, source: Option<&Geometry>, params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let params = procgeo_sops::attributes::AttribCopyParams {
        attrib_name: get_str(&p, "attrib_name", "attrib").to_string(),
        class: parse_attrib_class(get_str(&p, "class", "Point")),
        attrib_type: parse_attrib_type(get_str(&p, "attrib_type", "Float")),
        new_name: get_str(&p, "new_name", "").to_string(),
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

#[napi]
pub fn attrib_randomize(geo: &Geometry, params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let distribution_str = get_str(&p, "distribution", "Uniform");
    let distribution = serde_json::from_str::<procgeo_sops::attributes::RandomDistribution>(
        &format!("\"{}\"", distribution_str),
    ).unwrap_or(procgeo_sops::attributes::RandomDistribution::Uniform);
    let operation_str = get_str(&p, "operation", "Set");
    let operation = serde_json::from_str::<procgeo_sops::attributes::RandomOperation>(
        &format!("\"{}\"", operation_str),
    ).unwrap_or(procgeo_sops::attributes::RandomOperation::Set);
    let params = procgeo_sops::attributes::AttribRandomizeParams {
        attrib_name: get_str(&p, "attrib_name", "randomize").to_string(),
        class: parse_attrib_class(get_str(&p, "class", "Point")),
        attrib_type: parse_attrib_type(get_str(&p, "attrib_type", "Float")),
        distribution,
        operation,
        seed: p.get("seed").and_then(|v| v.as_u64()).unwrap_or(0),
        min_value: get_f32(&p, "min_value", 0.0),
        max_value: get_f32(&p, "max_value", 1.0),
        mean: get_f32(&p, "mean", 0.0),
        stddev: get_f32(&p, "stddev", 1.0),
        value_a: get_f32(&p, "value_a", 0.0),
        value_b: get_f32(&p, "value_b", 1.0),
        probability: get_f32(&p, "probability", 0.5),
        dimensions: get_u32(&p, "dimensions", 1),
        global_scale: get_f32(&p, "global_scale", 1.0),
    };
    let inner = procgeo_sops::attributes::AttribRandomizeSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[napi]
pub fn attrib_sort(geo: &Geometry, params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let order_str = get_str(&p, "order", "Ascending");
    let order = serde_json::from_str::<procgeo_sops::attributes::AttribSortOrder>(
        &format!("\"{}\"", order_str),
    ).unwrap_or(procgeo_sops::attributes::AttribSortOrder::Ascending);
    let params = procgeo_sops::attributes::AttribSortParams {
        attrib_name: get_str(&p, "attrib_name", "attrib").to_string(),
        class: parse_attrib_class(get_str(&p, "class", "Point")),
        attrib_type: parse_attrib_type(get_str(&p, "attrib_type", "Float")),
        order,
        component: p.get("component").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
    };
    let inner = procgeo_sops::attributes::AttribSortSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[napi]
pub fn attrib_blur(geo: &Geometry, params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let params = procgeo_sops::attributes::AttribBlurParams {
        attrib_name: get_str(&p, "attrib_name", "attrib").to_string(),
        attrib_type: parse_attrib_type(get_str(&p, "attrib_type", "Float")),
        iterations: get_u32(&p, "iterations", 1),
        step_size: get_f32(&p, "step_size", 1.0),
    };
    let inner = procgeo_sops::attributes::AttribBlurSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[napi]
pub fn attrib_fill(geo: &Geometry, params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let params = procgeo_sops::attributes::AttribFillParams {
        attrib_name: get_str(&p, "attrib_name", "attrib").to_string(),
        attrib_type: parse_attrib_type(get_str(&p, "attrib_type", "Float")),
        boundary_group: get_str(&p, "boundary_group", "").to_string(),
        iterations: get_u32(&p, "iterations", 10),
        step_size: get_f32(&p, "step_size", 0.5),
    };
    let inner = procgeo_sops::attributes::AttribFillSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[napi(js_name = "attribNoise")]
pub fn attrib_noise(geo: &Geometry, params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));

    let noise_type = match get_str(&p, "noiseType", "perlin") {
        "simplex" => procgeo_sops::attributes::NoiseType::Simplex,
        "worley" => procgeo_sops::attributes::NoiseType::Worley,
        "worleyF2F1" => procgeo_sops::attributes::NoiseType::WorleyF2F1,
        _ => procgeo_sops::attributes::NoiseType::Perlin,
    };
    let operation = match get_str(&p, "operation", "Set") {
        "Add" => procgeo_sops::attributes::NoiseOperation::Add,
        "Subtract" => procgeo_sops::attributes::NoiseOperation::Subtract,
        "Multiply" => procgeo_sops::attributes::NoiseOperation::Multiply,
        _ => procgeo_sops::attributes::NoiseOperation::Set,
    };
    let range = match get_str(&p, "range", "Positive") {
        "ZeroCentered" => procgeo_sops::attributes::NoiseRange::ZeroCentered,
        "MinMax" => procgeo_sops::attributes::NoiseRange::MinMax,
        _ => procgeo_sops::attributes::NoiseRange::Positive,
    };
    let fractal = match get_str(&p, "fractal", "none") {
        "standard" => procgeo_sops::attributes::FractalType::Standard,
        "terrain" => procgeo_sops::attributes::FractalType::Terrain,
        _ => procgeo_sops::attributes::FractalType::None,
    };
    let offset_val = p.get("offset").and_then(|v| v.as_array()).map(|arr| {
        [
            arr.get(0).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
            arr.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
            arr.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
        ]
    }).unwrap_or([0.0, 0.0, 0.0]);
    let params = procgeo_sops::attributes::AttribNoiseParams {
        attrib_name: get_str(&p, "attribName", "noise").to_string(),
        class: parse_attrib_class(get_str(&p, "class", "Point")),
        dimensions: get_u32(&p, "dimensions", 1),
        noise_type,
        operation,
        element_size: get_f32(&p, "elementSize", 1.0),
        offset: offset_val,
        seed: p.get("seed").and_then(|v| v.as_u64()).unwrap_or(0),
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

#[napi]
pub fn attrib_create(geo: &Geometry, params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let value_vector3 = get_vec3(&p, "value_vector3", [0.0, 0.0, 0.0]);
    let params = procgeo_sops::attributes::AttribCreateParams {
        name: get_str(&p, "name", "attrib1").to_string(),
        class: parse_attrib_class(get_str(&p, "class", "Point")),
        attrib_type: parse_attrib_type(get_str(&p, "attrib_type", "Float")),
        value_int: get_f32(&p, "value_int", 0.0) as i32,
        value_float: get_f32(&p, "value_float", 0.0),
        value_vector3: [value_vector3.x, value_vector3.y, value_vector3.z],
        value_string: get_str(&p, "value_string", "").to_string(),
        qualifier: Default::default(),
    };
    let inner = procgeo_sops::attributes::AttribCreateSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[napi]
pub fn attrib_delete(geo: &Geometry, params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let params = procgeo_sops::attributes::AttribDeleteParams {
        name: get_str(&p, "name", "attrib1").to_string(),
        class: parse_attrib_class(get_str(&p, "class", "Point")),
    };
    let inner = procgeo_sops::attributes::AttribDeleteSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[napi]
pub fn attrib_rename(geo: &Geometry, params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let params = procgeo_sops::attributes::AttribRenameParams {
        from_name: get_str(&p, "from_name", "attrib1").to_string(),
        to_name: get_str(&p, "to_name", "attrib2").to_string(),
        class: parse_attrib_class(get_str(&p, "class", "Point")),
    };
    let inner = procgeo_sops::attributes::AttribRenameSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[napi]
pub fn attrib_promote(geo: &Geometry, params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let method = match get_str(&p, "method", "average") {
        "first" => procgeo_sops::attributes::PromoteMethod::First,
        "last" => procgeo_sops::attributes::PromoteMethod::Last,
        "min" => procgeo_sops::attributes::PromoteMethod::Min,
        "max" => procgeo_sops::attributes::PromoteMethod::Max,
        _ => procgeo_sops::attributes::PromoteMethod::Average,
    };
    let params = procgeo_sops::attributes::AttribPromoteParams {
        name: get_str(&p, "name", "attrib").to_string(),
        from_class: parse_attrib_class(get_str(&p, "from_class", "Point")),
        to_class: parse_attrib_class(get_str(&p, "to_class", "Primitive")),
        method,
        delete_original: get_bool(&p, "delete_original", true),
    };
    let inner = procgeo_sops::attributes::AttribPromoteSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

// ---------------------------------------------------------------------------
// Group SOPs
// ---------------------------------------------------------------------------

#[napi]
pub fn group_create(geo: &Geometry, params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let group_type = match get_str(&p, "group_type", "points") {
        "primitives" | "prims" => procgeo_sops::groups::GroupType::Primitives,
        _ => procgeo_sops::groups::GroupType::Points,
    };
    let mode = match get_str(&p, "mode", "range") {
        "bounding_box" | "bbox" => procgeo_sops::groups::GroupCreateMode::BoundingBox,
        "normal" => procgeo_sops::groups::GroupCreateMode::Normal,
        _ => procgeo_sops::groups::GroupCreateMode::Range,
    };
    let params = procgeo_sops::groups::GroupCreateParams {
        name: get_str(&p, "name", "group1").to_string(),
        group_type,
        mode,
        range_start: get_u32(&p, "range_start", 0) as usize,
        range_end: get_u32(&p, "range_end", u32::MAX) as usize,
        bbox_min: get_vec3(&p, "bbox_min", [f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY]),
        bbox_max: get_vec3(&p, "bbox_max", [f32::INFINITY, f32::INFINITY, f32::INFINITY]),
        normal_direction: get_vec3(&p, "normal_direction", [0.0, 1.0, 0.0]),
        normal_angle: get_f32(&p, "normal_angle", 45.0),
    };
    let inner = procgeo_sops::groups::GroupCreateSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[napi]
pub fn group_combine(geo: &Geometry, params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let operation = match get_str(&p, "operation", "union") {
        "intersect" => procgeo_sops::groups::GroupBooleanOp::Intersect,
        "subtract" => procgeo_sops::groups::GroupBooleanOp::Subtract,
        _ => procgeo_sops::groups::GroupBooleanOp::Union,
    };
    let group_type = match get_str(&p, "group_type", "points") {
        "primitives" | "prims" => procgeo_sops::groups::GroupType::Primitives,
        _ => procgeo_sops::groups::GroupType::Points,
    };
    let params = procgeo_sops::groups::GroupCombineParams {
        name_a: get_str(&p, "name_a", "group_a").to_string(),
        name_b: get_str(&p, "name_b", "group_b").to_string(),
        result: get_str(&p, "result", "group_result").to_string(),
        operation,
        group_type,
    };
    let inner = procgeo_sops::groups::GroupCombineSop
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

// ---------------------------------------------------------------------------
// SOP Registry — generic execute
// ---------------------------------------------------------------------------

static REGISTRY: OnceLock<procgeo_sops::SopRegistry> = OnceLock::new();

fn get_registry() -> &'static procgeo_sops::SopRegistry {
    REGISTRY.get_or_init(procgeo_sops::default_registry)
}

/// Execute any registered SOP by name with JSON params.
/// Uses Rust/snake_case field names for params (matching serde serialization).
#[napi(js_name = "executeSop")]
pub fn execute_sop(
    name: String,
    geo: &Geometry,
    params: Option<serde_json::Value>,
) -> Result<Geometry> {
    let registry = get_registry();
    let params_json = match params {
        Some(v) => serde_json::to_string(&v).unwrap_or_default(),
        None => "{}".to_string(),
    };
    let inner = registry
        .execute(&name, &[&geo.inner], &params_json)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

/// Execute a creation SOP (no input geometry required).
#[napi(js_name = "executeSopCreate")]
pub fn execute_sop_create(
    name: String,
    params: Option<serde_json::Value>,
) -> Result<Geometry> {
    let registry = get_registry();
    let params_json = match params {
        Some(v) => serde_json::to_string(&v).unwrap_or_default(),
        None => "{}".to_string(),
    };
    let inner = registry
        .execute(&name, &[], &params_json)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

#[napi(js_name = "quadWild")]
pub fn quad_wild(geo: &Geometry, params: Option<serde_json::Value>) -> Result<Geometry> {
    let p = params.unwrap_or(serde_json::json!({}));
    let params = procgeo_sops::quadwild::QuadWildParams {
        sharp_angle: get_f32(&p, "sharpAngle", 35.0),
        curvature_weight: get_f32(&p, "curvatureWeight", 0.3),
        smooth_iterations: get_u32(&p, "smoothIterations", 20),
        scale_factor: get_f32(&p, "scaleFactor", 1.0),
        alpha: get_f32(&p, "alpha", 0.02),
        post_smooth_iterations: get_u32(&p, "postSmoothIterations", 30),
    };
    let inner = procgeo_sops::quadwild::QuadWildSop
        .execute(&[&geo.inner], &params)
        .map_err(sop_err)?;
    Ok(Geometry { inner })
}

/// List all registered SOP names.
#[napi(js_name = "listSops")]
pub fn list_sops() -> Vec<String> {
    get_registry()
        .list()
        .into_iter()
        .map(|s| s.to_string())
        .collect()
}

// ---------------------------------------------------------------------------
// COP Registry — GPU image compositing
// ---------------------------------------------------------------------------

use procgeo_cops::registry::CopRegistry;
use procgeo_cops::context::GpuContext as CopGpuContext;

static COP_REGISTRY: OnceLock<CopRegistry> = OnceLock::new();
static GPU_CONTEXT: OnceLock<std::sync::Arc<CopGpuContext>> = OnceLock::new();

fn get_cop_registry() -> &'static CopRegistry {
    COP_REGISTRY.get_or_init(procgeo_cops::registry::default_cop_registry)
}

fn get_gpu_context() -> Result<std::sync::Arc<CopGpuContext>> {
    if let Some(ctx) = GPU_CONTEXT.get() {
        return Ok(std::sync::Arc::clone(ctx));
    }
    let ctx = CopGpuContext::new_blocking()
        .map(std::sync::Arc::new)
        .map_err(|e| napi::Error::from_reason(format!("GPU init failed: {e}")))?;
    let _ = GPU_CONTEXT.set(std::sync::Arc::clone(&ctx));
    Ok(ctx)
}

fn cop_err(e: procgeo_cops::CopError) -> napi::Error {
    napi::Error::from_reason(format!("{e}"))
}

fn get_vec2_node(obj: &serde_json::Value, key: &str, default: [f32; 2]) -> [f32; 2] {
    obj.get(key)
        .and_then(|v| v.as_array())
        .map(|arr| [
            arr.get(0).and_then(|v| v.as_f64()).unwrap_or(default[0] as f64) as f32,
            arr.get(1).and_then(|v| v.as_f64()).unwrap_or(default[1] as f64) as f32,
        ])
        .unwrap_or(default)
}

fn get_vec4_node(obj: &serde_json::Value, key: &str, default: [f32; 4]) -> [f32; 4] {
    obj.get(key)
        .and_then(|v| v.as_array())
        .map(|arr| [
            arr.get(0).and_then(|v| v.as_f64()).unwrap_or(default[0] as f64) as f32,
            arr.get(1).and_then(|v| v.as_f64()).unwrap_or(default[1] as f64) as f32,
            arr.get(2).and_then(|v| v.as_f64()).unwrap_or(default[2] as f64) as f32,
            arr.get(3).and_then(|v| v.as_f64()).unwrap_or(default[3] as f64) as f32,
        ])
        .unwrap_or(default)
}

#[napi]
pub struct CopImage {
    inner: procgeo_cops::image::Image,
}

#[napi]
impl CopImage {
    #[napi(getter)]
    pub fn width(&self) -> u32 { self.inner.width() }

    #[napi(getter)]
    pub fn height(&self) -> u32 { self.inner.height() }

    #[napi(js_name = "toBuffer")]
    pub fn to_buffer(&self) -> Result<Vec<f32>> {
        self.inner.to_cpu().map_err(cop_err)
    }
}

#[napi(js_name = "executeCopCreate")]
pub fn execute_cop_create(name: String, params: Option<serde_json::Value>) -> Result<CopImage> {
    let ctx = get_gpu_context()?;
    let registry = get_cop_registry();
    let params_json = match params {
        Some(v) => serde_json::to_string(&v).unwrap_or_default(),
        None => "{}".to_string(),
    };
    let inner = registry.execute(&name, &ctx, &[], &params_json).map_err(cop_err)?;
    Ok(CopImage { inner })
}

#[napi(js_name = "executeCop")]
pub fn execute_cop(name: String, image: &CopImage, params: Option<serde_json::Value>) -> Result<CopImage> {
    let ctx = get_gpu_context()?;
    let registry = get_cop_registry();
    let params_json = match params {
        Some(v) => serde_json::to_string(&v).unwrap_or_default(),
        None => "{}".to_string(),
    };
    let inner = registry.execute(&name, &ctx, &[&image.inner], &params_json).map_err(cop_err)?;
    Ok(CopImage { inner })
}

#[napi(js_name = "executeCopComposite")]
pub fn execute_cop_composite(name: String, image_a: &CopImage, image_b: &CopImage, params: Option<serde_json::Value>) -> Result<CopImage> {
    let ctx = get_gpu_context()?;
    let registry = get_cop_registry();
    let params_json = match params {
        Some(v) => serde_json::to_string(&v).unwrap_or_default(),
        None => "{}".to_string(),
    };
    let inner = registry.execute(&name, &ctx, &[&image_a.inner, &image_b.inner], &params_json).map_err(cop_err)?;
    Ok(CopImage { inner })
}

#[napi(js_name = "listCops")]
pub fn list_cops() -> Vec<String> {
    get_cop_registry().list().into_iter().map(|s| s.to_string()).collect()
}

#[napi(js_name = "saveCopImage")]
pub fn save_cop_image(image: &CopImage, path: String) -> Result<()> {
    let params = procgeo_cops::io::SaveImageParams {
        path: path.clone(),
        ..Default::default()
    };
    procgeo_cops::io::save_image(&image.inner, &params).map_err(cop_err)
}

// ---------------------------------------------------------------------------
// Dedicated COP functions
// ---------------------------------------------------------------------------

// --- Generators (0 inputs) ---

#[napi(js_name = "copConstant")]
pub fn cop_constant(params: Option<serde_json::Value>) -> Result<CopImage> {
    use procgeo_cops::Cop;
    let p = params.unwrap_or(serde_json::json!({}));
    let cop_params = procgeo_cops::generator::ConstantParams {
        color: get_vec4_node(&p, "color", [0.0, 0.0, 0.0, 1.0]),
        width: get_u32(&p, "width", 256),
        height: get_u32(&p, "height", 256),
    };
    let ctx = get_gpu_context()?;
    let inner = procgeo_cops::generator::ConstantCop
        .execute(&ctx, &[], &cop_params)
        .map_err(cop_err)?;
    Ok(CopImage { inner })
}

#[napi(js_name = "copCheckerboard")]
pub fn cop_checkerboard(params: Option<serde_json::Value>) -> Result<CopImage> {
    use procgeo_cops::Cop;
    let p = params.unwrap_or(serde_json::json!({}));
    let cop_params = procgeo_cops::generator::CheckerboardParams {
        color_a: get_vec4_node(&p, "color_a", [0.0, 0.0, 0.0, 1.0]),
        color_b: get_vec4_node(&p, "color_b", [1.0, 1.0, 1.0, 1.0]),
        frequency: get_vec2_node(&p, "frequency", [8.0, 8.0]),
        width: get_u32(&p, "width", 256),
        height: get_u32(&p, "height", 256),
    };
    let ctx = get_gpu_context()?;
    let inner = procgeo_cops::generator::CheckerboardCop
        .execute(&ctx, &[], &cop_params)
        .map_err(cop_err)?;
    Ok(CopImage { inner })
}

#[napi(js_name = "copNoise")]
pub fn cop_noise(params: Option<serde_json::Value>) -> Result<CopImage> {
    use procgeo_cops::Cop;
    use procgeo_cops::generator::NoiseType;
    let p = params.unwrap_or(serde_json::json!({}));
    let noise_type = match get_str(&p, "noise_type", "perlin") {
        "simplex" => NoiseType::Simplex,
        "worley" => NoiseType::Worley,
        _ => NoiseType::Perlin,
    };
    let cop_params = procgeo_cops::generator::NoiseParams {
        noise_type,
        frequency: get_f32(&p, "frequency", 4.0),
        octaves: get_u32(&p, "octaves", 4),
        lacunarity: get_f32(&p, "lacunarity", 2.0),
        gain: get_f32(&p, "gain", 0.5),
        amplitude: get_f32(&p, "amplitude", 1.0),
        offset: get_vec2_node(&p, "offset", [0.0, 0.0]),
        seed: get_u32(&p, "seed", 0),
        width: get_u32(&p, "width", 256),
        height: get_u32(&p, "height", 256),
    };
    let ctx = get_gpu_context()?;
    let inner = procgeo_cops::generator::NoiseCop
        .execute(&ctx, &[], &cop_params)
        .map_err(cop_err)?;
    Ok(CopImage { inner })
}

#[napi(js_name = "copRamp")]
pub fn cop_ramp(params: Option<serde_json::Value>) -> Result<CopImage> {
    use procgeo_cops::Cop;
    use procgeo_cops::generator::{RampType, RampStop};
    let p = params.unwrap_or(serde_json::json!({}));
    let ramp_type = match get_str(&p, "ramp_type", "linear") {
        "radial" => RampType::Radial,
        "box" => RampType::Box,
        "diagonal" => RampType::Diagonal,
        _ => RampType::Linear,
    };
    let stops: Vec<RampStop> = p
        .get("stops")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|stop_val| {
                    let position = stop_val
                        .get("position")
                        .and_then(|v| v.as_f64())
                        .map(|v| v as f32)?;
                    let color = stop_val
                        .get("color")
                        .and_then(|v| v.as_array())
                        .map(|c| [
                            c.get(0).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                            c.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                            c.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                            c.get(3).and_then(|v| v.as_f64()).unwrap_or(1.0) as f32,
                        ])?;
                    Some((position, color))
                })
                .collect()
        })
        .unwrap_or_else(|| vec![
            (0.0, [0.0, 0.0, 0.0, 1.0]),
            (1.0, [1.0, 1.0, 1.0, 1.0]),
        ]);
    let cop_params = procgeo_cops::generator::RampParams {
        ramp_type,
        stops,
        width: get_u32(&p, "width", 256),
        height: get_u32(&p, "height", 256),
    };
    let ctx = get_gpu_context()?;
    let inner = procgeo_cops::generator::RampCop
        .execute(&ctx, &[], &cop_params)
        .map_err(cop_err)?;
    Ok(CopImage { inner })
}

#[napi(js_name = "copLoadImage")]
pub fn cop_load_image(params: Option<serde_json::Value>) -> Result<CopImage> {
    use procgeo_cops::Cop;
    let p = params.unwrap_or(serde_json::json!({}));
    let cop_params = procgeo_cops::generator::LoadImageParams {
        path: get_str(&p, "path", "").to_string(),
    };
    let ctx = get_gpu_context()?;
    let inner = procgeo_cops::generator::LoadImageCop
        .execute(&ctx, &[], &cop_params)
        .map_err(cop_err)?;
    Ok(CopImage { inner })
}

// --- Filters (1 input) ---

#[napi(js_name = "copBlur")]
pub fn cop_blur(image: &CopImage, params: Option<serde_json::Value>) -> Result<CopImage> {
    use procgeo_cops::Cop;
    use procgeo_cops::filter::blur::BlurType;
    let p = params.unwrap_or(serde_json::json!({}));
    let blur_type = match get_str(&p, "blur_type", "gaussian") {
        "box" => BlurType::Box,
        _ => BlurType::Gaussian,
    };
    let cop_params = procgeo_cops::filter::blur::BlurParams {
        blur_type,
        radius_x: get_f32(&p, "radius_x", 4.0),
        radius_y: get_f32(&p, "radius_y", 4.0),
    };
    let inner = procgeo_cops::filter::blur::BlurCop
        .execute(image.inner.ctx(), &[&image.inner], &cop_params)
        .map_err(cop_err)?;
    Ok(CopImage { inner })
}

#[napi(js_name = "copFlip")]
pub fn cop_flip(image: &CopImage, params: Option<serde_json::Value>) -> Result<CopImage> {
    use procgeo_cops::Cop;
    let p = params.unwrap_or(serde_json::json!({}));
    let cop_params = procgeo_cops::filter::flip::FlipParams {
        horizontal: get_bool(&p, "horizontal", false),
        vertical: get_bool(&p, "vertical", true),
    };
    let inner = procgeo_cops::filter::flip::FlipCop
        .execute(image.inner.ctx(), &[&image.inner], &cop_params)
        .map_err(cop_err)?;
    Ok(CopImage { inner })
}

#[napi(js_name = "copMirror")]
pub fn cop_mirror(image: &CopImage, params: Option<serde_json::Value>) -> Result<CopImage> {
    use procgeo_cops::Cop;
    use procgeo_cops::filter::mirror::MirrorAxis;
    let p = params.unwrap_or(serde_json::json!({}));
    let axis = match get_str(&p, "axis", "x") {
        "y" | "Y" => MirrorAxis::Y,
        _ => MirrorAxis::X,
    };
    let cop_params = procgeo_cops::filter::mirror::MirrorParams {
        axis,
        offset: get_f32(&p, "offset", 0.5),
    };
    let inner = procgeo_cops::filter::mirror::MirrorCop
        .execute(image.inner.ctx(), &[&image.inner], &cop_params)
        .map_err(cop_err)?;
    Ok(CopImage { inner })
}

#[napi(js_name = "copChannelSwap")]
pub fn cop_channel_swap(image: &CopImage, params: Option<serde_json::Value>) -> Result<CopImage> {
    use procgeo_cops::Cop;
    use procgeo_cops::filter::channel_swap::Channel;
    let p = params.unwrap_or(serde_json::json!({}));
    let parse_channel = |key: &str, default: Channel| -> Channel {
        match get_str(&p, key, "") {
            "r" | "R" => Channel::R,
            "g" | "G" => Channel::G,
            "b" | "B" => Channel::B,
            "a" | "A" => Channel::A,
            "one" | "One" | "1" => Channel::One,
            "zero" | "Zero" | "0" => Channel::Zero,
            _ => default,
        }
    };
    let cop_params = procgeo_cops::filter::channel_swap::ChannelSwapParams {
        r: parse_channel("r", Channel::R),
        g: parse_channel("g", Channel::G),
        b: parse_channel("b", Channel::B),
        a: parse_channel("a", Channel::A),
    };
    let inner = procgeo_cops::filter::channel_swap::ChannelSwapCop
        .execute(image.inner.ctx(), &[&image.inner], &cop_params)
        .map_err(cop_err)?;
    Ok(CopImage { inner })
}

#[napi(js_name = "copResize")]
pub fn cop_resize(image: &CopImage, params: Option<serde_json::Value>) -> Result<CopImage> {
    use procgeo_cops::Cop;
    let p = params.unwrap_or(serde_json::json!({}));
    let filter = match get_str(&p, "filter", "nearest") {
        "bilinear" | "Bilinear" => procgeo_cops::FilterMode::Bilinear,
        _ => procgeo_cops::FilterMode::Nearest,
    };
    let cop_params = procgeo_cops::filter::resize::ResizeParams {
        width: get_u32(&p, "width", 256),
        height: get_u32(&p, "height", 256),
        filter,
    };
    let inner = procgeo_cops::filter::resize::ResizeCop
        .execute(image.inner.ctx(), &[&image.inner], &cop_params)
        .map_err(cop_err)?;
    Ok(CopImage { inner })
}

#[napi(js_name = "copRotate")]
pub fn cop_rotate(image: &CopImage, params: Option<serde_json::Value>) -> Result<CopImage> {
    use procgeo_cops::Cop;
    let p = params.unwrap_or(serde_json::json!({}));
    let filter = match get_str(&p, "filter", "nearest") {
        "bilinear" | "Bilinear" => procgeo_cops::FilterMode::Bilinear,
        _ => procgeo_cops::FilterMode::Nearest,
    };
    let cop_params = procgeo_cops::filter::rotate::RotateParams {
        angle: get_f32(&p, "angle", 0.0),
        center: get_vec2_node(&p, "center", [0.5, 0.5]),
        filter,
    };
    let inner = procgeo_cops::filter::rotate::RotateCop
        .execute(image.inner.ctx(), &[&image.inner], &cop_params)
        .map_err(cop_err)?;
    Ok(CopImage { inner })
}

#[napi(js_name = "copSwirl")]
pub fn cop_swirl(image: &CopImage, params: Option<serde_json::Value>) -> Result<CopImage> {
    use procgeo_cops::Cop;
    let p = params.unwrap_or(serde_json::json!({}));
    let cop_params = procgeo_cops::filter::swirl::SwirlParams {
        center: get_vec2_node(&p, "center", [0.5, 0.5]),
        angle: get_f32(&p, "angle", 90.0),
        radius: get_f32(&p, "radius", 0.5),
    };
    let inner = procgeo_cops::filter::swirl::SwirlCop
        .execute(image.inner.ctx(), &[&image.inner], &cop_params)
        .map_err(cop_err)?;
    Ok(CopImage { inner })
}

// --- Composite (2 inputs) ---

#[napi(js_name = "copComposite")]
pub fn cop_composite(a: &CopImage, b: &CopImage, params: Option<serde_json::Value>) -> Result<CopImage> {
    use procgeo_cops::Cop;
    use procgeo_cops::composite::CompOp;
    let p = params.unwrap_or(serde_json::json!({}));
    let operation = match get_str(&p, "operation", "over") {
        "add" => CompOp::Add,
        "multiply" => CompOp::Multiply,
        "screen" => CompOp::Screen,
        "subtract" => CompOp::Subtract,
        "difference" => CompOp::Difference,
        "min" => CompOp::Min,
        "max" => CompOp::Max,
        _ => CompOp::Over,
    };
    let cop_params = procgeo_cops::composite::CompositeParams {
        operation,
        mix: get_f32(&p, "mix", 1.0),
    };
    let inner = procgeo_cops::composite::CompositeCop
        .execute(a.inner.ctx(), &[&a.inner, &b.inner], &cop_params)
        .map_err(cop_err)?;
    Ok(CopImage { inner })
}

// --- Custom ---

#[napi(js_name = "copCustomShader")]
pub fn cop_custom_shader(
    input_a: Option<&CopImage>,
    input_b: Option<&CopImage>,
    params: Option<serde_json::Value>,
) -> Result<CopImage> {
    use procgeo_cops::Cop;
    use procgeo_cops::custom::ShaderLang;
    let p = params.unwrap_or(serde_json::json!({}));
    let language = match get_str(&p, "language", "wgsl") {
        "glsl" | "Glsl" | "GLSL" => ShaderLang::Glsl,
        _ => ShaderLang::Wgsl,
    };
    let cop_params = procgeo_cops::custom::CustomShaderParams {
        source: get_str(&p, "source", "").to_string(),
        language,
        uniforms: std::collections::HashMap::new(),
        width: get_u32(&p, "width", 256),
        height: get_u32(&p, "height", 256),
    };
    let ctx = get_gpu_context()?;
    let mut inputs: Vec<&procgeo_cops::image::Image> = Vec::new();
    if let Some(a) = input_a {
        inputs.push(&a.inner);
    }
    if let Some(b) = input_b {
        inputs.push(&b.inner);
    }
    let inner = procgeo_cops::custom::CustomShaderCop
        .execute(&ctx, &inputs, &cop_params)
        .map_err(cop_err)?;
    Ok(CopImage { inner })
}
