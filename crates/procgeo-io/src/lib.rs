use procgeo_core::Geometry;
use std::io::{Read, Write};
use std::path::Path;
use thiserror::Error;

#[cfg(feature = "obj")]
pub mod obj;

#[cfg(feature = "gltf")]
pub mod gltf;

#[derive(Debug, Error)]
pub enum IoError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),
}

pub trait GeometryWriter {
    fn write(&self, geo: &Geometry, writer: &mut dyn Write) -> Result<(), IoError>;
    fn extensions(&self) -> &[&str];
}

pub trait GeometryReader {
    fn read(&self, reader: &mut dyn Read) -> Result<Geometry, IoError>;
    fn extensions(&self) -> &[&str];
}

pub fn write_file(geo: &Geometry, path: &Path) -> Result<(), IoError> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| IoError::UnsupportedFormat("no extension".to_string()))?;
    match ext {
        #[cfg(feature = "obj")]
        "obj" => {
            let mut f = std::fs::File::create(path)?;
            obj::ObjWriter.write(geo, &mut f)
        }
        #[cfg(feature = "gltf")]
        "glb" => {
            let mut f = std::fs::File::create(path)?;
            gltf::GlbWriter.write(geo, &mut f)
        }
        _ => Err(IoError::UnsupportedFormat(ext.to_string())),
    }
}

pub fn read_file(path: &Path) -> Result<Geometry, IoError> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| IoError::UnsupportedFormat("no extension".to_string()))?;
    match ext {
        #[cfg(feature = "obj")]
        "obj" => {
            let mut f = std::fs::File::open(path)?;
            obj::ObjReader.read(&mut f)
        }
        _ => Err(IoError::UnsupportedFormat(ext.to_string())),
    }
}
