pub mod attribute;
pub mod error;
pub mod geometry;
pub mod group;
pub mod handle;
pub mod math;
pub mod point;
pub mod primitive;
pub mod vertex;

pub use attribute::{
    AttribClass, AttribDefault, AttribHandle, AttribStorage, AttribType, AttribValue, Attribute,
    AttributeMap, TypeQualifier,
};
pub use error::CoreError;
pub use geometry::Geometry;
pub use group::{EdgeGroup, ElementGroup, GroupMap};
pub use handle::{PrimHandle, PointHandle, VertexHandle};
pub use math::{BBox, fit, efit, smooth};
pub use point::PointStorage;
pub use primitive::{PolyType, PolygonPrim, PrimStorage, Primitive};
pub use vertex::VertexStorage;
