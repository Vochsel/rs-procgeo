// Generator COPs — create images from scratch (Constant, Checkerboard, Noise, Ramp, etc.)

#[cfg(feature = "gpu")]
mod constant;
#[cfg(feature = "gpu")]
pub use constant::{ConstantCop, ConstantParams};
