// RotateCop — rotates an image around a center point

use std::sync::Arc;

use crate::context::GpuContext;
use crate::image::Image;
use crate::{Cop, CopError, FilterMode, dispatch_2d, hash_name};

/// Parameters for the Rotate COP.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RotateParams {
    /// Rotation angle in degrees (positive = counter-clockwise).
    pub angle: f32,
    /// Center of rotation in normalized UV coordinates [0.0, 1.0].
    pub center: [f32; 2],
    /// Sampling filter mode.
    pub filter: FilterMode,
}

impl Default for RotateParams {
    fn default() -> Self {
        Self {
            angle: 0.0,
            center: [0.5, 0.5],
            filter: FilterMode::Nearest,
        }
    }
}

/// Uniform buffer layout for the rotate shader (16 bytes).
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct RotateUniform {
    center: [f32; 2],
    angle: f32,
    filter_mode: u32,
}

/// Filter COP that rotates an image around a center point.
pub struct RotateCop;

const SHADER_SOURCE: &str = include_str!("../../shaders/rotate.wgsl");

impl Cop for RotateCop {
    type Params = RotateParams;

    fn name(&self) -> &'static str {
        "rotate"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 1)
    }

    fn execute(&self, ctx: &Arc<GpuContext>, inputs: &[&Image], params: &RotateParams) -> Result<Image, CopError> {
        self.validate_inputs(inputs)?;
        let input = inputs[0];

        let output = Image::create_storage(Arc::clone(ctx), input.width(), input.height());

        let uniform = RotateUniform {
            center: params.center,
            angle: params.angle,
            filter_mode: match params.filter {
                FilterMode::Nearest => 0,
                FilterMode::Bilinear => 1,
            },
        };

        let uniform_buffer = ctx.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("rotate_params"),
            size: std::mem::size_of::<RotateUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        ctx.queue()
            .write_buffer(&uniform_buffer, 0, bytemuck::bytes_of(&uniform));

        let input_view = input
            .texture()
            .create_view(&wgpu::TextureViewDescriptor::default());
        let output_view = output
            .texture()
            .create_view(&wgpu::TextureViewDescriptor::default());

        let pipeline_key = hash_name("rotate");
        let pipeline = ctx.get_or_create_pipeline(pipeline_key, SHADER_SOURCE, "main")?;

        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let bind_group = ctx.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rotate_bind_group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&input_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&output_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });

        let mut encoder =
            ctx.device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("rotate_dispatch"),
                });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("rotate_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            dispatch_2d(&mut pass, input.width(), input.height());
        }

        ctx.queue().submit(std::iter::once(encoder.finish()));

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotate_zero_preserves_image() {
        let ctx = match GpuContext::new_blocking() {
            Ok(c) => Arc::new(c),
            Err(e) => {
                eprintln!("Skipping rotate_zero_preserves_image test (no GPU): {e}");
                return;
            }
        };

        // Create a simple test image with known pixel values
        let data: Vec<f32> = (0..4u32)
            .flat_map(|i| {
                let v = i as f32 / 3.0;
                [v, 0.5, 1.0 - v, 1.0]
            })
            .collect();
        let input = Image::from_cpu(Arc::clone(&ctx), 2, 2, &data).expect("from_cpu failed");

        let params = RotateParams {
            angle: 0.0,
            center: [0.5, 0.5],
            filter: FilterMode::Nearest,
        };

        let output = RotateCop.execute(&ctx, &[&input], &params).expect("execute failed");
        let pixels = output.to_cpu().expect("readback failed");

        assert_eq!(pixels.len(), data.len());

        // With 0 degree rotation and nearest filter, pixels should match
        for (i, (expected, actual)) in data.iter().zip(pixels.iter()).enumerate() {
            assert!(
                (expected - actual).abs() < 1e-4,
                "pixel channel {i}: expected {expected}, got {actual}"
            );
        }
    }
}
