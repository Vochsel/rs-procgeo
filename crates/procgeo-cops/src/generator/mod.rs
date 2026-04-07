// Generator COPs — create images from scratch (Constant, Checkerboard, Noise, Ramp, LoadImage)

#[cfg(feature = "gpu")]
mod constant;
#[cfg(feature = "gpu")]
pub use constant::{ConstantCop, ConstantParams};

#[cfg(feature = "gpu")]
mod checkerboard;
#[cfg(feature = "gpu")]
pub use checkerboard::{CheckerboardCop, CheckerboardParams};

#[cfg(feature = "gpu")]
mod noise;
#[cfg(feature = "gpu")]
pub use noise::{NoiseCop, NoiseParams, NoiseType};

#[cfg(feature = "gpu")]
mod ramp;
#[cfg(feature = "gpu")]
pub use ramp::{RampCop, RampParams, RampStop, RampType};

#[cfg(feature = "gpu")]
mod load_image;
#[cfg(feature = "gpu")]
pub use load_image::{LoadImageCop, LoadImageParams};
