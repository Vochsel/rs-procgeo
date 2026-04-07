pub use procgeo_core as core;
pub use procgeo_sops as sops;
pub use procgeo_io as io;
#[cfg(feature = "cops")]
pub use procgeo_cops as cops;

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
    #[cfg(feature = "attributes")]
    pub use procgeo_sops::attributes::*;
    #[cfg(feature = "groups_sops")]
    pub use procgeo_sops::groups::*;
    #[cfg(feature = "delete")]
    pub use procgeo_sops::delete::*;
    #[cfg(feature = "copy")]
    pub use procgeo_sops::copy::*;
    #[cfg(feature = "reshape")]
    pub use procgeo_sops::reshape::*;
    #[cfg(feature = "scatter")]
    pub use procgeo_sops::scatter::*;
    #[cfg(feature = "topology")]
    pub use procgeo_sops::topology::*;
    #[cfg(feature = "measure_sops")]
    pub use procgeo_sops::measure::*;
    #[cfg(feature = "utility_sops")]
    pub use procgeo_sops::utility::*;
    #[cfg(feature = "color")]
    pub use procgeo_sops::color::*;
    #[cfg(feature = "cops")]
    pub use procgeo_cops::prelude::*;
    #[cfg(feature = "deform")]
    pub use procgeo_sops::deform::*;
    #[cfg(feature = "boolean")]
    pub use procgeo_sops::boolean::*;
}
