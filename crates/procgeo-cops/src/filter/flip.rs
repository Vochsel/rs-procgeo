// FlipCop — flips an image horizontally and/or vertically

use std::sync::Arc;

use crate::context::GpuContext;
use crate::image::Image;
use crate::{Cop, CopError, dispatch_2d, hash_name};

/// Parameters for the Flip COP.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct FlipParams {
    /// Flip horizontally (mirror along the vertical axis).
    pub horizontal: bool,
    /// Flip vertically (mirror along the horizontal axis).
    pub vertical: bool,
}

impl Default for FlipParams {
    fn default() -> Self {
        Self {
            horizontal: false,
            vertical: true,
        }
    }
}

/// Uniform buffer layout for the flip shader (must be 16 bytes / 4 u32s).
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct FlipUniform {
    horizontal: u32,
    vertical: u32,
    _pad: [u32; 2],
}

/// Filter COP that flips an image horizontally and/or vertically.
pub struct FlipCop;

const SHADER_SOURCE: &str = include_str!("../../shaders/flip.wgsl");

impl Cop for FlipCop {
    type Params = FlipParams;

    fn name(&self) -> &'static str {
        "flip"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 1)
    }

    fn execute(&self, inputs: &[&Image], params: &FlipParams) -> Result<Image, CopError> {
        self.validate_inputs(inputs)?;

        let ctx: Arc<GpuContext> = Arc::clone(inputs[0].ctx());
        let input = inputs[0];

        let output = Image::create_storage(Arc::clone(&ctx), input.width(), input.height());

        let uniform = FlipUniform {
            horizontal: params.horizontal as u32,
            vertical: params.vertical as u32,
            _pad: [0; 2],
        };

        let uniform_buffer = ctx.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("flip_params"),
            size: std::mem::size_of::<FlipUniform>() as u64,
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

        let pipeline_key = hash_name("flip");
        let pipeline = ctx.get_or_create_pipeline(pipeline_key, SHADER_SOURCE, "main")?;

        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let bind_group = ctx.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("flip_bind_group"),
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
                    label: Some("flip_dispatch"),
                });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("flip_pass"),
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
    fn flip_horizontal_gradient() {
        let ctx = match GpuContext::new_blocking() {
            Ok(c) => Arc::new(c),
            Err(e) => {
                eprintln!("Skipping flip_horizontal_gradient test (no GPU): {e}");
                return;
            }
        };

        // 4x1 image: gradient [0.25, 0.5, 0.75, 1.0] in red channel
        let data: Vec<f32> = vec![
            0.25, 0.0, 0.0, 1.0,
            0.5,  0.0, 0.0, 1.0,
            0.75, 0.0, 0.0, 1.0,
            1.0,  0.0, 0.0, 1.0,
        ];
        let input = Image::from_cpu(Arc::clone(&ctx), 4, 1, &data).expect("from_cpu failed");

        let params = FlipParams {
            horizontal: true,
            vertical: false,
        };

        let output = FlipCop.execute(&[&input], &params).expect("execute failed");
        let pixels = output.to_cpu().expect("readback failed");

        // After horizontal flip, order should be [1.0, 0.75, 0.5, 0.25]
        let reds: Vec<f32> = pixels.chunks_exact(4).map(|c| c[0]).collect();
        assert!(
            (reds[0] - 1.0).abs() < 1e-5,
            "pixel 0 red expected 1.0, got {}",
            reds[0]
        );
        assert!(
            (reds[1] - 0.75).abs() < 1e-5,
            "pixel 1 red expected 0.75, got {}",
            reds[1]
        );
        assert!(
            (reds[2] - 0.5).abs() < 1e-5,
            "pixel 2 red expected 0.5, got {}",
            reds[2]
        );
        assert!(
            (reds[3] - 0.25).abs() < 1e-5,
            "pixel 3 red expected 0.25, got {}",
            reds[3]
        );
    }

    #[test]
    fn flip_vertical_red_over_blue() {
        let ctx = match GpuContext::new_blocking() {
            Ok(c) => Arc::new(c),
            Err(e) => {
                eprintln!("Skipping flip_vertical_red_over_blue test (no GPU): {e}");
                return;
            }
        };

        // 1x2 image: top row = red, bottom row = blue
        let data: Vec<f32> = vec![
            1.0, 0.0, 0.0, 1.0, // row 0 (top): red
            0.0, 0.0, 1.0, 1.0, // row 1 (bottom): blue
        ];
        let input = Image::from_cpu(Arc::clone(&ctx), 1, 2, &data).expect("from_cpu failed");

        let params = FlipParams {
            horizontal: false,
            vertical: true,
        };

        let output = FlipCop.execute(&[&input], &params).expect("execute failed");
        let pixels = output.to_cpu().expect("readback failed");

        // After vertical flip, top row should be blue and bottom row should be red
        let top = &pixels[0..4];
        let bottom = &pixels[4..8];

        assert!(
            top[2].abs() - 1.0 < 1e-5,
            "top pixel should be blue, got {:?}",
            top
        );
        assert!(
            bottom[0].abs() - 1.0 < 1e-5,
            "bottom pixel should be red, got {:?}",
            bottom
        );
    }
}
