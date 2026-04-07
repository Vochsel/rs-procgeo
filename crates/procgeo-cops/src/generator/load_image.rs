// LoadImageCop — loads an image from disk and uploads it to the GPU

use std::sync::Arc;

use crate::context::GpuContext;
use crate::image::Image;
use crate::{Cop, CopError};

/// Parameters for the Load Image COP.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct LoadImageParams {
    /// Path to the image file (PNG, JPEG, HDR, EXR, etc.).
    pub path: String,
}

impl Default for LoadImageParams {
    fn default() -> Self {
        Self {
            path: String::new(),
        }
    }
}

/// Generator COP that loads an image from disk and uploads it to the GPU.
///
/// Supports all formats enabled in the `image` crate workspace dependency
/// (PNG, JPEG, HDR, EXR).
pub struct LoadImageCop;

impl Cop for LoadImageCop {
    type Params = LoadImageParams;

    fn name(&self) -> &'static str {
        "load_image"
    }

    fn input_count(&self) -> (usize, usize) {
        (0, 0)
    }

    fn execute(
        &self,
        ctx: &Arc<GpuContext>,
        inputs: &[&Image],
        params: &LoadImageParams,
    ) -> Result<Image, CopError> {
        self.validate_inputs(inputs)?;

        if params.path.is_empty() {
            return Err(CopError::ImageLoad("path is empty".into()));
        }

        // Decode the image from disk and convert to RGBA f32
        let dyn_image = image::open(&params.path)
            .map_err(|e| CopError::ImageLoad(format!("failed to open '{}': {e}", params.path)))?;

        let rgba_f32 = dyn_image.to_rgba32f();
        let width = rgba_f32.width();
        let height = rgba_f32.height();
        let flat: &[f32] = &rgba_f32;

        Image::from_cpu(Arc::clone(ctx), width, height, flat)
            .map_err(|e| CopError::ImageLoad(format!("GPU upload failed: {e}")))
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
                eprintln!("Skipping load_image test (no GPU): {e}");
                None
            }
        }
    }

    #[test]
    fn empty_path_errors() {
        let Some(ctx) = try_ctx() else { return; };

        let params = LoadImageParams { path: String::new() };
        let result = generate_cop(&ctx, &LoadImageCop, &params);
        assert!(
            result.is_err(),
            "expected error for empty path, got Ok"
        );
        if let Err(e) = result {
            assert!(
                matches!(e, CopError::ImageLoad(_)),
                "expected ImageLoad error, got: {e:?}"
            );
        }
    }

    #[test]
    fn missing_file_errors() {
        let Some(ctx) = try_ctx() else { return; };

        let params = LoadImageParams {
            path: "/nonexistent/path/to/image.png".into(),
        };
        let result = generate_cop(&ctx, &LoadImageCop, &params);
        assert!(
            result.is_err(),
            "expected error for missing file, got Ok"
        );
        if let Err(e) = result {
            assert!(
                matches!(e, CopError::ImageLoad(_)),
                "expected ImageLoad error, got: {e:?}"
            );
        }
    }
}
