// Custom COPs — user-defined compute shaders via WGSL or GLSL

#[cfg(feature = "gpu")]
mod custom_shader;
#[cfg(feature = "gpu")]
pub use custom_shader::{CustomShaderCop, CustomShaderParams, ShaderLang, UniformValue};
