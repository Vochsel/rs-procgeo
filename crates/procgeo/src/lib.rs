pub use procgeo_core as core;
pub use procgeo_sops as sops;
pub use procgeo_io as io;

pub mod prelude {
    pub use procgeo_core::{
        AttribClass, AttribDefault, AttribHandle, AttribValue, Geometry,
        PointHandle, PrimHandle, VertexHandle, PolyType, TypeQualifier,
    };
    pub use procgeo_core::math::BBox;
    pub use procgeo_sops::{Sop, SopError, GeometryExt, generate};

    #[cfg(feature = "creation")]
    pub use procgeo_sops::creation::*;
    #[cfg(feature = "transform")]
    pub use procgeo_sops::transform::*;
    #[cfg(feature = "normals")]
    pub use procgeo_sops::normals::*;
    #[cfg(feature = "merge")]
    pub use procgeo_sops::merge::*;
}
