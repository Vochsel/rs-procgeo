// procgeo-sops: Surface Operations (SOPs) for procedural geometry

use procgeo_core::Geometry;
use thiserror::Error;

#[cfg(feature = "creation")]
pub mod creation;
#[cfg(feature = "transform")]
pub mod transform;
#[cfg(feature = "normals")]
pub mod normals;
#[cfg(feature = "merge")]
pub mod merge;
#[cfg(feature = "attributes")]
pub mod attributes;
#[cfg(feature = "groups_sops")]
pub mod groups;
#[cfg(feature = "delete")]
pub mod delete;
#[cfg(feature = "copy")]
pub mod copy;
#[cfg(feature = "reshape")]
pub mod reshape;
#[cfg(feature = "scatter")]
pub mod scatter;
#[cfg(feature = "topology")]
pub mod topology;
#[cfg(feature = "measure_sops")]
pub mod measure;
#[cfg(feature = "utility_sops")]
pub mod utility;

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
