// RampCop — generates gradient ramp textures via GPU compute shader

use std::sync::Arc;

use bytemuck::{Pod, Zeroable};

use crate::context::GpuContext;
use crate::image::Image;
use crate::{Cop, CopError, dispatch_2d, hash_name};

/// Ramp gradient type.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RampType {
    /// Horizontal linear gradient (left to right).
    #[default]
    Linear,
    /// Radial gradient from the center outward.
    Radial,
    /// Box gradient — maximum of horizontal/vertical distance from center.
    Box,
    /// Diagonal gradient ((uv.x + uv.y) / 2).
    Diagonal,
}

impl RampType {
    fn as_u32(&self) -> u32 {
        match self {
            Self::Linear => 0,
            Self::Radial => 1,
            Self::Box => 2,
            Self::Diagonal => 3,
        }
    }
}

/// A color stop in the ramp: (position 0–1, RGBA color).
pub type RampStop = (f32, [f32; 4]);

/// Parameters for the Ramp COP.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RampParams {
    /// Gradient type.
    pub ramp_type: RampType,
    /// Color stops sorted by position ascending.
    pub stops: Vec<RampStop>,
    /// Output width in pixels.
    pub width: u32,
    /// Output height in pixels.
    pub height: u32,
}

impl Default for RampParams {
    fn default() -> Self {
        Self {
            ramp_type: RampType::default(),
            stops: vec![(0.0, [0.0, 0.0, 0.0, 1.0]), (1.0, [1.0, 1.0, 1.0, 1.0])],
            width: 256,
            height: 256,
        }
    }
}

/// GPU-side uniform layout (must match ramp.wgsl `Params` struct).
/// Total size: 4 + 4 + 8 = 16 bytes.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct RampUniform {
    ramp_type: u32,
    stop_count: u32,
    _pad: [u32; 2],
}

/// GPU-side stop layout — must be padded to align with WGSL struct layout.
/// position(f32) + _pad0,_pad1,_pad2(f32) + color(vec4f) = 32 bytes.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct GpuStop {
    position: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
    color: [f32; 4],
}

/// Generator COP that produces gradient ramps (Linear, Radial, Box, Diagonal).
pub struct RampCop;

const SHADER_SOURCE: &str = include_str!("../../shaders/ramp.wgsl");

impl Cop for RampCop {
    type Params = RampParams;

    fn name(&self) -> &'static str {
        "ramp"
    }

    fn input_count(&self) -> (usize, usize) {
        (0, 0)
    }

    fn execute(
        &self,
        ctx: &Arc<GpuContext>,
        inputs: &[&Image],
        params: &RampParams,
    ) -> Result<Image, CopError> {
        self.validate_inputs(inputs)?;

        if params.stops.is_empty() {
            return Err(CopError::InvalidParam(
                "ramp must have at least one stop".into(),
            ));
        }

        let output = Image::create_storage(Arc::clone(ctx), params.width, params.height);

        // --- Uniform buffer (ramp_type + stop_count) ---
        let uniform_data = RampUniform {
            ramp_type: params.ramp_type.as_u32(),
            stop_count: params.stops.len() as u32,
            _pad: [0; 2],
        };

        let uniform_buffer = ctx.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("ramp_params"),
            size: std::mem::size_of::<RampUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        ctx.queue()
            .write_buffer(&uniform_buffer, 0, bytemuck::bytes_of(&uniform_data));

        // --- Storage buffer (stops array) ---
        let gpu_stops: Vec<GpuStop> = params
            .stops
            .iter()
            .map(|(pos, color)| GpuStop {
                position: *pos,
                _pad0: 0.0,
                _pad1: 0.0,
                _pad2: 0.0,
                color: *color,
            })
            .collect();

        let stops_size = (std::mem::size_of::<GpuStop>() * gpu_stops.len()) as u64;
        let stops_buffer = ctx.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("ramp_stops"),
            size: stops_size.max(16), // minimum size for wgpu
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        ctx.queue()
            .write_buffer(&stops_buffer, 0, bytemuck::cast_slice(&gpu_stops));

        let output_view = output
            .texture()
            .create_view(&wgpu::TextureViewDescriptor::default());

        let pipeline_key = hash_name("ramp");
        let pipeline = ctx.get_or_create_pipeline(pipeline_key, SHADER_SOURCE, "main")?;

        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let bind_group = ctx.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ramp_bind_group"),
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
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: stops_buffer.as_entire_binding(),
                },
            ],
        });

        let mut encoder = ctx
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("ramp_dispatch"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("ramp_pass"),
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

    fn try_ctx() -> Option<Arc<GpuContext>> {
        match GpuContext::new_blocking() {
            Ok(c) => Some(Arc::new(c)),
            Err(e) => {
                eprintln!("Skipping ramp test (no GPU): {e}");
                None
            }
        }
    }

    #[test]
    fn linear_ramp_left_dark_right_bright() {
        let Some(ctx) = try_ctx() else {
            return;
        };

        // 8-wide linear ramp: black at 0, white at 1
        let params = RampParams {
            ramp_type: RampType::Linear,
            stops: vec![(0.0, [0.0, 0.0, 0.0, 1.0]), (1.0, [1.0, 1.0, 1.0, 1.0])],
            width: 8,
            height: 4,
        };

        let img = generate_cop(&ctx, &RampCop, &params).expect("execute failed");
        let pixels = img.to_cpu().expect("readback failed");

        // Leftmost pixel should be dark (R close to 0)
        let left_r = pixels[0];
        assert!(left_r < 0.2, "left pixel should be dark, got R={left_r}");

        // Rightmost pixel (x=7) should be bright (R close to 1)
        let right_base = (7) * 4; // pixel at (7, 0)
        let right_r = pixels[right_base];
        assert!(
            right_r > 0.8,
            "right pixel should be bright, got R={right_r}"
        );
    }

    #[test]
    fn radial_ramp_center_is_first_stop() {
        let Some(ctx) = try_ctx() else {
            return;
        };

        // Radial ramp: first stop is red at center, second stop is blue at edge
        let params = RampParams {
            ramp_type: RampType::Radial,
            stops: vec![(0.0, [1.0, 0.0, 0.0, 1.0]), (1.0, [0.0, 0.0, 1.0, 1.0])],
            width: 16,
            height: 16,
        };

        let img = generate_cop(&ctx, &RampCop, &params).expect("execute failed");
        let pixels = img.to_cpu().expect("readback failed");

        // Center pixel (8, 8) should be close to first stop (red)
        let cx = 8usize;
        let cy = 8usize;
        let base = (cy * 16 + cx) * 4;
        let r = pixels[base];
        let b = pixels[base + 2];
        assert!(
            r > b,
            "center pixel should be redder than blue, got R={r} B={b}"
        );
    }

    #[test]
    fn ramp_no_stops_errors() {
        let ctx = match GpuContext::new_blocking() {
            Ok(c) => Arc::new(c),
            Err(e) => {
                eprintln!("Skipping ramp_no_stops_errors test (no GPU): {e}");
                return;
            }
        };

        let params = RampParams {
            stops: vec![],
            width: 4,
            height: 4,
            ..Default::default()
        };

        let result = generate_cop(&ctx, &RampCop, &params);
        assert!(result.is_err(), "expected error with empty stops");
    }
}
