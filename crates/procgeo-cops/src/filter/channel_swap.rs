// ChannelSwapCop — reorders RGBA channels of an image

use std::sync::Arc;

use crate::context::GpuContext;
use crate::image::Image;
use crate::{Cop, CopError, dispatch_2d, hash_name};

/// Source channel for a channel swap operation.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum Channel {
    R,
    G,
    B,
    A,
    One,
    Zero,
}

impl Channel {
    fn as_u32(&self) -> u32 {
        match self {
            Channel::R => 0,
            Channel::G => 1,
            Channel::B => 2,
            Channel::A => 3,
            Channel::One => 4,
            Channel::Zero => 5,
        }
    }
}

/// Parameters for the ChannelSwap COP.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ChannelSwapParams {
    /// Source channel for the output R channel.
    pub r: Channel,
    /// Source channel for the output G channel.
    pub g: Channel,
    /// Source channel for the output B channel.
    pub b: Channel,
    /// Source channel for the output A channel.
    pub a: Channel,
}

impl Default for ChannelSwapParams {
    fn default() -> Self {
        Self {
            r: Channel::R,
            g: Channel::G,
            b: Channel::B,
            a: Channel::A,
        }
    }
}

/// Uniform buffer layout for the channel swap shader (16 bytes).
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ChannelSwapUniform {
    r_src: u32,
    g_src: u32,
    b_src: u32,
    a_src: u32,
}

/// Filter COP that reorders RGBA channels.
pub struct ChannelSwapCop;

const SHADER_SOURCE: &str = include_str!("../../shaders/channel_swap.wgsl");

impl Cop for ChannelSwapCop {
    type Params = ChannelSwapParams;

    fn name(&self) -> &'static str {
        "channel_swap"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 1)
    }

    fn execute(
        &self,
        inputs: &[&Image],
        params: &ChannelSwapParams,
    ) -> Result<Image, CopError> {
        self.validate_inputs(inputs)?;

        let ctx: Arc<GpuContext> = Arc::clone(inputs[0].ctx());
        let input = inputs[0];

        let output = Image::create_storage(Arc::clone(&ctx), input.width(), input.height());

        let uniform = ChannelSwapUniform {
            r_src: params.r.as_u32(),
            g_src: params.g.as_u32(),
            b_src: params.b.as_u32(),
            a_src: params.a.as_u32(),
        };

        let uniform_buffer = ctx.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("channel_swap_params"),
            size: std::mem::size_of::<ChannelSwapUniform>() as u64,
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

        let pipeline_key = hash_name("channel_swap");
        let pipeline = ctx.get_or_create_pipeline(pipeline_key, SHADER_SOURCE, "main")?;

        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let bind_group = ctx.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("channel_swap_bind_group"),
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
                    label: Some("channel_swap_dispatch"),
                });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("channel_swap_pass"),
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
    fn channel_swap_red_to_blue() {
        let ctx = match GpuContext::new_blocking() {
            Ok(c) => Arc::new(c),
            Err(e) => {
                eprintln!("Skipping channel_swap_red_to_blue test (no GPU): {e}");
                return;
            }
        };

        // 1x1 red pixel
        let data: Vec<f32> = vec![1.0, 0.0, 0.0, 1.0];
        let input = Image::from_cpu(Arc::clone(&ctx), 1, 1, &data).expect("from_cpu failed");

        // Swap R and B channels
        let params = ChannelSwapParams {
            r: Channel::B,
            g: Channel::G,
            b: Channel::R,
            a: Channel::A,
        };

        let output = ChannelSwapCop
            .execute(&[&input], &params)
            .expect("execute failed");
        let pixels = output.to_cpu().expect("readback failed");

        // Red pixel with R<->B swap should become blue: [0, 0, 1, 1]
        assert!(
            pixels[0].abs() < 1e-5,
            "R should be 0.0, got {}",
            pixels[0]
        );
        assert!(
            pixels[1].abs() < 1e-5,
            "G should be 0.0, got {}",
            pixels[1]
        );
        assert!(
            (pixels[2] - 1.0).abs() < 1e-5,
            "B should be 1.0, got {}",
            pixels[2]
        );
        assert!(
            (pixels[3] - 1.0).abs() < 1e-5,
            "A should be 1.0, got {}",
            pixels[3]
        );
    }
}
