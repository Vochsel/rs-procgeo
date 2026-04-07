// Boolean SOP module — CSG boolean operations on polygon meshes.

pub mod boolean_sop;
pub mod bvh;
pub mod classification;
pub mod detriangulate;
pub mod intersection;
pub mod splitting;

pub use boolean_sop::{
    BooleanOp, BooleanParams, BooleanSop, BooleanTreatAs, CustomMatch, Detriangulate,
};
