// procgeo-sops: Surface Operations (SOPs) for procedural geometry

use procgeo_core::Geometry;
use thiserror::Error;

pub mod registry;
pub use registry::{DynSop, SopRegistry, default_registry};

#[cfg(feature = "attributes")]
pub mod attributes;
#[cfg(feature = "boolean")]
pub mod boolean;
#[cfg(feature = "color")]
pub mod color;
#[cfg(feature = "copy")]
pub mod copy;
#[cfg(feature = "creation")]
pub mod creation;
#[cfg(feature = "deform")]
pub mod deform;
#[cfg(feature = "delete")]
pub mod delete;
#[cfg(feature = "groups_sops")]
pub mod groups;
#[cfg(feature = "measure_sops")]
pub mod measure;
#[cfg(feature = "merge")]
pub mod merge;
#[cfg(feature = "normals")]
pub mod normals;
#[cfg(feature = "quadwild")]
pub mod quadwild;
#[cfg(feature = "reshape")]
pub mod reshape;
#[cfg(feature = "scatter")]
pub mod scatter;
#[cfg(feature = "topology")]
pub mod topology;
#[cfg(feature = "transform")]
pub mod transform;
#[cfg(feature = "utility_sops")]
pub mod utility;
#[cfg(feature = "voronoi")]
pub mod voronoi;

#[derive(Debug, Error)]
pub enum SopError {
    #[error("wrong number of inputs: expected {expected_min}-{expected_max}, got {got}")]
    WrongInputCount {
        expected_min: usize,
        expected_max: usize,
        got: usize,
    },
    #[error("invalid parameter: {0}")]
    InvalidParam(String),
    #[error("core error: {0}")]
    Core(#[from] procgeo_core::error::CoreError),
    #[error("{0}")]
    Other(String),
}

pub trait Sop {
    type Params: Default;
    fn execute(&self, inputs: &[&Geometry], params: &Self::Params) -> Result<Geometry, SopError>;
    fn input_count(&self) -> (usize, usize);
    fn name(&self) -> &'static str;
    fn validate_inputs(&self, inputs: &[&Geometry]) -> Result<(), SopError> {
        let (min, max) = self.input_count();
        if inputs.len() < min || inputs.len() > max {
            return Err(SopError::WrongInputCount {
                expected_min: min,
                expected_max: max,
                got: inputs.len(),
            });
        }
        Ok(())
    }
}

pub trait GeometryExt {
    fn apply<S: Sop>(self, sop: &S, params: &S::Params) -> Result<Geometry, SopError>;
}

impl GeometryExt for Geometry {
    fn apply<S: Sop>(self, sop: &S, params: &S::Params) -> Result<Geometry, SopError> {
        sop.execute(&[&self], params)
    }
}

pub fn generate<S: Sop>(sop: &S, params: &S::Params) -> Result<Geometry, SopError> {
    sop.execute(&[], params)
}
