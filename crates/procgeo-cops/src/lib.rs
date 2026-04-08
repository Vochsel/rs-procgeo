// procgeo-cops: Compositing Operations (COPs) with GPU compute shaders

use thiserror::Error;

#[cfg(feature = "gpu")]
pub mod context;
#[cfg(feature = "gpu")]
pub mod image;

pub mod io;
pub mod registry;

#[cfg(feature = "composite")]
pub mod composite;
#[cfg(feature = "custom")]
pub mod custom;
#[cfg(feature = "filter")]
pub mod filter;
#[cfg(feature = "generator")]
pub mod generator;

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum CopError {
    #[error("wrong number of inputs: expected {expected_min}-{expected_max}, got {got}")]
    WrongInputCount {
        expected_min: usize,
        expected_max: usize,
        got: usize,
    },
    #[error("GPU error: {0}")]
    Gpu(String),
    #[error("shader compilation error: {0}")]
    ShaderCompilation(String),
    #[error("image load error: {0}")]
    ImageLoad(String),
    #[error("invalid parameter: {0}")]
    InvalidParam(String),
    #[error("{0}")]
    Other(String),
}

// ---------------------------------------------------------------------------
// FilterMode — shared by multiple filter COPs
// ---------------------------------------------------------------------------

/// Sampling mode for image filtering operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum FilterMode {
    #[default]
    Nearest,
    Bilinear,
}

// ---------------------------------------------------------------------------
// Cop trait
// ---------------------------------------------------------------------------

/// The core compositing operator trait, mirroring the SOP `Sop` trait.
#[cfg(feature = "gpu")]
pub trait Cop {
    type Params: Default;

    /// Execute this COP with the given GPU context, input images, and parameters.
    fn execute(
        &self,
        ctx: &std::sync::Arc<context::GpuContext>,
        inputs: &[&image::Image],
        params: &Self::Params,
    ) -> Result<image::Image, CopError>;

    /// The (min, max) number of input images this COP accepts.
    fn input_count(&self) -> (usize, usize);

    /// The name of this COP (e.g. "constant", "blur").
    fn name(&self) -> &'static str;

    /// Validate that the number of inputs is within the expected range.
    fn validate_inputs(&self, inputs: &[&image::Image]) -> Result<(), CopError> {
        let (min, max) = self.input_count();
        if inputs.len() < min || inputs.len() > max {
            return Err(CopError::WrongInputCount {
                expected_min: min,
                expected_max: max,
                got: inputs.len(),
            });
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// ImageExt — chaining API
// ---------------------------------------------------------------------------

/// Extension trait for chaining COP operations on images.
#[cfg(feature = "gpu")]
pub trait ImageExt {
    fn apply<C: Cop>(self, cop: &C, params: &C::Params) -> Result<image::Image, CopError>;
}

#[cfg(feature = "gpu")]
impl ImageExt for image::Image {
    fn apply<C: Cop>(self, cop: &C, params: &C::Params) -> Result<image::Image, CopError> {
        cop.execute(self.ctx(), &[&self], params)
    }
}

// ---------------------------------------------------------------------------
// Utility functions
// ---------------------------------------------------------------------------

/// Generate an image from a COP that takes no inputs (generators).
#[cfg(feature = "gpu")]
pub fn generate_cop<C: Cop>(
    ctx: &std::sync::Arc<context::GpuContext>,
    cop: &C,
    params: &C::Params,
) -> Result<image::Image, CopError> {
    cop.execute(ctx, &[], params)
}

/// Compute a stable hash for a COP name, used as pipeline cache key.
pub fn hash_name(name: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    name.hash(&mut hasher);
    hasher.finish()
}

/// Dispatch a 2D compute shader with 16x16 workgroups sized to cover `width x height`.
#[cfg(feature = "gpu")]
pub fn dispatch_2d(encoder: &mut wgpu::ComputePass<'_>, width: u32, height: u32) {
    let wg_x = (width + 15) / 16;
    let wg_y = (height + 15) / 16;
    encoder.dispatch_workgroups(wg_x, wg_y, 1);
}

// ---------------------------------------------------------------------------
// Prelude
// ---------------------------------------------------------------------------

pub mod prelude {
    pub use crate::CopError;
    pub use crate::FilterMode;

    #[cfg(feature = "gpu")]
    pub use crate::context::GpuContext;
    #[cfg(feature = "gpu")]
    pub use crate::image::Image;
    #[cfg(feature = "gpu")]
    pub use crate::{Cop, ImageExt, dispatch_2d, generate_cop};

    pub use crate::hash_name;

    pub use crate::io::*;
    pub use crate::registry::*;
}
