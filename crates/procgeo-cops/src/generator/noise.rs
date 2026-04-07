// NoiseCop — generates procedural noise textures via GPU compute shader

use std::sync::Arc;

use bytemuck::{Pod, Zeroable};

use crate::context::GpuContext;
use crate::image::Image;
use crate::{Cop, CopError, dispatch_2d, hash_name};

/// Noise algorithm to use.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum NoiseType {
    /// Value noise with smoothstep interpolation (Perlin-style).
    #[default]
    Perlin,
    /// Simplex-like noise.
    Simplex,
    /// Worley/cellular noise.
    Worley,
}

impl NoiseType {
    fn as_u32(&self) -> u32 {
        match self {
            Self::Perlin => 0,
            Self::Simplex => 1,
            Self::Worley => 2,
        }
    }
}

/// Parameters for the Noise COP.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct NoiseParams {
    /// Noise algorithm.
    pub noise_type: NoiseType,
    /// Base frequency (scale of noise features).
    pub frequency: f32,
    /// Number of fBm octaves.
    pub octaves: u32,
    /// Frequency multiplier per octave.
    pub lacunarity: f32,
    /// Amplitude multiplier per octave.
    pub gain: f32,
    /// Overall amplitude scaling.
    pub amplitude: f32,
    /// XY offset into noise space.
    pub offset: [f32; 2],
    /// Seed value (shifts noise origin).
    pub seed: u32,
    /// Output width in pixels.
    pub width: u32,
    /// Output height in pixels.
    pub height: u32,
}

impl Default for NoiseParams {
    fn default() -> Self {
        Self {
            noise_type: NoiseType::default(),
            frequency: 4.0,
            octaves: 4,
            lacunarity: 2.0,
            gain: 0.5,
            amplitude: 1.0,
            offset: [0.0, 0.0],
            seed: 0,
            width: 256,
            height: 256,
        }
    }
}

/// GPU-side uniform layout (must match noise.wgsl `Params` struct).
///
/// WGSL layout (with vec3u align(16)):
///   offset  0: frequency(f32), amplitude(f32), lacunarity(f32), gain(f32) → 16 bytes
///   offset 16: offset(vec2f) → 8 bytes
///   offset 24: seed(u32), octaves(u32), noise_type(u32) → 12 bytes
///   offset 36: 12 bytes implicit padding (vec3u is align 16, next boundary = 48)
///   offset 48: _pad(vec3u) → 12 bytes
///   offset 60: 4 bytes tail padding → struct size = 64 bytes
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct NoiseUniform {
    frequency: f32,        // offset 0
    amplitude: f32,        // offset 4
    lacunarity: f32,       // offset 8
    gain: f32,             // offset 12
    offset: [f32; 2],     // offset 16
    seed: u32,             // offset 24
    octaves: u32,          // offset 28
    noise_type: u32,       // offset 32
    _pad0: [u32; 3],      // offset 36 — padding to reach 48
    _pad1: [u32; 4],      // offset 48 — _pad vec3u slot (12 bytes) + 4 tail = 16 bytes
}

/// Generator COP that produces procedural noise images (Perlin, Simplex, Worley + fBm).
pub struct NoiseCop;

const SHADER_SOURCE: &str = include_str!("../../shaders/noise.wgsl");

impl Cop for NoiseCop {
    type Params = NoiseParams;

    fn name(&self) -> &'static str {
        "noise"
    }

    fn input_count(&self) -> (usize, usize) {
        (0, 0)
    }

    fn execute(
        &self,
        ctx: &Arc<GpuContext>,
        inputs: &[&Image],
        params: &NoiseParams,
    ) -> Result<Image, CopError> {
        self.validate_inputs(inputs)?;

        let output = Image::create_storage(Arc::clone(ctx), params.width, params.height);

        let uniform_data = NoiseUniform {
            frequency: params.frequency,
            amplitude: params.amplitude,
            lacunarity: params.lacunarity,
            gain: params.gain,
            offset: params.offset,
            seed: params.seed,
            octaves: params.octaves,
            noise_type: params.noise_type.as_u32(),
            _pad0: [0; 3],
            _pad1: [0; 4],
        };

        let uniform_buffer = ctx.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("noise_params"),
            size: std::mem::size_of::<NoiseUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        ctx.queue()
            .write_buffer(&uniform_buffer, 0, bytemuck::bytes_of(&uniform_data));

        let output_view = output
            .texture()
            .create_view(&wgpu::TextureViewDescriptor::default());

        let pipeline_key = hash_name("noise");
        let pipeline = ctx.get_or_create_pipeline(pipeline_key, SHADER_SOURCE, "main")?;

        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let bind_group = ctx.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("noise_bind_group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&output_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });

        let mut encoder =
            ctx.device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("noise_dispatch"),
                });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("noise_pass"),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate_cop;

    fn try_ctx() -> Option<Arc<GpuContext>> {
        match GpuContext::new_blocking() {
            Ok(c) => Some(Arc::new(c)),
            Err(e) => {
                eprintln!("Skipping noise test (no GPU): {e}");
                None
            }
        }
    }

    fn is_non_flat(pixels: &[f32]) -> bool {
        let first = pixels[0];
        pixels.iter().any(|&v| (v - first).abs() > 1e-4)
    }

    #[test]
    fn perlin_non_flat() {
        let Some(ctx) = try_ctx() else { return; };

        let params = NoiseParams {
            noise_type: NoiseType::Perlin,
            width: 32,
            height: 32,
            ..Default::default()
        };

        let img = generate_cop(&ctx, &NoiseCop, &params).expect("execute failed");
        let pixels = img.to_cpu().expect("readback failed");
        assert!(is_non_flat(&pixels), "Perlin noise produced a flat image");
    }

    #[test]
    fn simplex_non_flat() {
        let Some(ctx) = try_ctx() else { return; };

        let params = NoiseParams {
            noise_type: NoiseType::Simplex,
            width: 32,
            height: 32,
            ..Default::default()
        };

        let img = generate_cop(&ctx, &NoiseCop, &params).expect("execute failed");
        let pixels = img.to_cpu().expect("readback failed");
        assert!(is_non_flat(&pixels), "Simplex noise produced a flat image");
    }

    #[test]
    fn worley_non_flat() {
        let Some(ctx) = try_ctx() else { return; };

        let params = NoiseParams {
            noise_type: NoiseType::Worley,
            width: 32,
            height: 32,
            ..Default::default()
        };

        let img = generate_cop(&ctx, &NoiseCop, &params).expect("execute failed");
        let pixels = img.to_cpu().expect("readback failed");
        assert!(is_non_flat(&pixels), "Worley noise produced a flat image");
    }

    #[test]
    fn different_seeds_produce_different_results() {
        let Some(ctx) = try_ctx() else { return; };

        let params_a = NoiseParams {
            noise_type: NoiseType::Perlin,
            seed: 0,
            width: 16,
            height: 16,
            ..Default::default()
        };
        let params_b = NoiseParams {
            seed: 42,
            ..params_a.clone()
        };

        let img_a = generate_cop(&ctx, &NoiseCop, &params_a).expect("execute failed");
        let img_b = generate_cop(&ctx, &NoiseCop, &params_b).expect("execute failed");

        let pixels_a = img_a.to_cpu().expect("readback failed");
        let pixels_b = img_b.to_cpu().expect("readback failed");

        let different = pixels_a
            .iter()
            .zip(pixels_b.iter())
            .any(|(a, b)| (a - b).abs() > 1e-4);

        assert!(different, "Different seeds produced identical noise");
    }
}
