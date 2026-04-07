// CustomShaderCop — run user-provided WGSL or GLSL compute shaders

use std::collections::HashMap;
use std::sync::Arc;

use crate::context::GpuContext;
use crate::image::Image;
use crate::{Cop, CopError, dispatch_2d};

/// Shader source language.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ShaderLang {
    /// WebGPU Shading Language (used directly).
    #[default]
    Wgsl,
    /// OpenGL Shading Language (transpiled to WGSL via naga).
    Glsl,
}

/// A typed uniform value that can be packed into a byte buffer.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum UniformValue {
    Float(f32),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
    Int(i32),
}

/// Parameters for the Custom Shader COP.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CustomShaderParams {
    /// Shader source code.
    pub source: String,
    /// Language of the source code.
    pub language: ShaderLang,
    /// Named uniform values (packed in declaration order).
    pub uniforms: HashMap<String, UniformValue>,
    /// Output image width.
    pub width: u32,
    /// Output image height.
    pub height: u32,
}

impl Default for CustomShaderParams {
    fn default() -> Self {
        Self {
            source: String::new(),
            language: ShaderLang::Wgsl,
            uniforms: HashMap::new(),
            width: 256,
            height: 256,
        }
    }
}

/// Custom shader COP — executes user-provided WGSL (or transpiled GLSL) shaders.
///
/// For WGSL: the shader must declare binding 0 as a `texture_storage_2d<rgba32float, write>`
/// (and optionally input textures at bindings 1+).
///
/// For GLSL: the user writes only the body code. It is wrapped in a full compute
/// shader that provides `cop_uv`, `cop_resolution`, and `cop_output` variables, and
/// the result is transpiled to WGSL via naga.
pub struct CustomShaderCop;

impl Cop for CustomShaderCop {
    type Params = CustomShaderParams;

    fn name(&self) -> &'static str {
        "custom_shader"
    }

    fn input_count(&self) -> (usize, usize) {
        (0, 8)
    }

    fn execute(&self, ctx: &Arc<GpuContext>, inputs: &[&Image], params: &CustomShaderParams) -> Result<Image, CopError> {
        self.validate_inputs(inputs)?;

        if params.source.trim().is_empty() {
            return Err(CopError::InvalidParam(
                "custom_shader: source must not be empty".into(),
            ));
        }

        let output = Image::create_storage(Arc::clone(ctx), params.width, params.height);

        // Obtain WGSL source — transpile from GLSL if needed.
        let wgsl_source: String = match params.language {
            ShaderLang::Wgsl => params.source.clone(),
            ShaderLang::Glsl => transpile_glsl(&params.source)?,
        };

        // Use a hash of the source as the pipeline cache key.
        let source_hash = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            wgsl_source.hash(&mut h);
            h.finish()
        };

        let pipeline = ctx.get_or_create_pipeline(source_hash, &wgsl_source, "main")?;

        let output_view = output
            .texture()
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Build bind group entries.
        // Binding 0 is always the output storage texture.
        // Bindings 1..N are the input textures.
        let mut entries: Vec<wgpu::BindGroupEntry> = Vec::new();

        entries.push(wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(&output_view),
        });

        let input_views: Vec<wgpu::TextureView> = inputs
            .iter()
            .map(|img| img.texture().create_view(&wgpu::TextureViewDescriptor::default()))
            .collect();

        for (i, view) in input_views.iter().enumerate() {
            entries.push(wgpu::BindGroupEntry {
                binding: (i + 1) as u32,
                resource: wgpu::BindingResource::TextureView(view),
            });
        }

        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let bind_group = ctx.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("custom_shader_bind_group"),
            layout: &bind_group_layout,
            entries: &entries,
        });

        let mut encoder =
            ctx.device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("custom_shader_dispatch"),
                });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("custom_shader_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            dispatch_2d(&mut pass, params.width, params.height);
        }

        ctx.queue().submit(std::iter::once(encoder.finish()));

        Ok(output)
    }
}

