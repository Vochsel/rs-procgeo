// MirrorCop — mirrors an image across a horizontal or vertical axis

use std::sync::Arc;

use crate::context::GpuContext;
use crate::image::Image;
use crate::{Cop, CopError, dispatch_2d, hash_name};

/// Which axis to mirror across.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum MirrorAxis {
    #[default]
    X,
    Y,
}

/// Parameters for the Mirror COP.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct MirrorParams {
    /// Axis to mirror across.
    pub axis: MirrorAxis,
    /// Normalized position of the mirror line (0.0–1.0).
    pub offset: f32,
}

impl Default for MirrorParams {
    fn default() -> Self {
        Self {
            axis: MirrorAxis::X,
            offset: 0.5,
        }
    }
}

/// Uniform buffer layout for the mirror shader (16 bytes).
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct MirrorUniform {
    axis: u32,
    offset: f32,
    _pad: [u32; 2],
}

/// Filter COP that mirrors an image across a horizontal or vertical axis.
pub struct MirrorCop;

const SHADER_SOURCE: &str = include_str!("../../shaders/mirror.wgsl");

impl Cop for MirrorCop {
    type Params = MirrorParams;

    fn name(&self) -> &'static str {
        "mirror"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 1)
    }

    fn execute(&self, inputs: &[&Image], params: &MirrorParams) -> Result<Image, CopError> {
        self.validate_inputs(inputs)?;

        let ctx: Arc<GpuContext> = Arc::clone(inputs[0].ctx());
        let input = inputs[0];

        let output = Image::create_storage(Arc::clone(&ctx), input.width(), input.height());

        let uniform = MirrorUniform {
            axis: match params.axis {
                MirrorAxis::X => 0,
                MirrorAxis::Y => 1,
            },
            offset: params.offset,
            _pad: [0; 2],
        };

        let uniform_buffer = ctx.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("mirror_params"),
            size: std::mem::size_of::<MirrorUniform>() as u64,
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

        let pipeline_key = hash_name("mirror");
        let pipeline = ctx.get_or_create_pipeline(pipeline_key, SHADER_SOURCE, "main")?;

        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let bind_group = ctx.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("mirror_bind_group"),
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
                    label: Some("mirror_dispatch"),
                });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("mirror_pass"),
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
    fn mirror_x_at_half() {
        let ctx = match GpuContext::new_blocking() {
            Ok(c) => Arc::new(c),
            Err(e) => {
                eprintln!("Skipping mirror_x_at_half test (no GPU): {e}");
                return;
            }
        };

        // 4x1 image: [red, green, blue, white]
        let data: Vec<f32> = vec![
            1.0, 0.0, 0.0, 1.0, // pixel 0: red
            0.0, 1.0, 0.0, 1.0, // pixel 1: green
            0.0, 0.0, 1.0, 1.0, // pixel 2: blue
            1.0, 1.0, 1.0, 1.0, // pixel 3: white
        ];
        let input = Image::from_cpu(Arc::clone(&ctx), 4, 1, &data).expect("from_cpu failed");

        let params = MirrorParams {
            axis: MirrorAxis::X,
            offset: 0.5,
        };

        let output = MirrorCop.execute(&[&input], &params).expect("execute failed");
        let pixels = output.to_cpu().expect("readback failed");

        // Left half (x < 2) should be unchanged; right half mirrors left
        // pixel 0 (x=0): unchanged → red
        assert!(
            (pixels[0] - 1.0).abs() < 1e-5 && pixels[1].abs() < 1e-5 && pixels[2].abs() < 1e-5,
            "pixel 0 should be red, got {:?}",
            &pixels[0..4]
        );
        // pixel 1 (x=1): unchanged → green
        assert!(
            pixels[4].abs() < 1e-5 && (pixels[5] - 1.0).abs() < 1e-5 && pixels[6].abs() < 1e-5,
            "pixel 1 should be green, got {:?}",
            &pixels[4..8]
        );
    }
}
