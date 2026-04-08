use std::io::Write;

use procgeo_core::{AttribClass, Geometry, PolyType, PrimHandle, Primitive};
use serde_json::json;

use crate::{GeometryWriter, IoError};

// ---------------------------------------------------------------------------
// GlbWriter
// ---------------------------------------------------------------------------

pub struct GlbWriter;

impl GeometryWriter for GlbWriter {
    fn extensions(&self) -> &[&str] {
        &["glb"]
    }

    fn write(&self, geo: &Geometry, writer: &mut dyn Write) -> Result<(), IoError> {
        let glb = build_glb(geo)?;
        writer.write_all(&glb)?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// GLB binary construction
// ---------------------------------------------------------------------------

/// Pad a byte slice to 4-byte alignment using the given fill byte.
fn pad_to_4(data: &mut Vec<u8>, fill: u8) {
    let rem = data.len() % 4;
    if rem != 0 {
        let pad = 4 - rem;
        data.extend(std::iter::repeat(fill).take(pad));
    }
}

/// Write a u32 in little-endian to a Vec<u8>.
fn push_u32_le(buf: &mut Vec<u8>, v: u32) {
    buf.extend_from_slice(&v.to_le_bytes());
}

/// Write an f32 in little-endian to a Vec<u8>.
fn push_f32_le(buf: &mut Vec<u8>, v: f32) {
    buf.extend_from_slice(&v.to_le_bytes());
}

/// Fan-triangulate a polygon vertex list (0-based local indices into the face).
/// Input: slice of point indices for the face.
/// Output: triangle indices appended to `out`.
fn fan_triangulate(face_pts: &[usize], out: &mut Vec<u32>) {
    if face_pts.len() < 3 {
        return;
    }
    let v0 = face_pts[0] as u32;
    for i in 1..(face_pts.len() - 1) {
        out.push(v0);
        out.push(face_pts[i] as u32);
        out.push(face_pts[i + 1] as u32);
    }
}

/// Build a complete GLB byte blob from a Geometry.
pub fn build_glb(geo: &Geometry) -> Result<Vec<u8>, IoError> {
    // ------------------------------------------------------------------
    // 1. Detect optional attributes
    // ------------------------------------------------------------------
    let n_handle = geo.find_attrib::<[f32; 3]>(AttribClass::Point, "N").ok();
    let cd_handle = geo.find_attrib::<[f32; 3]>(AttribClass::Point, "Cd").ok();

    let has_normals = n_handle.is_some();
    let has_colors = cd_handle.is_some();

    // ------------------------------------------------------------------
    // 2. Collect unique vertices and build index buffer via fan triangulation
    // ------------------------------------------------------------------
    // We use a flat vertex list (one entry per point) and index into it.
    // Positions, normals, and colors are stored per-point.

    let num_pts = geo.num_points();

    // Collect positions
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(num_pts);
    for pos in geo.points() {
        positions.push([pos.x, pos.y, pos.z]);
    }

    // Collect normals if present
    let mut normals: Vec<[f32; 3]> = Vec::new();
    if let Some(ref nh) = n_handle {
        normals.reserve(num_pts);
        for i in 0..num_pts {
            let n = geo
                .get_attrib(nh, i)
                .map_err(|e| IoError::Parse(e.to_string()))?;
            normals.push(n);
        }
    }

    // Collect colors if present
    let mut colors: Vec<[f32; 3]> = Vec::new();
    if let Some(ref cdh) = cd_handle {
        colors.reserve(num_pts);
        for i in 0..num_pts {
            let cd = geo
                .get_attrib(cdh, i)
                .map_err(|e| IoError::Parse(e.to_string()))?;
            colors.push(cd);
        }
    }

    // Build index buffer from closed polygons only
    let mut indices: Vec<u32> = Vec::new();
    for prim_idx in 0..geo.num_prims() {
        let prim_handle = PrimHandle::from_index(prim_idx);
        let pt_handles = geo.prim_points(prim_handle);
        let prim = geo.prim(prim_handle);
        match prim {
            Primitive::Polygon(poly) if poly.poly_type == PolyType::Closed => {
                let face_pts: Vec<usize> = pt_handles.iter().map(|ph| ph.index()).collect();
                fan_triangulate(&face_pts, &mut indices);
            }
            _ => {}
        }
    }

    // ------------------------------------------------------------------
    // 3. Compute position min/max for accessor
    // ------------------------------------------------------------------
    let (pos_min, pos_max) = if positions.is_empty() {
        ([0.0f32, 0.0, 0.0], [0.0f32, 0.0, 0.0])
    } else {
        let mut mn = positions[0];
        let mut mx = positions[0];
        for p in &positions[1..] {
            for i in 0..3 {
                mn[i] = mn[i].min(p[i]);
                mx[i] = mx[i].max(p[i]);
            }
        }
        (mn, mx)
    };

    // ------------------------------------------------------------------
    // 4. Build binary buffer
    // ------------------------------------------------------------------
    // Layout: [positions] [normals?] [colors?] [indices]
    let mut bin: Vec<u8> = Vec::new();

    // Positions
    let pos_byte_offset = 0usize;
    let pos_byte_length = positions.len() * 3 * 4;
    for p in &positions {
        push_f32_le(&mut bin, p[0]);
        push_f32_le(&mut bin, p[1]);
        push_f32_le(&mut bin, p[2]);
    }

    // Normals
    let norm_byte_offset = bin.len();
    let norm_byte_length = normals.len() * 3 * 4;
    for n in &normals {
        push_f32_le(&mut bin, n[0]);
        push_f32_le(&mut bin, n[1]);
        push_f32_le(&mut bin, n[2]);
    }

    // Colors
    let color_byte_offset = bin.len();
    let color_byte_length = colors.len() * 3 * 4;
    for c in &colors {
        push_f32_le(&mut bin, c[0]);
        push_f32_le(&mut bin, c[1]);
        push_f32_le(&mut bin, c[2]);
    }

    // Pad to 4-byte alignment before indices (u32 alignment)
    pad_to_4(&mut bin, 0x00);

    // Indices
    let idx_byte_offset = bin.len();
    let idx_byte_length = indices.len() * 4;
    for idx in &indices {
        push_u32_le(&mut bin, *idx);
    }

    // Final padding of BIN chunk to 4-byte alignment
    pad_to_4(&mut bin, 0x00);
    let bin_chunk_data_len = bin.len() as u32;

    // ------------------------------------------------------------------
    // 5. Build glTF JSON
    // ------------------------------------------------------------------
    // Accessor/BufferView indices:
    //   0: positions BufferView
    //   1: normals BufferView (if has_normals)
    //   2: colors BufferView (if has_colors)
    //   N: indices BufferView (last)
    //
    //   Accessor 0: positions
    //   Accessor 1: normals (if has_normals)
    //   Accessor 2: colors (if has_colors)
    //   Accessor N: indices

    let mut buffer_views = Vec::new();
    let mut accessors = Vec::new();
    let mut accessor_idx: usize = 0;

    // Positions buffer view + accessor
    buffer_views.push(json!({
        "buffer": 0,
        "byteOffset": pos_byte_offset,
        "byteLength": pos_byte_length,
        "target": 34962  // ARRAY_BUFFER
    }));
    let pos_accessor_idx = accessor_idx;
    accessors.push(json!({
        "bufferView": pos_accessor_idx,
        "byteOffset": 0,
        "componentType": 5126,  // FLOAT
        "count": num_pts,
        "type": "VEC3",
        "min": [pos_min[0], pos_min[1], pos_min[2]],
        "max": [pos_max[0], pos_max[1], pos_max[2]]
    }));
    accessor_idx += 1;

    // Normals buffer view + accessor
    let norm_accessor_idx = if has_normals {
        buffer_views.push(json!({
            "buffer": 0,
            "byteOffset": norm_byte_offset,
            "byteLength": norm_byte_length,
            "target": 34962
        }));
        let idx = accessor_idx;
        accessors.push(json!({
            "bufferView": idx,
            "byteOffset": 0,
            "componentType": 5126,  // FLOAT
            "count": num_pts,
            "type": "VEC3"
        }));
        accessor_idx += 1;
        Some(idx)
    } else {
        None
    };

    // Colors buffer view + accessor
    let color_accessor_idx = if has_colors {
        buffer_views.push(json!({
            "buffer": 0,
            "byteOffset": color_byte_offset,
            "byteLength": color_byte_length,
            "target": 34962
        }));
        let idx = accessor_idx;
        accessors.push(json!({
            "bufferView": idx,
            "byteOffset": 0,
            "componentType": 5126,  // FLOAT
            "count": num_pts,
            "type": "VEC3"
        }));
        accessor_idx += 1;
        Some(idx)
    } else {
        None
    };

    // Indices buffer view + accessor
    let idx_accessor_idx = accessor_idx;
    buffer_views.push(json!({
        "buffer": 0,
        "byteOffset": idx_byte_offset,
        "byteLength": idx_byte_length,
        "target": 34963  // ELEMENT_ARRAY_BUFFER
    }));
    accessors.push(json!({
        "bufferView": idx_accessor_idx,
        "byteOffset": 0,
        "componentType": 5125,  // UNSIGNED_INT
        "count": indices.len(),
        "type": "SCALAR"
    }));

    // Build mesh primitive attributes object
    let mut attributes = serde_json::Map::new();
    attributes.insert("POSITION".to_string(), json!(pos_accessor_idx));
    if let Some(ni) = norm_accessor_idx {
        attributes.insert("NORMAL".to_string(), json!(ni));
    }
    if let Some(ci) = color_accessor_idx {
        attributes.insert("COLOR_0".to_string(), json!(ci));
    }

    let gltf_json = json!({
        "asset": {
            "version": "2.0",
            "generator": "procgeo"
        },
        "scene": 0,
        "scenes": [{ "nodes": [0] }],
        "nodes": [{ "mesh": 0 }],
        "meshes": [{
            "primitives": [{
                "attributes": attributes,
                "indices": idx_accessor_idx
            }]
        }],
        "accessors": accessors,
        "bufferViews": buffer_views,
        "buffers": [{
            "byteLength": bin_chunk_data_len
        }]
    });

    // ------------------------------------------------------------------
    // 6. Build JSON chunk bytes (padded to 4-byte alignment with spaces)
    // ------------------------------------------------------------------
    let mut json_bytes =
        serde_json::to_vec(&gltf_json).map_err(|e| IoError::Parse(e.to_string()))?;
    pad_to_4(&mut json_bytes, 0x20); // spaces for JSON padding

    // ------------------------------------------------------------------
    // 7. Assemble GLB
    // ------------------------------------------------------------------
    // GLB Header: magic(4) + version(4) + total_length(4) = 12 bytes
    // JSON chunk: chunk_length(4) + chunk_type(4) + data
    // BIN chunk:  chunk_length(4) + chunk_type(4) + data

    let json_chunk_header_size = 8u32;
    let bin_chunk_header_size = 8u32;
    let header_size = 12u32;

    let json_data_len = json_bytes.len() as u32;
    let total_length = header_size
        + json_chunk_header_size
        + json_data_len
        + bin_chunk_header_size
        + bin_chunk_data_len;

    let mut glb: Vec<u8> = Vec::with_capacity(total_length as usize);

    // GLB header
    push_u32_le(&mut glb, 0x46546C67); // magic "glTF"
    push_u32_le(&mut glb, 2); // version
    push_u32_le(&mut glb, total_length);

    // JSON chunk
    push_u32_le(&mut glb, json_data_len);
    push_u32_le(&mut glb, 0x4E4F534A); // "JSON"
    glb.extend_from_slice(&json_bytes);

    // BIN chunk
    push_u32_le(&mut glb, bin_chunk_data_len);
    push_u32_le(&mut glb, 0x004E4942); // "BIN\0"
    glb.extend_from_slice(&bin);

    Ok(glb)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GeometryWriter;
    use glam::Vec3;

    fn make_triangle() -> Geometry {
        let mut geo = Geometry::new();
        let p0 = geo.add_point(Vec3::new(0.0, 0.0, 0.0));
        let p1 = geo.add_point(Vec3::new(1.0, 0.0, 0.0));
        let p2 = geo.add_point(Vec3::new(0.0, 1.0, 0.0));
        geo.add_face(&[p0, p1, p2]);
        geo
    }

    /// GLB magic bytes: "glTF" = 0x67, 0x6C, 0x54, 0x46 (little-endian u32 0x46546C67)
    fn assert_glb_magic(buf: &[u8]) {
        assert!(buf.len() >= 4, "GLB too short");
        assert_eq!(
            &buf[0..4],
            &[0x67, 0x6C, 0x54, 0x46],
            "GLB magic mismatch — expected 'glTF'"
        );
    }

    #[test]
    fn gltf_write_triangle() {
        let geo = make_triangle();
        let mut buf: Vec<u8> = Vec::new();
        GlbWriter.write(&geo, &mut buf).unwrap();

        assert!(!buf.is_empty(), "GLB output must not be empty");
        assert_glb_magic(&buf);

        // Check GLB version == 2
        let version = u32::from_le_bytes(buf[4..8].try_into().unwrap());
        assert_eq!(version, 2, "GLB version must be 2");
    }

    #[test]
    fn gltf_write_box() {
        use procgeo_sops::creation::{BoxParams, BoxSop};
        use procgeo_sops::generate;

        let box_geo = generate(&BoxSop, &BoxParams::default()).unwrap();
        let mut buf: Vec<u8> = Vec::new();
        GlbWriter.write(&box_geo, &mut buf).unwrap();

        assert!(!buf.is_empty(), "GLB output must not be empty for box");
        assert_glb_magic(&buf);
    }

    #[test]
    fn gltf_write_with_normals() {
        use procgeo_sops::creation::{BoxParams, BoxSop};
        use procgeo_sops::normals::{NormalParams, NormalSop};
        use procgeo_sops::{GeometryExt, generate};

        let box_geo = generate(&BoxSop, &BoxParams::default()).unwrap();
        let geo_with_normals = box_geo.apply(&NormalSop, &NormalParams).unwrap();

        let mut buf: Vec<u8> = Vec::new();
        GlbWriter.write(&geo_with_normals, &mut buf).unwrap();

        assert!(!buf.is_empty(), "GLB with normals must not be empty");
        assert_glb_magic(&buf);

        // Verify the JSON chunk contains "NORMAL" accessor
        // JSON chunk starts at byte 12, chunk data at byte 20
        let json_data_len = u32::from_le_bytes(buf[12..16].try_into().unwrap()) as usize;
        let json_bytes = &buf[20..20 + json_data_len];
        let json_str = std::str::from_utf8(json_bytes)
            .unwrap()
            .trim_end_matches(char::is_whitespace);
        let json: serde_json::Value = serde_json::from_str(json_str).unwrap();

        // Check NORMAL key exists in the mesh primitive attributes
        let attributes = &json["meshes"][0]["primitives"][0]["attributes"];
        assert!(
            attributes.get("NORMAL").is_some(),
            "NORMAL accessor should be present in glTF JSON when geometry has 'N' attribute"
        );
    }
}
