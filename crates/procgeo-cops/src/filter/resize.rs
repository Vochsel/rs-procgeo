// ResizeCop — resizes an image to a new resolution

use std::sync::Arc;

use crate::context::GpuContext;
use crate::image::Image;
use crate::{Cop, CopError, FilterMode, dispatch_2d, hash_name};

/// Parameters for the Resize COP.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ResizeParams {
    /// Output width in pixels.
    pub width: u32,
    /// Output height in pixels.
    pub height: u32,
    /// Sampling filter mode.
    pub filter: FilterMode,
}

impl Default for ResizeParams {
    fn default() -> Self {
        Self {
            width: 256,
            height: 256,
            filter: FilterMode::Nearest,
        }
    }
}

/// Uniform buffer layout for the resize shader (16 bytes).
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ResizeUniform {
    src_width: u32,
    src_height: u32,
    filter_mode: u32,
    _pad: u32,
}

/// Filter COP that resizes an image to new dimensions.
pub struct ResizeCop;

const SHADER_SOURCE: &str = include_str!("../../shaders/resize.wgsl");

impl Cop for ResizeCop {
    type Params = ResizeParams;

    fn name(&self) -> &'static str {
        "resize"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 1)
    }

    fn execute(&self, inputs: &[&Image], params: &ResizeParams) -> Result<Image, CopError> {
        self.validate_inputs(inputs)?;

        if params.width == 0 || params.height == 0 {
            return Err(CopError::InvalidParam(
                "resize width and height must be > 0".into(),
            ));
        }

        let ctx: Arc<GpuContext> = Arc::clone(inputs[0].ctx());
        let input = inputs[0];

        // Output dimensions come from params, NOT from input
        let output = Image::create_storage(Arc::clone(&ctx), params.width, params.height);

        let uniform = ResizeUniform {
            src_width: input.width(),
            src_height: input.height(),
            filter_mode: match params.filter {
                FilterMode::Nearest => 0,
                FilterMode::Bilinear => 1,
            },
            _pad: 0,
        };

        let uniform_buffer = ctx.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("resize_params"),
            size: std::mem::size_of::<ResizeUniform>() as u64,
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

        let pipeline_key = hash_name("resize");
        let pipeline = ctx.get_or_create_pipeline(pipeline_key, SHADER_SOURCE, "main")?;

        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let bind_group = ctx.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("resize_bind_group"),
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
                    label: Some("resize_dispatch"),
                });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("resize_pass"),
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

    #[test]
    fn resize_same_size_preserves_image() {
        let ctx = match GpuContext::new_blocking() {
            Ok(c) => Arc::new(c),
            Err(e) => {
                eprintln!("Skipping resize_same_size_preserves_image test (no GPU): {e}");
                return;
            }
        };

        let data: Vec<f32> = vec![
            1.0, 0.0, 0.0, 1.0, // red
            0.0, 1.0, 0.0, 1.0, // green
            0.0, 0.0, 1.0, 1.0, // blue
            1.0, 1.0, 0.0, 1.0, // yellow
        ];
        let input = Image::from_cpu(Arc::clone(&ctx), 2, 2, &data).expect("from_cpu failed");

        let params = ResizeParams {
            width: 2,
            height: 2,
            filter: FilterMode::Nearest,
        };

        let output = ResizeCop.execute(&[&input], &params).expect("execute failed");
        assert_eq!(output.width(), 2);
        assert_eq!(output.height(), 2);

        let pixels = output.to_cpu().expect("readback failed");
        assert_eq!(pixels.len(), 2 * 2 * 4);

        for (i, (expected, actual)) in data.iter().zip(pixels.iter()).enumerate() {
            assert!(
                (expected - actual).abs() < 1e-4,
                "channel {i}: expected {expected}, got {actual}"
            );
        }
    }

    #[test]
    fn resize_changes_dimensions() {
        let ctx = match GpuContext::new_blocking() {
            Ok(c) => Arc::new(c),
            Err(e) => {
                eprintln!("Skipping resize_changes_dimensions test (no GPU): {e}");
                return;
            }
        };

        let data: Vec<f32> = (0..4 * 4).flat_map(|_| [0.5f32, 0.5, 0.5, 1.0]).collect();
        let input = Image::from_cpu(Arc::clone(&ctx), 4, 4, &data).expect("from_cpu failed");

        let params = ResizeParams {
            width: 8,
            height: 8,
            filter: FilterMode::Bilinear,
        };

        let output = ResizeCop.execute(&[&input], &params).expect("execute failed");
        assert_eq!(output.width(), 8);
        assert_eq!(output.height(), 8);

        let pixels = output.to_cpu().expect("readback failed");
        assert_eq!(pixels.len(), 8 * 8 * 4);
    }
}
