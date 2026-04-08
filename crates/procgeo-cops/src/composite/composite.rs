// CompositeCop — combines two input images with various blend modes

use std::sync::Arc;

use crate::context::GpuContext;
use crate::image::Image;
use crate::{Cop, CopError, dispatch_2d, hash_name};

/// Compositing operation (blend mode).
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CompOp {
    /// Alpha-compositing (Porter-Duff over).
    #[default]
    Over,
    /// Additive blend.
    Add,
    /// Multiplicative blend.
    Multiply,
    /// Screen blend.
    Screen,
    /// Subtract B from A.
    Subtract,
    /// Absolute difference.
    Difference,
    /// Per-channel minimum.
    Min,
    /// Per-channel maximum.
    Max,
}

impl CompOp {
    fn as_u32(&self) -> u32 {
        match self {
            CompOp::Over => 0,
            CompOp::Add => 1,
            CompOp::Multiply => 2,
            CompOp::Screen => 3,
            CompOp::Subtract => 4,
            CompOp::Difference => 5,
            CompOp::Min => 6,
            CompOp::Max => 7,
        }
    }
}

/// Parameters for the Composite COP.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CompositeParams {
    /// Blend operation.
    pub operation: CompOp,
    /// Mix factor (0.0 = use input_a unchanged, 1.0 = full operation result).
    pub mix: f32,
}

impl Default for CompositeParams {
    fn default() -> Self {
        Self {
            operation: CompOp::Over,
            mix: 1.0,
        }
    }
}

/// Uniform buffer layout for the composite shader (16 bytes).
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct CompositeUniform {
    operation: u32,
    mix_factor: f32,
    _pad: [u32; 2],
}

/// Compositor COP that blends two images using various operations.
pub struct CompositeCop;

const SHADER_SOURCE: &str = include_str!("../../shaders/composite.wgsl");

impl Cop for CompositeCop {
    type Params = CompositeParams;

    fn name(&self) -> &'static str {
        "composite"
    }

    fn input_count(&self) -> (usize, usize) {
        (2, 2)
    }

    fn execute(
        &self,
        ctx: &Arc<GpuContext>,
        inputs: &[&Image],
        params: &CompositeParams,
    ) -> Result<Image, CopError> {
        self.validate_inputs(inputs)?;
        let input_a = inputs[0];
        let input_b = inputs[1];

        // Output dimensions match input_a
        let output = Image::create_storage(Arc::clone(ctx), input_a.width(), input_a.height());

        let uniform = CompositeUniform {
            operation: params.operation.as_u32(),
            mix_factor: params.mix,
            _pad: [0; 2],
        };

        let uniform_buffer = ctx.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("composite_params"),
            size: std::mem::size_of::<CompositeUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        ctx.queue()
            .write_buffer(&uniform_buffer, 0, bytemuck::bytes_of(&uniform));

        let view_a = input_a
            .texture()
            .create_view(&wgpu::TextureViewDescriptor::default());
        let view_b = input_b
            .texture()
            .create_view(&wgpu::TextureViewDescriptor::default());
        let output_view = output
            .texture()
            .create_view(&wgpu::TextureViewDescriptor::default());

        let pipeline_key = hash_name("composite");
        let pipeline = ctx.get_or_create_pipeline(pipeline_key, SHADER_SOURCE, "main")?;

        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let bind_group = ctx.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("composite_bind_group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view_a),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view_b),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&output_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });

        let mut encoder = ctx
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("composite_dispatch"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("composite_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            dispatch_2d(&mut pass, input_a.width(), input_a.height());
        }

        ctx.queue().submit(std::iter::once(encoder.finish()));

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::Image;

    fn make_ctx() -> Option<Arc<GpuContext>> {
        match GpuContext::new_blocking() {
            Ok(c) => Some(Arc::new(c)),
            Err(e) => {
                eprintln!("Skipping test (no GPU): {e}");
                None
            }
        }
    }

    /// Create a solid color 4x4 image.
    fn solid(ctx: Arc<GpuContext>, r: f32, g: f32, b: f32, a: f32) -> Image {
        let data: Vec<f32> = std::iter::repeat([r, g, b, a])
            .take(4 * 4)
            .flatten()
            .collect();
        Image::from_cpu(ctx, 4, 4, &data).expect("from_cpu failed")
    }

    #[test]
    fn multiply_half_half() {
        let ctx = match make_ctx() {
            Some(c) => c,
            None => return,
        };

        // 0.5 * 0.5 = 0.25
        let a = solid(Arc::clone(&ctx), 0.5, 0.5, 0.5, 1.0);
        let b = solid(Arc::clone(&ctx), 0.5, 0.5, 0.5, 1.0);

        let params = CompositeParams {
            operation: CompOp::Multiply,
            mix: 1.0,
        };
        let out = CompositeCop
            .execute(&ctx, &[&a, &b], &params)
            .expect("execute failed");
        let pixels = out.to_cpu().expect("readback failed");

        for chunk in pixels.chunks_exact(4) {
            assert!(
                (chunk[0] - 0.25).abs() < 1e-4,
                "expected 0.25, got {}",
                chunk[0]
            );
        }
    }

    #[test]
    fn add_colors() {
        let ctx = match make_ctx() {
            Some(c) => c,
            None => return,
        };

        // 0.3 + 0.2 = 0.5
        let a = solid(Arc::clone(&ctx), 0.3, 0.0, 0.0, 1.0);
        let b = solid(Arc::clone(&ctx), 0.2, 0.0, 0.0, 1.0);

        let params = CompositeParams {
            operation: CompOp::Add,
            mix: 1.0,
        };
        let out = CompositeCop
            .execute(&ctx, &[&a, &b], &params)
            .expect("execute failed");
        let pixels = out.to_cpu().expect("readback failed");

        for chunk in pixels.chunks_exact(4) {
            assert!(
                (chunk[0] - 0.5).abs() < 1e-4,
                "expected 0.5, got {}",
                chunk[0]
            );
        }
    }

    #[test]
    fn wrong_input_count_errors() {
        let ctx = match make_ctx() {
            Some(c) => c,
            None => return,
        };

        let a = solid(Arc::clone(&ctx), 1.0, 0.0, 0.0, 1.0);
        let params = CompositeParams::default();

        // Too few inputs
        let result = CompositeCop.execute(&ctx, &[&a], &params);
        assert!(result.is_err(), "expected error for 1 input");

        // Too many inputs
        let b = solid(Arc::clone(&ctx), 0.0, 1.0, 0.0, 1.0);
        let c = solid(Arc::clone(&ctx), 0.0, 0.0, 1.0, 1.0);
        let result = CompositeCop.execute(&ctx, &[&a, &b, &c], &params);
        assert!(result.is_err(), "expected error for 3 inputs");
    }
}
