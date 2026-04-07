// I/O utilities for saving COP images to disk

use serde::{Deserialize, Serialize};

/// Output image format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ImageFormat {
    #[default]
    Png,
    Jpeg,
    Hdr,
    Exr,
}

/// Output bit depth for integer formats (PNG, JPEG).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BitDepth {
    #[default]
    Eight,
    Sixteen,
}

/// Parameters for saving an image to disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveImageParams {
    /// Output file path.
    pub path: String,
    /// Image format.
    pub format: ImageFormat,
    /// Bit depth (for integer formats).
    pub bit_depth: BitDepth,
}

impl Default for SaveImageParams {
    fn default() -> Self {
        Self {
            path: "output.png".into(),
            format: ImageFormat::Png,
            bit_depth: BitDepth::Eight,
        }
    }
}

/// Save an image to disk.
///
/// Reads the GPU texture back to CPU, converts RGBA32F to RGBA8 by clamping to
/// `[0, 1]` and scaling to `[0, 255]`, then writes the resulting file using the
/// `image` crate.
#[cfg(feature = "gpu")]
pub fn save_image(
    image: &crate::image::Image,
    params: &SaveImageParams,
) -> Result<(), crate::CopError> {
    let floats = image
        .to_cpu()
        .map_err(|e| crate::CopError::Other(format!("readback failed: {e}")))?;

    let w = image.width();
    let h = image.height();

    // Convert RGBA32F → RGBA8
    let rgba8: Vec<u8> = floats
        .iter()
        .map(|&v| (v.clamp(0.0, 1.0) * 255.0).round() as u8)
        .collect();

    let img_buf = ::image::RgbaImage::from_raw(w, h, rgba8).ok_or_else(|| {
        crate::CopError::Other("failed to create RgbaImage from pixel data".into())
    })?;

    img_buf
        .save(&params.path)
        .map_err(|e| crate::CopError::Other(format!("save failed: {e}")))?;

    Ok(())
}

#[cfg(all(feature = "gpu", test))]
mod tests {
    use super::*;
    use crate::context::GpuContext;
    use crate::generate_cop;
    use crate::generator::{ConstantCop, ConstantParams};
    use std::sync::Arc;

    #[test]
    fn save_constant_red_png() {
        let ctx = match GpuContext::new_blocking() {
            Ok(c) => Arc::new(c),
            Err(e) => {
                eprintln!("Skipping save_image test (no GPU): {e}");
                return;
            }
        };

        let params = ConstantParams {
            color: [1.0, 0.0, 0.0, 1.0],
            width: 8,
            height: 8,
        };
        let img = generate_cop(Arc::clone(&ctx), &ConstantCop, &params).expect("generate failed");

        let path = std::env::temp_dir().join("procgeo_cops_test_red.png");
        let save_params = SaveImageParams {
            path: path.to_string_lossy().into_owned(),
            format: ImageFormat::Png,
            bit_depth: BitDepth::Eight,
        };

        save_image(&img, &save_params).expect("save_image failed");

        assert!(path.exists(), "output file not found");
        let meta = std::fs::metadata(&path).unwrap();
        assert!(meta.len() > 0, "output file is empty");

        // Clean up
        let _ = std::fs::remove_file(&path);
    }
}
