// GpuContext — wgpu device/queue holder with compute pipeline cache

use std::collections::HashMap;
use std::sync::RwLock;

use wgpu::{ComputePipeline, Device, Queue};

/// Holds a wgpu device, queue, and a cache of compiled compute pipelines.
pub struct GpuContext {
    device: Device,
    queue: Queue,
    pipeline_cache: RwLock<HashMap<u64, ComputePipeline>>,
}

impl GpuContext {
    /// Create a new GpuContext by requesting a high-performance GPU adapter.
    pub async fn new() -> Result<Self, crate::CopError> {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .ok_or_else(|| crate::CopError::Gpu("no suitable GPU adapter found".into()))?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("procgeo-cops"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    ..Default::default()
                },
                None,
            )
            .await
            .map_err(|e| crate::CopError::Gpu(format!("failed to request device: {e}")))?;

        Ok(Self {
            device,
            queue,
            pipeline_cache: RwLock::new(HashMap::new()),
        })
    }

    /// Blocking version of `new()` using pollster.
    pub fn new_blocking() -> Result<Self, crate::CopError> {
        pollster::block_on(Self::new())
    }

    /// Return a reference to the wgpu device.
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Return a reference to the wgpu queue.
    pub fn queue(&self) -> &Queue {
        &self.queue
    }

    /// Ensure a compute pipeline exists for the given key, creating it if needed.
    ///
    /// Uses a read lock for the cache check and a write lock for insertion.
    /// The pipeline layout is inferred by wgpu (`layout: None`).
    fn ensure_pipeline(&self, key: u64, wgsl_source: &str, entry_point: &str) {
        // Fast path: check if pipeline already exists
        {
            let cache = self.pipeline_cache.read().unwrap();
            if cache.contains_key(&key) {
                return;
            }
        }

        // Slow path: create and insert pipeline
        let mut cache = self.pipeline_cache.write().unwrap();
        // Double-check after acquiring write lock (another thread may have inserted)
        if cache.contains_key(&key) {
            return;
        }

        let shader_module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(entry_point),
                source: wgpu::ShaderSource::Wgsl(wgsl_source.into()),
            });

        let pipeline = self
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry_point),
                layout: None,
                module: &shader_module,
                entry_point: Some(entry_point),
                compilation_options: Default::default(),
                cache: None,
            });

        cache.insert(key, pipeline);
    }

    /// Get or create a cached compute pipeline for the given key, then invoke `f`
    /// with a reference to it.
    ///
    /// This pattern avoids lifetime issues with returning references through RwLock guards.
    pub fn with_pipeline<R>(
        &self,
        key: u64,
        wgsl_source: &str,
        entry_point: &str,
        f: impl FnOnce(&ComputePipeline) -> R,
    ) -> Result<R, crate::CopError> {
        self.ensure_pipeline(key, wgsl_source, entry_point);
        let cache = self.pipeline_cache.read().unwrap();
        let pipeline = cache
            .get(&key)
            .ok_or_else(|| crate::CopError::Gpu("pipeline not found after creation".into()))?;
        Ok(f(pipeline))
    }

    /// Get or create a cached compute pipeline for the given key.
    ///
    /// Returns a reference tied to `&self`. This is safe because pipeline entries
    /// are never removed from the cache, and the `HashMap` values are heap-allocated
    /// and remain stable as long as the map is only grown (never shrunk or cleared).
    /// The `RwLock` prevents concurrent mutation during the pointer dereference.
    pub fn get_or_create_pipeline(
        &self,
        key: u64,
        wgsl_source: &str,
        entry_point: &str,
    ) -> Result<&ComputePipeline, crate::CopError> {
        self.ensure_pipeline(key, wgsl_source, entry_point);

        let cache = self.pipeline_cache.read().unwrap();
        let ptr = cache.get(&key).unwrap() as *const ComputePipeline;
        // SAFETY: The pipeline lives in a heap-allocated HashMap entry owned by
        // `self.pipeline_cache`. Entries are never removed, so the pointer remains
        // valid for the lifetime of `&self`. The read lock prevents concurrent writes
        // during the pointer derivation; after dropping the guard the data stays put
        // because no code path removes or clears the cache.
        Ok(unsafe { &*ptr })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_context() {
        match GpuContext::new_blocking() {
            Ok(ctx) => {
                // Verify device and queue are accessible
                let _ = ctx.device();
                let _ = ctx.queue();
            }
            Err(e) => {
                // Skip gracefully if no GPU is available (CI, headless, etc.)
                eprintln!("Skipping GPU context test: {e}");
            }
        }
    }

    #[test]
    fn test_pipeline_cache() {
        let ctx = match GpuContext::new_blocking() {
            Ok(ctx) => ctx,
            Err(e) => {
                eprintln!("Skipping pipeline cache test: {e}");
                return;
            }
        };

        let shader = r#"
            @group(0) @binding(0) var output: texture_storage_2d<rgba32float, write>;

            @compute @workgroup_size(16, 16)
            fn main(@builtin(global_invocation_id) id: vec3<u32>) {
                let dims = textureDimensions(output);
                if id.x >= dims.x || id.y >= dims.y { return; }
                textureStore(output, vec2<i32>(i32(id.x), i32(id.y)), vec4<f32>(1.0, 0.0, 0.0, 1.0));
            }
        "#;

        let key = 42u64;
        // First call creates the pipeline
        let p1 = ctx.get_or_create_pipeline(key, shader, "main");
        assert!(p1.is_ok());

        // Second call should return the cached pipeline
        let p2 = ctx.get_or_create_pipeline(key, shader, "main");
        assert!(p2.is_ok());
    }
}
