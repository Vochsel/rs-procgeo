// Composite COPs — combine multiple input images (Over, Add, Multiply, etc.)

#[cfg(feature = "gpu")]
mod composite;
#[cfg(feature = "gpu")]
pub use composite::{CompOp, CompositeCop, CompositeParams};
