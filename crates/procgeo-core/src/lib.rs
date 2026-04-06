pub mod error;
pub mod handle;
pub mod point;
pub mod primitive;
pub mod vertex;

pub use handle::{PrimHandle, PointHandle, VertexHandle};
pub use point::PointStorage;
pub use primitive::{PolyType, PolygonPrim, PrimStorage, Primitive};
pub use vertex::VertexStorage;
