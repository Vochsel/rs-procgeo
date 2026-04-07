// BlurCop — separable two-pass Gaussian or Box blur

use std::sync::Arc;

use crate::context::GpuContext;
use crate::image::Image;
use crate::{Cop, CopError, dispatch_2d, hash_name};

/// Blur kernel type.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum BlurType {
    #[default]
    Gaussian,
    Box,
}

/// Parameters for the Blur COP.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct BlurParams {
    /// Blur kernel type.
    pub blur_type: BlurType,
    /// Horizontal blur radius in pixels.
    pub radius_x: f32,
    /// Vertical blur radius in pixels.
    pub radius_y: f32,
}

impl Default for BlurParams {
    fn default() -> Self {
        Self {
            blur_type: BlurType::Gaussian,
            radius_x: 4.0,
            radius_y: 4.0,
        }
    }
}

/// Uniform buffer layout for the blur shaders (16 bytes).
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct BlurUniform {
    radius: i32,
    is_gaussian: u32,
    _pad: [u32; 2],
}

/// Filter COP that applies a separable Gaussian or Box blur.
pub struct BlurCop;

const SHADER_H: &str = include_str!("../../shaders/blur_h.wgsl");
const SHADER_V: &str = include_str!("../../shaders/blur_v.wgsl");

fn run_pass(
    ctx: &Arc<GpuContext>,
    input: &Image,
    output: &Image,
    radius: i32,
    is_gaussian: bool,
    pipeline_key: u64,
    shader_source: &str,
    label: &str,
) -> Result<(), CopError> {
    let uniform = BlurUniform {
        radius,
        is_gaussian: is_gaussian as u32,
        _pad: [0; 2],
    };

    let uniform_buffer = ctx.device().create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: std::mem::size_of::<BlurUniform>() as u64,
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

    let pipeline = ctx.get_or_create_pipeline(pipeline_key, shader_source, "main")?;

    let bind_group_layout = pipeline.get_bind_group_layout(0);
    let bind_group = ctx.device().create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
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
                label: Some(label),
            });

    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some(label),
            timestamp_writes: None,
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        dispatch_2d(&mut pass, output.width(), output.height());
    }

    ctx.queue().submit(std::iter::once(encoder.finish()));

    Ok(())
}

impl Cop for BlurCop {
    type Params = BlurParams;

    fn name(&self) -> &'static str {
        "blur"
    }

    fn input_count(&self) -> (usize, usize) {
        (1, 1)
    }

    fn execute(&self, inputs: &[&Image], params: &BlurParams) -> Result<Image, CopError> {
        self.validate_inputs(inputs)?;

        let ctx: Arc<GpuContext> = Arc::clone(inputs[0].ctx());
        let input = inputs[0];

        let width = input.width();
        let height = input.height();

        let is_gaussian = matches!(params.blur_type, BlurType::Gaussian);
        let radius_x = params.radius_x.max(0.0) as i32;
        let radius_y = params.radius_y.max(0.0) as i32;

        // Pass 1: horizontal blur — input → temp
        let temp = Image::create_storage(Arc::clone(&ctx), width, height);
        run_pass(
            &ctx,
            input,
            &temp,
            radius_x,
            is_gaussian,
            hash_name("blur_h"),
            SHADER_H,
            "blur_h",
        )?;

        // Pass 2: vertical blur — temp → output
        let output = Image::create_storage(Arc::clone(&ctx), width, height);
        run_pass(
            &ctx,
            &temp,
            &output,
            radius_y,
            is_gaussian,
            hash_name("blur_v"),
            SHADER_V,
            "blur_v",
        )?;

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blur_constant_stays_constant() {
        let ctx = match GpuContext::new_blocking() {
            Ok(c) => Arc::new(c),
            Err(e) => {
                eprintln!("Skipping blur_constant_stays_constant test (no GPU): {e}");
                return;
            }
        };

        // Constant red image
        let data: Vec<f32> = (0..8 * 8)
            .flat_map(|_| [0.5f32, 0.3, 0.1, 1.0])
            .collect();
        let input = Image::from_cpu(Arc::clone(&ctx), 8, 8, &data).expect("from_cpu failed");

        let params = BlurParams {
            blur_type: BlurType::Gaussian,
            radius_x: 3.0,
            radius_y: 3.0,
        };

        let output = BlurCop.execute(&[&input], &params).expect("execute failed");
        let pixels = output.to_cpu().expect("readback failed");

        // Blurring a constant image should stay constant
        for (i, chunk) in pixels.chunks_exact(4).enumerate() {
            assert!(
                (chunk[0] - 0.5).abs() < 1e-3,
                "pixel {i} R mismatch: got {}",
                chunk[0]
            );
            assert!(
                (chunk[1] - 0.3).abs() < 1e-3,
                "pixel {i} G mismatch: got {}",
                chunk[1]
            );
            assert!(
                (chunk[2] - 0.1).abs() < 1e-3,
                "pixel {i} B mismatch: got {}",
                chunk[2]
            );
        }
    }

    #[test]
    fn blur_sharp_edge_reduces_contrast() {
        let ctx = match GpuContext::new_blocking() {
            Ok(c) => Arc::new(c),
            Err(e) => {
                eprintln!("Skipping blur_sharp_edge_reduces_contrast test (no GPU): {e}");
                return;
            }
        };

        // 4x1 image: left half white, right half black
        let data: Vec<f32> = vec![
            1.0, 1.0, 1.0, 1.0, // pixel 0: white
            1.0, 1.0, 1.0, 1.0, // pixel 1: white
            0.0, 0.0, 0.0, 1.0, // pixel 2: black
            0.0, 0.0, 0.0, 1.0, // pixel 3: black
        ];
        let input = Image::from_cpu(Arc::clone(&ctx), 4, 1, &data).expect("from_cpu failed");

        let params = BlurParams {
            blur_type: BlurType::Box,
            radius_x: 1.0,
            radius_y: 0.0,
        };

        let output = BlurCop.execute(&[&input], &params).expect("execute failed");
        let pixels = output.to_cpu().expect("readback failed");

        // Boundary pixels should be blended — neither pure white nor pure black
        let boundary_r = pixels[4]; // pixel 1 red (was 1.0)
        assert!(
            boundary_r < 1.0,
            "boundary pixel should be blurred below 1.0, got {boundary_r}"
        );
    }
}
