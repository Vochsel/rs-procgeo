// SwirlCop — applies a swirl/twist distortion effect

use std::sync::Arc;

use crate::context::GpuContext;
use crate::image::Image;
use crate::{Cop, CopError, dispatch_2d, hash_name};

/// Parameters for the Swirl COP.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SwirlParams {
    /// Center of the swirl effect in normalized UV coordinates [0.0, 1.0].
    pub center: [f32; 2],
    /// Maximum swirl angle in degrees at the center.
    pub angle: f32,
    /// Radius of the swirl effect in normalized UV space.
    pub radius: f32,
}

impl Default for SwirlParams {
    fn default() -> Self {
        Self {
            center: [0.5, 0.5],
            angle: 90.0,
            radius: 0.5,
        }
    }
}

/// Uniform buffer layout for the swirl shader (16 bytes).
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct SwirlUniform {
    center: [f32; 2],
    angle: f32,
    radius: f32,
}

/// Filter COP that applies a swirl distortion.
pub struct SwirlCop;

const SHADER_SOURCE: &str = include_str!("../../shaders/swirl.wgsl");

impl Cop for SwirlCop {
    type Params = SwirlParams;

    fn name(&self) -> &'static str {
        "swirl"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 1)
    }

    fn execute(&self, ctx: &Arc<GpuContext>, inputs: &[&Image], params: &SwirlParams) -> Result<Image, CopError> {
        self.validate_inputs(inputs)?;
        let input = inputs[0];

        let output = Image::create_storage(Arc::clone(ctx), input.width(), input.height());

        let uniform = SwirlUniform {
            center: params.center,
            angle: params.angle,
            radius: params.radius,
        };

        let uniform_buffer = ctx.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("swirl_params"),
            size: std::mem::size_of::<SwirlUniform>() as u64,
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

        let pipeline_key = hash_name("swirl");
        let pipeline = ctx.get_or_create_pipeline(pipeline_key, SHADER_SOURCE, "main")?;

        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let bind_group = ctx.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("swirl_bind_group"),
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
                    label: Some("swirl_dispatch"),
                });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("swirl_pass"),
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
    fn swirl_does_not_crash() {
        let ctx = match GpuContext::new_blocking() {
            Ok(c) => Arc::new(c),
            Err(e) => {
                eprintln!("Skipping swirl_does_not_crash test (no GPU): {e}");
                return;
            }
        };

        let data: Vec<f32> = (0..8 * 8).flat_map(|_| [1.0f32, 0.5, 0.0, 1.0]).collect();
        let input = Image::from_cpu(Arc::clone(&ctx), 8, 8, &data).expect("from_cpu failed");

        let params = SwirlParams::default();
        let output = SwirlCop.execute(&ctx, &[&input], &params).expect("execute failed");

        assert_eq!(output.width(), 8);
        assert_eq!(output.height(), 8);

        let pixels = output.to_cpu().expect("readback failed");
        assert_eq!(pixels.len(), 8 * 8 * 4);
    }
}
