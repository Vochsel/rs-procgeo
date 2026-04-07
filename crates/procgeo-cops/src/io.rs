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
#[cfg(feature = "gpu")]
pub fn save_image(
    _image: &crate::image::Image,
    _params: &SaveImageParams,
) -> Result<(), crate::CopError> {
    todo!("save_image not yet implemented")
}
