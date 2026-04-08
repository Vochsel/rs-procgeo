use std::io::{BufRead, BufReader, Read, Write};

use glam::Vec3;
use procgeo_core::{AttribClass, Geometry, PolyType, PrimHandle, Primitive};

use crate::{GeometryReader, GeometryWriter, IoError};

// ---------------------------------------------------------------------------
// ObjWriter
// ---------------------------------------------------------------------------

pub struct ObjWriter;

impl GeometryWriter for ObjWriter {
    fn extensions(&self) -> &[&str] {
        &["obj"]
    }

    fn write(&self, geo: &Geometry, writer: &mut dyn Write) -> Result<(), IoError> {
        // Header
        writeln!(writer, "# ProcGeo OBJ export")?;
        writeln!(
            writer,
            "# points: {}  prims: {}",
            geo.num_points(),
            geo.num_prims()
        )?;
        writeln!(writer)?;

        // Detect whether the "N" point attribute exists
        let has_normals = geo.find_attrib::<[f32; 3]>(AttribClass::Point, "N").is_ok();

        // Write vertex positions
        for pos in geo.points() {
            writeln!(writer, "v {} {} {}", pos.x, pos.y, pos.z)?;
        }

        // Write vertex normals if the attribute exists
        if has_normals {
            let n_handle = geo
                .find_attrib::<[f32; 3]>(AttribClass::Point, "N")
                .expect("just verified N exists");
            for i in 0..geo.num_points() {
                let n = geo
                    .get_attrib(&n_handle, i)
                    .map_err(|e| IoError::Parse(e.to_string()))?;
                writeln!(writer, "vn {} {} {}", n[0], n[1], n[2])?;
            }
            writeln!(writer)?;
        } else {
            writeln!(writer)?;
        }

        // Write faces / polylines
        for prim_idx in 0..geo.num_prims() {
            let prim_handle = PrimHandle::from_index(prim_idx);
            let pt_handles = geo.prim_points(prim_handle);
            let prim = geo.prim(prim_handle);

            match prim {
                Primitive::Polygon(poly) => {
                    match poly.poly_type {
                        PolyType::Closed => {
                            // Face — use 1-based indices
                            write!(writer, "f")?;
                            for ph in &pt_handles {
                                let idx = ph.index() + 1; // 1-based
                                if has_normals {
                                    write!(writer, " {}//{}", idx, idx)?;
                                } else {
                                    write!(writer, " {}", idx)?;
                                }
                            }
                            writeln!(writer)?;
                        }
                        PolyType::Open => {
                            // Polyline — "l" line
                            write!(writer, "l")?;
                            for ph in &pt_handles {
                                write!(writer, " {}", ph.index() + 1)?;
                            }
                            writeln!(writer)?;
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// ObjReader
// ---------------------------------------------------------------------------

pub struct ObjReader;

impl GeometryReader for ObjReader {
    fn extensions(&self) -> &[&str] {
        &["obj"]
    }

    fn read(&self, reader: &mut dyn Read) -> Result<Geometry, IoError> {
        let buf_reader = BufReader::new(reader);
        let mut geo = Geometry::new();

        for line_result in buf_reader.lines() {
            let line = line_result?;
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let mut tokens = line.splitn(2, char::is_whitespace);
            let keyword = match tokens.next() {
                Some(k) => k,
                None => continue,
            };
            let rest = tokens.next().unwrap_or("").trim();

            match keyword {
                "v" => {
                    // Parse "v x y z" (optional w ignored)
                    let coords: Vec<f32> = rest
                        .split_whitespace()
                        .take(3)
                        .map(|s| s.parse::<f32>().map_err(|e| IoError::Parse(e.to_string())))
                        .collect::<Result<_, _>>()?;
                    if coords.len() < 3 {
                        return Err(IoError::Parse(format!("invalid vertex: {}", line)));
                    }
                    geo.add_point(Vec3::new(coords[0], coords[1], coords[2]));
                }
                "f" => {
                    // Parse face — each token can be "1", "1/2", "1/2/3", "1//3"
                    // Extract the first number (vertex position index, 1-based)
                    let indices: Vec<usize> = rest
                        .split_whitespace()
                        .map(|tok| {
                            let first = tok.split('/').next().unwrap_or(tok);
                            first
                                .parse::<usize>()
                                .map_err(|e| IoError::Parse(e.to_string()))
                                .map(|n| n - 1) // convert to 0-based
                        })
                        .collect::<Result<_, _>>()?;

                    if indices.len() < 2 {
                        return Err(IoError::Parse(format!(
                            "face needs at least 2 indices: {}",
                            line
                        )));
                    }

                    use procgeo_core::PointHandle;
                    let handles: Vec<_> = indices
                        .iter()
                        .map(|&i| PointHandle::from_index(i))
                        .collect();
                    geo.add_face(&handles);
                }
                "l" => {
                    // Parse polyline
                    let indices: Vec<usize> = rest
                        .split_whitespace()
                        .map(|tok| {
                            let first = tok.split('/').next().unwrap_or(tok);
                            first
                                .parse::<usize>()
                                .map_err(|e| IoError::Parse(e.to_string()))
                                .map(|n| n - 1) // convert to 0-based
                        })
                        .collect::<Result<_, _>>()?;

                    if indices.len() < 2 {
                        return Err(IoError::Parse(format!(
                            "polyline needs at least 2 indices: {}",
                            line
                        )));
                    }

                    use procgeo_core::PointHandle;
                    let handles: Vec<_> = indices
                        .iter()
                        .map(|&i| PointHandle::from_index(i))
                        .collect();
                    geo.add_polyline(&handles);
                }
                // Skip "vn", "vt", "vp", "mtllib", "usemtl", "o", "g", "s", etc.
                _ => {}
            }
        }

        Ok(geo)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GeometryReader, GeometryWriter};
    use approx::assert_relative_eq;
    use glam::Vec3;

    fn make_triangle() -> Geometry {
        let mut geo = Geometry::new();
        let p0 = geo.add_point(Vec3::new(0.0, 0.0, 0.0));
        let p1 = geo.add_point(Vec3::new(1.0, 0.0, 0.0));
        let p2 = geo.add_point(Vec3::new(0.0, 1.0, 0.0));
        geo.add_face(&[p0, p1, p2]);
        geo
    }

    #[test]
    fn test_obj_write() {
        let geo = make_triangle();
        let mut buf: Vec<u8> = Vec::new();
        ObjWriter.write(&geo, &mut buf).unwrap();

        let output = String::from_utf8(buf).unwrap();
        assert!(
            output.contains("v 0 0 0"),
            "missing v 0 0 0\n---\n{}",
            output
        );
        assert!(
            output.contains("v 1 0 0"),
            "missing v 1 0 0\n---\n{}",
            output
        );
        assert!(
            output.contains("v 0 1 0"),
            "missing v 0 1 0\n---\n{}",
            output
        );
        assert!(
            output.contains("f 1 2 3"),
            "missing f 1 2 3\n---\n{}",
            output
        );
    }

    #[test]
    fn test_obj_roundtrip() {
        let geo = make_triangle();

        // Write to buffer
        let mut buf: Vec<u8> = Vec::new();
        ObjWriter.write(&geo, &mut buf).unwrap();

        // Read back
        let geo2 = ObjReader.read(&mut buf.as_slice()).unwrap();

        assert_eq!(geo2.num_points(), 3);
        assert_eq!(geo2.num_prims(), 1);

        // Check positions
        let positions: Vec<Vec3> = geo2.points().collect();
        assert_relative_eq!(positions[0].x, 0.0, epsilon = 1e-5);
        assert_relative_eq!(positions[0].y, 0.0, epsilon = 1e-5);
        assert_relative_eq!(positions[0].z, 0.0, epsilon = 1e-5);

        assert_relative_eq!(positions[1].x, 1.0, epsilon = 1e-5);
        assert_relative_eq!(positions[1].y, 0.0, epsilon = 1e-5);
        assert_relative_eq!(positions[1].z, 0.0, epsilon = 1e-5);

        assert_relative_eq!(positions[2].x, 0.0, epsilon = 1e-5);
        assert_relative_eq!(positions[2].y, 1.0, epsilon = 1e-5);
        assert_relative_eq!(positions[2].z, 0.0, epsilon = 1e-5);
    }

    #[test]
    fn test_obj_roundtrip_box() {
        use procgeo_sops::creation::{BoxParams, BoxSop};
        use procgeo_sops::generate;

        let box_geo = generate(&BoxSop, &BoxParams::default()).unwrap();
        assert_eq!(box_geo.num_points(), 8);
        assert_eq!(box_geo.num_prims(), 6);

        // Write to buffer
        let mut buf: Vec<u8> = Vec::new();
        ObjWriter.write(&box_geo, &mut buf).unwrap();

        // Read back
        let geo2 = ObjReader.read(&mut buf.as_slice()).unwrap();

        assert_eq!(geo2.num_points(), 8, "box roundtrip should have 8 points");
        assert_eq!(geo2.num_prims(), 6, "box roundtrip should have 6 prims");
    }
}
