// Image — GPU-backed RGBA32Float texture with CPU upload/readback

use std::sync::Arc;

use wgpu;

use crate::context::GpuContext;
use crate::CopError;

/// A GPU-backed RGBA32Float image.
pub struct Image {
    ctx: Arc<GpuContext>,
    texture: wgpu::Texture,
    width: u32,
    height: u32,
}

/// Texture usage flags for storage images (read/write from compute shaders).
const STORAGE_USAGE: wgpu::TextureUsages = wgpu::TextureUsages::STORAGE_BINDING
    .union(wgpu::TextureUsages::TEXTURE_BINDING)
    .union(wgpu::TextureUsages::COPY_SRC)
    .union(wgpu::TextureUsages::COPY_DST);

/// The texture format used for all COP images.
pub const IMAGE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba32Float;

/// Bytes per pixel for Rgba32Float (4 channels * 4 bytes each).
const BYTES_PER_PIXEL: u32 = 16;

/// wgpu requires buffer row alignment of 256 bytes.
const COPY_BYTES_PER_ROW_ALIGNMENT: u32 = 256;

impl Image {
    /// Create an empty 0x0 image that carries a context reference.
    /// Internally allocates a 1x1 texture since wgpu requires non-zero dimensions.
    pub fn empty(ctx: Arc<GpuContext>) -> Self {
        let texture = ctx.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("empty"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: IMAGE_FORMAT,
            usage: STORAGE_USAGE,
            view_formats: &[],
        });

        Self {
            ctx,
            texture,
            width: 0,
            height: 0,
        }
    }

    /// Create a GPU storage texture of the given dimensions.
    pub fn create_storage(ctx: Arc<GpuContext>, width: u32, height: u32) -> Self {
        let texture = ctx.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("storage"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: IMAGE_FORMAT,
            usage: STORAGE_USAGE,
            view_formats: &[],
        });

        Self {
            ctx,
            texture,
            width,
            height,
        }
    }

    /// Upload CPU f32 pixel data to a new GPU texture.
    ///
    /// `data` must contain exactly `width * height * 4` floats (RGBA per pixel).
    pub fn from_cpu(
        ctx: Arc<GpuContext>,
        width: u32,
        height: u32,
        data: &[f32],
    ) -> Result<Self, CopError> {
        let expected = (width as usize) * (height as usize) * 4;
        if data.len() != expected {
            return Err(CopError::InvalidParam(format!(
                "expected {expected} floats for {width}x{height} RGBA, got {}",
                data.len()
            )));
        }

        let img = Self::create_storage(ctx, width, height);

        let bytes: &[u8] = bytemuck::cast_slice(data);
        let bytes_per_row = width * BYTES_PER_PIXEL;

        img.ctx.queue().write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &img.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        Ok(img)
    }

    /// Read GPU texture data back to CPU, returning RGBA f32 values.
    ///
    /// Handles row padding (256-byte alignment) that wgpu requires for buffer copies.
    pub fn to_cpu(&self) -> Result<Vec<f32>, CopError> {
        if self.width == 0 || self.height == 0 {
            return Ok(Vec::new());
        }

        let unpadded_bytes_per_row = self.width * BYTES_PER_PIXEL;
        let padded_bytes_per_row = align_up(unpadded_bytes_per_row, COPY_BYTES_PER_ROW_ALIGNMENT);
        let buffer_size = (padded_bytes_per_row as u64) * (self.height as u64);

        let staging = self
            .ctx
            .device()
            .create_buffer(&wgpu::BufferDescriptor {
                label: Some("readback"),
                size: buffer_size,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });

        let mut encoder = self
            .ctx
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("readback"),
            });

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &staging,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );

        self.ctx.queue().submit(std::iter::once(encoder.finish()));

        // Map the staging buffer for reading
        let buffer_slice = staging.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
        self.ctx.device().poll(wgpu::Maintain::Wait);
        rx.recv()
            .map_err(|e| CopError::Gpu(format!("buffer map channel error: {e}")))?
            .map_err(|e| CopError::Gpu(format!("buffer map failed: {e}")))?;

        let mapped = buffer_slice.get_mapped_range();

        // Copy data, stripping row padding
        let pixel_count = (self.width as usize) * (self.height as usize);
        let mut result = Vec::with_capacity(pixel_count * 4);

        for row in 0..self.height as usize {
            let row_start = row * padded_bytes_per_row as usize;
            let row_end = row_start + unpadded_bytes_per_row as usize;
            let row_bytes = &mapped[row_start..row_end];
            let row_floats: &[f32] = bytemuck::cast_slice(row_bytes);
            result.extend_from_slice(row_floats);
        }

        drop(mapped);
        staging.unmap();

        Ok(result)
    }

    /// Width of the image in pixels (0 for empty images).
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Height of the image in pixels (0 for empty images).
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Reference to the GPU context that owns this image.
    pub fn ctx(&self) -> &Arc<GpuContext> {
        &self.ctx
    }

    /// Reference to the underlying wgpu texture.
    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }
}

/// Align `value` up to the next multiple of `alignment`.
fn align_up(value: u32, alignment: u32) -> u32 {
    (value + alignment - 1) / alignment * alignment
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_image() {
        let ctx = match GpuContext::new_blocking() {
            Ok(ctx) => Arc::new(ctx),
            Err(e) => {
                eprintln!("Skipping empty image test: {e}");
                return;
            }
        };

        let img = Image::empty(ctx);
        assert_eq!(img.width(), 0);
        assert_eq!(img.height(), 0);

        let data = img.to_cpu().unwrap();
        assert!(data.is_empty());
    }

    #[test]
    fn test_roundtrip_from_cpu_to_cpu() {
        let ctx = match GpuContext::new_blocking() {
            Ok(ctx) => Arc::new(ctx),
            Err(e) => {
                eprintln!("Skipping roundtrip test: {e}");
                return;
            }
        };

        let width = 4;
        let height = 3;
        // Create test data: each pixel has unique RGBA values
        let mut data = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            for x in 0..width {
                let r = x as f32 / width as f32;
                let g = y as f32 / height as f32;
                let b = 0.5;
                let a = 1.0;
                data.extend_from_slice(&[r, g, b, a]);
            }
        }

        let img = Image::from_cpu(ctx, width, height, &data).unwrap();
        assert_eq!(img.width(), width);
        assert_eq!(img.height(), height);

        let readback = img.to_cpu().unwrap();
        assert_eq!(readback.len(), data.len());

        for (i, (expected, actual)) in data.iter().zip(readback.iter()).enumerate() {
            assert!(
                (expected - actual).abs() < 1e-5,
                "pixel mismatch at index {i}: expected {expected}, got {actual}"
            );
        }
    }

    #[test]
    fn test_from_cpu_wrong_size() {
        let ctx = match GpuContext::new_blocking() {
            Ok(ctx) => Arc::new(ctx),
            Err(e) => {
                eprintln!("Skipping wrong size test: {e}");
                return;
            }
        };

        let result = Image::from_cpu(ctx, 2, 2, &[0.0; 8]); // needs 16 floats
        assert!(result.is_err());
    }
}
