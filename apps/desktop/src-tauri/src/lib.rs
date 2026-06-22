//! ProcGeo desktop backend.
//!
//! Unlike the web playground (which calls procgeo compiled to WASM), this app
//! links procgeo as a **native Rust dependency**. SOP graphs sent from the
//! webview are cooked on the native side and only render-ready buffers cross
//! the IPC boundary.

use std::sync::OnceLock;

use procgeo_core::{AttribClass, Geometry, PolyType, PrimHandle, Primitive};
use procgeo_sops::SopRegistry;
use serde::{Deserialize, Serialize};

/// The SOP registry is immutable and stateless, so build it once.
static REGISTRY: OnceLock<SopRegistry> = OnceLock::new();

fn registry() -> &'static SopRegistry {
    REGISTRY.get_or_init(procgeo_sops::default_registry)
}

// ---------------------------------------------------------------------------
// Wire types
// ---------------------------------------------------------------------------

/// A single SOP invocation: a registered name plus a JSON params object.
#[derive(Deserialize)]
struct SopCall {
    name: String,
    #[serde(default)]
    params: serde_json::Value,
}

impl SopCall {
    fn params_json(&self) -> String {
        if self.params.is_null() {
            "{}".to_string()
        } else {
            self.params.to_string()
        }
    }
}

/// A linear SOP graph: one creation node followed by chained modifiers.
#[derive(Deserialize)]
struct Graph {
    create: SopCall,
    #[serde(default)]
    modifiers: Vec<SopCall>,
}

/// Render-ready buffers, mirroring the WASM binding's getters so the frontend
/// bridge is identical across web and desktop.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GeoBuffers {
    positions: Vec<f32>,
    indices: Vec<u32>,
    normals: Option<Vec<f32>>,
    colors: Option<Vec<f32>>,
    num_points: usize,
    num_prims: usize,
}

impl GeoBuffers {
    fn from_geometry(geo: &Geometry) -> Self {
        GeoBuffers {
            positions: positions(geo),
            indices: triangle_indices(geo),
            normals: point_vec3(geo, "N"),
            colors: point_vec3(geo, "Cd"),
            num_points: geo.num_points(),
            num_prims: geo.num_prims(),
        }
    }
}

// ---------------------------------------------------------------------------
// Buffer extraction (mirrors bindings/procgeo-wasm getPositions/etc.)
// ---------------------------------------------------------------------------

fn positions(geo: &Geometry) -> Vec<f32> {
    let mut buf = Vec::with_capacity(geo.num_points() * 3);
    for p in geo.points() {
        buf.push(p.x);
        buf.push(p.y);
        buf.push(p.z);
    }
    buf
}

fn triangle_indices(geo: &Geometry) -> Vec<u32> {
    let mut indices = Vec::new();
    for (i, _prim) in geo.prims().enumerate() {
        let ph = PrimHandle::from_index(i);
        let Primitive::Polygon(poly) = geo.prim(ph);
        if poly.poly_type != PolyType::Closed {
            continue;
        }
        let pts = geo.prim_points(ph);
        if pts.len() >= 3 {
            // Fan-triangulate from the first vertex.
            for j in 1..pts.len() - 1 {
                indices.push(pts[0].index() as u32);
                indices.push(pts[j].index() as u32);
                indices.push(pts[j + 1].index() as u32);
            }
        }
    }
    indices
}

/// Read a per-point `[f32; 3]` attribute as a flat buffer, if it exists.
fn point_vec3(geo: &Geometry, name: &str) -> Option<Vec<f32>> {
    let handle = geo.find_attrib::<[f32; 3]>(AttribClass::Point, name).ok()?;
    let n = geo.num_points();
    let mut buf = Vec::with_capacity(n * 3);
    for i in 0..n {
        let v = geo.get_attrib(&handle, i).ok()?;
        buf.extend_from_slice(&v);
    }
    Some(buf)
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

/// Cook a SOP graph natively and return render-ready buffers.
#[tauri::command]
fn cook(graph: Graph) -> Result<GeoBuffers, String> {
    let reg = registry();
    let mut geo = reg
        .execute(&graph.create.name, &[], &graph.create.params_json())
        .map_err(|e| e.to_string())?;
    for m in &graph.modifiers {
        geo = reg
            .execute(&m.name, &[&geo], &m.params_json())
            .map_err(|e| e.to_string())?;
    }
    Ok(GeoBuffers::from_geometry(&geo))
}

/// List every registered SOP name.
#[tauri::command]
fn list_sops() -> Vec<String> {
    registry().list().into_iter().map(String::from).collect()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![cook, list_sops])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
