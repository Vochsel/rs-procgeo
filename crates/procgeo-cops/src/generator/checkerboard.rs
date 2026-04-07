// CheckerboardCop — generates a checkerboard pattern via GPU compute shader

use std::sync::Arc;

use bytemuck::{Pod, Zeroable};

use crate::context::GpuContext;
use crate::image::Image;
use crate::{Cop, CopError, dispatch_2d, hash_name};

/// Parameters for the Checkerboard COP.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CheckerboardParams {
    /// First (even) checker color — RGBA (0.0–1.0).
    pub color_a: [f32; 4],
    /// Second (odd) checker color — RGBA (0.0–1.0).
    pub color_b: [f32; 4],
    /// Number of checker tiles in X and Y.
    pub frequency: [f32; 2],
    /// Output width in pixels.
    pub width: u32,
    /// Output height in pixels.
    pub height: u32,
}

impl Default for CheckerboardParams {
    fn default() -> Self {
        Self {
            color_a: [0.0, 0.0, 0.0, 1.0],
            color_b: [1.0, 1.0, 1.0, 1.0],
            frequency: [8.0, 8.0],
            width: 256,
            height: 256,
        }
    }
}

/// GPU-side uniform layout (must match checkerboard.wgsl `Params` struct).
/// Total size: 4*4 + 4*4 + 2*4 + 2*4 = 48 bytes.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct CheckerboardUniform {
    color_a: [f32; 4],
    color_b: [f32; 4],
    frequency: [f32; 2],
    _pad: [f32; 2],
}

/// Generator COP that fills an image with an alternating checker pattern.
pub struct CheckerboardCop;

const SHADER_SOURCE: &str = include_str!("../../shaders/checkerboard.wgsl");

impl Cop for CheckerboardCop {
    type Params = CheckerboardParams;

    fn name(&self) -> &'static str {
        "checkerboard"
    }

    fn input_count(&self) -> (usize, usize) {
        (0, 0)
    }

    fn execute(
        &self,
        ctx: &Arc<GpuContext>,
        inputs: &[&Image],
        params: &CheckerboardParams,
    ) -> Result<Image, CopError> {
        self.validate_inputs(inputs)?;

        let output = Image::create_storage(Arc::clone(ctx), params.width, params.height);

        let uniform_data = CheckerboardUniform {
            color_a: params.color_a,
            color_b: params.color_b,
            frequency: params.frequency,
            _pad: [0.0; 2],
        };

        let uniform_buffer = ctx.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("checkerboard_params"),
            size: std::mem::size_of::<CheckerboardUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        ctx.queue()
            .write_buffer(&uniform_buffer, 0, bytemuck::bytes_of(&uniform_data));

        let output_view = output
            .texture()
            .create_view(&wgpu::TextureViewDescriptor::default());

        let pipeline_key = hash_name("checkerboard");
        let pipeline = ctx.get_or_create_pipeline(pipeline_key, SHADER_SOURCE, "main")?;

        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let bind_group = ctx.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("checkerboard_bind_group"),
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
                    label: Some("checkerboard_dispatch"),
                });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("checkerboard_pass"),
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
    fn checkerboard_pattern() {
        let ctx = match GpuContext::new_blocking() {
            Ok(c) => Arc::new(c),
            Err(e) => {
                eprintln!("Skipping checkerboard_pattern test (no GPU): {e}");
                return;
            }
        };

        // 4x4 image with frequency [2,2] — each checker tile is 2x2 pixels
        // top-left 2x2 tile: color_a (black), top-right 2x2 tile: color_b (white)
        let params = CheckerboardParams {
            color_a: [0.0, 0.0, 0.0, 1.0],
            color_b: [1.0, 1.0, 1.0, 1.0],
            frequency: [2.0, 2.0],
            width: 4,
            height: 4,
        };

        let img = generate_cop(&ctx, &CheckerboardCop, &params).expect("execute failed");
        assert_eq!(img.width(), 4);
        assert_eq!(img.height(), 4);

        let pixels = img.to_cpu().expect("readback failed");
        assert_eq!(pixels.len(), 4 * 4 * 4);

        // Helper to read pixel at (x, y)
        let pixel = |x: usize, y: usize| -> [f32; 4] {
            let base = (y * 4 + x) * 4;
            [pixels[base], pixels[base + 1], pixels[base + 2], pixels[base + 3]]
        };

        // Top-left pixel (0,0) should be color_a (black)
        let tl = pixel(0, 0);
        assert!(
            tl[0].abs() < 1e-4,
            "top-left should be color_a (black), got {:?}",
            tl
        );

        // Pixel (2,0) should be color_b (white)
        let tr = pixel(2, 0);
        assert!(
            (tr[0] - 1.0).abs() < 1e-4,
            "pixel (2,0) should be color_b (white), got {:?}",
            tr
        );
    }

    #[test]
    fn checkerboard_default_params() {
        let ctx = match GpuContext::new_blocking() {
            Ok(c) => Arc::new(c),
            Err(e) => {
                eprintln!("Skipping checkerboard_default_params test (no GPU): {e}");
                return;
            }
        };

        let mut params = CheckerboardParams::default();
        params.width = 16;
        params.height = 16;

        let img = generate_cop(&ctx, &CheckerboardCop, &params).expect("execute failed");
        assert_eq!(img.width(), 16);
        assert_eq!(img.height(), 16);

        let pixels = img.to_cpu().expect("readback failed");
        assert_eq!(pixels.len(), 16 * 16 * 4);
    }
}
