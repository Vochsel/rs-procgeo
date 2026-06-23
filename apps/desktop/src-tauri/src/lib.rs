//! ProcGeo desktop backend.
//!
//! Unlike the web playground (which calls procgeo compiled to WASM), this app
//! links procgeo as a **native Rust dependency**. SOP graphs sent from the
//! webview are cooked on the native side and only render-ready buffers cross
//! the IPC boundary.

use std::collections::HashMap;
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

/// A node in a SOP DAG: a registered SOP name, JSON params, and the ids of the
/// nodes whose output feeds this node's inputs (in order).
#[derive(Deserialize)]
struct Node {
    id: String,
    #[serde(rename = "type")]
    sop: String,
    #[serde(default)]
    params: serde_json::Value,
    #[serde(default)]
    inputs: Vec<String>,
}

impl Node {
    fn params_json(&self) -> String {
        if self.params.is_null() {
            "{}".to_string()
        } else {
            self.params.to_string()
        }
    }
}

/// A SOP DAG: a set of nodes plus the id of the node to render. If `output` is
/// omitted, the last node in the list is rendered.
#[derive(Deserialize)]
struct Dag {
    nodes: Vec<Node>,
    #[serde(default)]
    output: Option<String>,
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

/// Cook a SOP DAG natively and return render-ready buffers for the output node.
#[tauri::command]
fn cook_graph(graph: Dag) -> Result<GeoBuffers, String> {
    if graph.nodes.is_empty() {
        return Err("graph has no nodes".into());
    }
    let reg = registry();

    let by_id: HashMap<&str, &Node> = graph.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let order = topo_order(&graph.nodes, &by_id)?;

    // Cooked geometry per node id. Inputs are looked up here by reference.
    let mut cache: HashMap<&str, Geometry> = HashMap::new();
    for id in &order {
        let node = by_id[id];
        let inputs: Vec<&Geometry> = node
            .inputs
            .iter()
            .map(|iid| {
                cache
                    .get(iid.as_str())
                    .ok_or_else(|| format!("node '{}' input '{}' not found", node.id, iid))
            })
            .collect::<Result<_, _>>()?;
        let geo = reg
            .execute(&node.sop, &inputs, &node.params_json())
            .map_err(|e| format!("node '{}' ({}): {}", node.id, node.sop, e))?;
        drop(inputs); // release the immutable borrows of `cache` before inserting
        cache.insert(node.id.as_str(), geo);
    }

    let output = graph
        .output
        .as_deref()
        .unwrap_or_else(|| graph.nodes.last().unwrap().id.as_str());
    let geo = cache
        .get(output)
        .ok_or_else(|| format!("output node '{output}' not found"))?;
    Ok(GeoBuffers::from_geometry(geo))
}

/// Depth-first topological sort over node inputs, with cycle detection.
fn topo_order<'a>(
    nodes: &'a [Node],
    by_id: &HashMap<&'a str, &'a Node>,
) -> Result<Vec<&'a str>, String> {
    #[derive(Clone, Copy, PartialEq)]
    enum Mark {
        Temp,
        Done,
    }
    let mut marks: HashMap<&str, Mark> = HashMap::new();
    let mut order: Vec<&str> = Vec::with_capacity(nodes.len());

    fn visit<'a>(
        id: &'a str,
        by_id: &HashMap<&'a str, &'a Node>,
        marks: &mut HashMap<&'a str, Mark>,
        order: &mut Vec<&'a str>,
    ) -> Result<(), String> {
        match marks.get(id) {
            Some(Mark::Done) => return Ok(()),
            Some(Mark::Temp) => return Err(format!("graph has a cycle through node '{id}'")),
            None => {}
        }
        let node = by_id
            .get(id)
            .ok_or_else(|| format!("unknown node id '{id}'"))?;
        marks.insert(id, Mark::Temp);
        for input in &node.inputs {
            visit(input.as_str(), by_id, marks, order)?;
        }
        marks.insert(id, Mark::Done);
        order.push(id);
        Ok(())
    }

    for node in nodes {
        visit(node.id.as_str(), by_id, &mut marks, &mut order)?;
    }
    Ok(order)
}

/// List every registered SOP name.
#[tauri::command]
fn list_sops() -> Vec<String> {
    registry().list().into_iter().map(String::from).collect()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![cook_graph, list_sops])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