/// Transpile a GLSL compute shader body (user code only) to WGSL via naga.
///
/// The user writes only the body; we wrap it in a full compute shader that provides:
/// - `cop_uv` — normalized UV coordinates (0..1)
/// - `cop_resolution` — image dimensions as vec2
/// - `cop_output` — the vec4 to write (user must set this)
#[cfg(feature = "custom")]
fn transpile_glsl(source: &str) -> Result<String, CopError> {
    let full_glsl = format!(
        r#"#version 450
layout(local_size_x = 16, local_size_y = 16) in;
layout(rgba32f, set = 0, binding = 0) uniform writeonly image2D cop_output_tex;
void main() {{
    ivec2 dims = imageSize(cop_output_tex);
    ivec2 gid = ivec2(gl_GlobalInvocationID.xy);
    if (gid.x >= dims.x || gid.y >= dims.y) return;
    vec2 cop_uv = (vec2(gid) + 0.5) / vec2(dims);
    vec2 cop_resolution = vec2(dims);
    vec4 cop_output;
    {source}
    imageStore(cop_output_tex, gid, cop_output);
}}"#
    );

    // Parse GLSL → naga IR
    let mut glsl_frontend = naga::front::glsl::Frontend::default();
    let options = naga::front::glsl::Options {
        stage: naga::ShaderStage::Compute,
        defines: Default::default(),
    };
    let module = glsl_frontend
        .parse(&options, &full_glsl)
        .map_err(|e| CopError::ShaderCompilation(format!("GLSL parse error: {e:?}")))?;

    // Validate
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );
    let module_info = validator
        .validate(&module)
        .map_err(|e| CopError::ShaderCompilation(format!("GLSL validation error: {e:?}")))?;

    // Write WGSL
    let wgsl = naga::back::wgsl::write_string(
        &module,
        &module_info,
        naga::back::wgsl::WriterFlags::empty(),
    )
    .map_err(|e| CopError::ShaderCompilation(format!("WGSL write error: {e:?}")))?;

    Ok(wgsl)
}

#[cfg(not(feature = "custom"))]
fn transpile_glsl(_source: &str) -> Result<String, CopError> {
    Err(CopError::Other(
        "GLSL transpilation requires the 'custom' feature".into(),
    ))
}

// ---------------------------------------------------------------------------
// Convenient WGSL template for green-fill test
// ---------------------------------------------------------------------------

/// A minimal WGSL shader that fills with solid green (for testing).
#[allow(dead_code)]
pub const GREEN_FILL_WGSL: &str = r#"
@group(0) @binding(0) var output: texture_storage_2d<rgba32float, write>;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3u) {
    let dims = textureDimensions(output);
    if gid.x >= dims.x || gid.y >= dims.y { return; }
    textureStore(output, vec2i(gid.xy), vec4f(0.0, 1.0, 0.0, 1.0));
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate_cop;

    fn make_ctx() -> Option<Arc<GpuContext>> {
        match GpuContext::new_blocking() {
            Ok(c) => Some(Arc::new(c)),
            Err(e) => {
                eprintln!("Skipping test (no GPU): {e}");
                None
            }
        }
    }

    #[test]
    fn custom_wgsl_green_fill() {
        let ctx = match make_ctx() { Some(c) => c, None => return };

        let params = CustomShaderParams {
            source: GREEN_FILL_WGSL.to_string(),
            language: ShaderLang::Wgsl,
            width: 4,
            height: 4,
            uniforms: HashMap::new(),
        };

        let img = generate_cop(&ctx, &CustomShaderCop, &params)
            .expect("execute failed");
        let pixels = img.to_cpu().expect("readback failed");

        for (i, chunk) in pixels.chunks_exact(4).enumerate() {
            assert!(
                chunk[0].abs() < 1e-5 && (chunk[1] - 1.0).abs() < 1e-5,
                "pixel {i} should be green, got {:?}",
                chunk
            );
        }
    }

    #[test]
    fn empty_source_errors() {
        let ctx = match make_ctx() { Some(c) => c, None => return };

        let params = CustomShaderParams {
            source: "".to_string(),
            language: ShaderLang::Wgsl,
            width: 4,
            height: 4,
            uniforms: HashMap::new(),
        };

        let result = generate_cop(&ctx, &CustomShaderCop, &params);
        assert!(result.is_err(), "expected error for empty source");
    }
}
