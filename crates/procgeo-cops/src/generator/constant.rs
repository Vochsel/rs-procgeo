// ConstantCop — fills an image with a uniform color via GPU compute shader

use std::sync::Arc;

use crate::context::GpuContext;
use crate::image::Image;
use crate::{Cop, CopError, dispatch_2d, hash_name};

/// Parameters for the Constant COP.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ConstantParams {
    /// RGBA color to fill (each channel 0.0–1.0).
    pub color: [f32; 4],
    /// Output width in pixels.
    pub width: u32,
    /// Output height in pixels.
    pub height: u32,
}

impl Default for ConstantParams {
    fn default() -> Self {
        Self {
            color: [0.0, 0.0, 0.0, 1.0],
            width: 256,
            height: 256,
        }
    }
}

/// Generator COP that fills every pixel with a constant color.
pub struct ConstantCop;

const SHADER_SOURCE: &str = include_str!("../../shaders/constant.wgsl");

impl Cop for ConstantCop {
    type Params = ConstantParams;

    fn name(&self) -> &'static str {
        "constant"
    }

    fn input_count(&self) -> (usize, usize) {
        (0, 0)
    }

    fn execute(
        &self,
        ctx: &Arc<GpuContext>,
        inputs: &[&Image],
        params: &ConstantParams,
    ) -> Result<Image, CopError> {
        self.validate_inputs(inputs)?;

        // Create the output storage texture
        let output = Image::create_storage(Arc::clone(ctx), params.width, params.height);

        // Create uniform buffer with the color data (4 x f32 = 16 bytes)
        let uniform_buffer = ctx.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("constant_params"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        ctx.queue().write_buffer(
            &uniform_buffer,
            0,
            bytemuck::cast_slice(&params.color),
        );

        // Create a texture view for the output
        let output_view = output
            .texture()
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Get or create the cached compute pipeline
        let pipeline_key = hash_name("constant");
        let pipeline = ctx.get_or_create_pipeline(pipeline_key, SHADER_SOURCE, "main")?;

        // Create the bind group matching the shader layout
        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let bind_group = ctx.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("constant_bind_group"),
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

        // Encode and dispatch the compute shader
        let mut encoder =
            ctx.device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("constant_dispatch"),
                });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("constant_pass"),
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

    #[test]
    fn constant_red() {
        let ctx = match GpuContext::new_blocking() {
            Ok(c) => Arc::new(c),
            Err(e) => {
                eprintln!("Skipping constant_red test (no GPU): {e}");
                return;
            }
        };

        let params = ConstantParams {
            color: [1.0, 0.0, 0.0, 1.0],
            width: 8,
            height: 8,
        };

        let img = generate_cop(&ctx, &ConstantCop, &params).expect("execute failed");
        assert_eq!(img.width(), 8);
        assert_eq!(img.height(), 8);

        let pixels = img.to_cpu().expect("readback failed");
        assert_eq!(pixels.len(), 8 * 8 * 4);

        for (i, chunk) in pixels.chunks_exact(4).enumerate() {
            assert!(
                (chunk[0] - 1.0).abs() < 1e-5
                    && chunk[1].abs() < 1e-5
                    && chunk[2].abs() < 1e-5
                    && (chunk[3] - 1.0).abs() < 1e-5,
                "pixel {i} mismatch: got {:?}, expected [1, 0, 0, 1]",
                chunk
            );
        }
    }

    #[test]
    fn constant_default_is_black() {
        let ctx = match GpuContext::new_blocking() {
            Ok(c) => Arc::new(c),
            Err(e) => {
                eprintln!("Skipping constant_default_is_black test (no GPU): {e}");
                return;
            }
        };

        let mut params = ConstantParams::default();
        params.width = 4;
        params.height = 4;

        let img = generate_cop(&ctx, &ConstantCop, &params).expect("execute failed");
        assert_eq!(img.width(), 4);
        assert_eq!(img.height(), 4);

        let pixels = img.to_cpu().expect("readback failed");
        assert_eq!(pixels.len(), 4 * 4 * 4);

        for (i, chunk) in pixels.chunks_exact(4).enumerate() {
            assert!(
                chunk[0].abs() < 1e-5
                    && chunk[1].abs() < 1e-5
                    && chunk[2].abs() < 1e-5
                    && (chunk[3] - 1.0).abs() < 1e-5,
                "pixel {i} mismatch: got {:?}, expected [0, 0, 0, 1]",
                chunk
            );
        }
    }
}
