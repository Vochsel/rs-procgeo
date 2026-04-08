// COP Registry — dynamic dispatch for compositing operators

use std::collections::HashMap;
use std::sync::Arc;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::CopError;

// ---------------------------------------------------------------------------
// DynCop — type-erased COP executor
// ---------------------------------------------------------------------------

/// A type-erased COP executor that deserializes params from JSON.
#[cfg(feature = "gpu")]
pub trait DynCop: Send + Sync {
    fn execute_json(
        &self,
        ctx: &Arc<crate::context::GpuContext>,
        inputs: &[&crate::image::Image],
        params_json: &str,
    ) -> Result<crate::image::Image, CopError>;

    fn name(&self) -> &'static str;
    fn input_count(&self) -> (usize, usize);
}

// ---------------------------------------------------------------------------
// CopWrapper — bridges typed Cop + Params to DynCop
// ---------------------------------------------------------------------------

#[cfg(feature = "gpu")]
struct CopWrapper<C, P> {
    cop: C,
    _params: std::marker::PhantomData<P>,
}

#[cfg(feature = "gpu")]
impl<C, P> DynCop for CopWrapper<C, P>
where
    C: crate::Cop<Params = P> + Send + Sync + 'static,
    P: Default + Serialize + DeserializeOwned + Send + Sync + 'static,
{
    fn execute_json(
        &self,
        ctx: &Arc<crate::context::GpuContext>,
        inputs: &[&crate::image::Image],
        params_json: &str,
    ) -> Result<crate::image::Image, CopError> {
        // Deserialize params, merging provided JSON over serialized defaults.
        let params: P = if params_json.is_empty() || params_json == "{}" {
            P::default()
        } else {
            let mut base = serde_json::to_value(P::default())
                .map_err(|e| CopError::Other(format!("failed to serialize defaults: {e}")))?;
            let overrides: serde_json::Value = serde_json::from_str(params_json)
                .map_err(|e| CopError::InvalidParam(format!("{e}")))?;
            if let (Some(base_obj), Some(over_obj)) = (base.as_object_mut(), overrides.as_object())
            {
                for (k, v) in over_obj {
                    base_obj.insert(k.clone(), v.clone());
                }
            }
            serde_json::from_value(base).map_err(|e| CopError::InvalidParam(format!("{e}")))?
        };

        self.cop.execute(ctx, inputs, &params)
    }

    fn name(&self) -> &'static str {
        self.cop.name()
    }

    fn input_count(&self) -> (usize, usize) {
        self.cop.input_count()
    }
}

// ---------------------------------------------------------------------------
// CopRegistry
// ---------------------------------------------------------------------------

/// Registry of all available COPs, keyed by name.
#[cfg(feature = "gpu")]
pub struct CopRegistry {
    cops: HashMap<&'static str, Box<dyn DynCop>>,
}

#[cfg(feature = "gpu")]
impl CopRegistry {
    pub fn new() -> Self {
        Self {
            cops: HashMap::new(),
        }
    }

    /// Register a typed COP. The params type must implement `Serialize + Deserialize`
    /// so that partial JSON can be merged with serialized defaults.
    pub fn add<C, P>(&mut self, cop: C)
    where
        C: crate::Cop<Params = P> + Send + Sync + 'static,
        P: Default + Serialize + DeserializeOwned + Send + Sync + 'static,
    {
        let wrapper = CopWrapper {
            cop,
            _params: std::marker::PhantomData,
        };
        self.cops.insert(wrapper.name(), Box::new(wrapper));
    }

    /// Register a pre-boxed `DynCop`.
    pub fn register(&mut self, cop: Box<dyn DynCop>) {
        let name = cop.name();
        self.cops.insert(name, cop);
    }

    /// Execute a COP by name, deserializing params from JSON.
    ///
    /// For generator COPs (no required inputs), pass an empty `inputs` slice;
    /// the registry will inject the GPU context automatically.
    pub fn execute(
        &self,
        name: &str,
        ctx: &Arc<crate::context::GpuContext>,
        inputs: &[&crate::image::Image],
        params_json: &str,
    ) -> Result<crate::image::Image, CopError> {
        let cop = self
            .cops
            .get(name)
            .ok_or_else(|| CopError::Other(format!("unknown COP: '{name}'")))?;
        cop.execute_json(ctx, inputs, params_json)
    }

    /// Return a sorted list of all registered COP names.
    pub fn list(&self) -> Vec<&'static str> {
        let mut names: Vec<_> = self.cops.keys().copied().collect();
        names.sort();
        names
    }

    pub fn has(&self, name: &str) -> bool {
        self.cops.contains_key(name)
    }
}

#[cfg(feature = "gpu")]
impl Default for CopRegistry {
    fn default() -> Self {
        default_cop_registry()
    }
}

// ---------------------------------------------------------------------------
// Stub CopRegistry for non-GPU builds
// ---------------------------------------------------------------------------

/// Registry of all available COPs, keyed by name (CPU stub for non-GPU builds).
#[cfg(not(feature = "gpu"))]
pub struct CopRegistry {
    names: Vec<&'static str>,
}

#[cfg(not(feature = "gpu"))]
impl CopRegistry {
    pub fn new() -> Self {
        Self { names: Vec::new() }
    }

    pub fn list(&self) -> &[&'static str] {
        &self.names
    }

    pub fn has(&self, name: &str) -> bool {
        self.names.iter().any(|n| *n == name)
    }
}

#[cfg(not(feature = "gpu"))]
impl Default for CopRegistry {
    fn default() -> Self {
        default_cop_registry()
    }
}

// ---------------------------------------------------------------------------
// default_cop_registry — registers ALL COPs
// ---------------------------------------------------------------------------

/// Build a registry with all available COPs.
///
/// This is the ONE place that needs updating when a new COP is added.
#[cfg(feature = "gpu")]
pub fn default_cop_registry() -> CopRegistry {
    let mut r = CopRegistry::new();

    // Generator COPs
    #[cfg(feature = "generator")]
    {
        r.add(crate::generator::ConstantCop);
        r.add(crate::generator::CheckerboardCop);
        r.add(crate::generator::NoiseCop);
        r.add(crate::generator::RampCop);
        r.add(crate::generator::LoadImageCop);
    }

    // Filter COPs
    #[cfg(feature = "filter")]
    {
        r.add(crate::filter::FlipCop);
        r.add(crate::filter::MirrorCop);
        r.add(crate::filter::ChannelSwapCop);
        r.add(crate::filter::BlurCop);
        r.add(crate::filter::SwirlCop);
        r.add(crate::filter::RotateCop);
        r.add(crate::filter::ResizeCop);
    }

    // Composite COPs
    #[cfg(feature = "composite")]
    {
        r.add(crate::composite::CompositeCop);
    }

    // Custom COPs
    #[cfg(feature = "custom")]
    {
        r.add(crate::custom::CustomShaderCop);
    }

    r
}

#[cfg(not(feature = "gpu"))]
pub fn default_cop_registry() -> CopRegistry {
    CopRegistry::new()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(all(feature = "gpu", test))]
mod tests {
    use super::*;
    use crate::context::GpuContext;

    fn make_ctx() -> Option<Arc<crate::context::GpuContext>> {
        match GpuContext::new_blocking() {
            Ok(c) => Some(Arc::new(c)),
            Err(e) => {
                eprintln!("Skipping test (no GPU): {e}");
                None
            }
        }
    }

    #[test]
    fn registry_has_constant() {
        let reg = default_cop_registry();
        assert!(reg.has("constant"), "expected 'constant' in registry");
    }

    #[test]
    fn list_cops_returns_expected_names() {
        let reg = default_cop_registry();
        let names = reg.list();
        let expected = [
            "constant",
            "checkerboard",
            "noise",
            "ramp",
            "load_image",
            "flip",
            "mirror",
            "channel_swap",
            "blur",
            "swirl",
            "rotate",
            "resize",
            "composite",
            "custom_shader",
        ];
        for &name in &expected {
            assert!(names.contains(&name), "missing COP: '{name}'");
        }
    }

    #[test]
    fn execute_constant_via_registry() {
        let ctx = match make_ctx() {
            Some(c) => c,
            None => return,
        };
        let reg = default_cop_registry();

        let img = reg
            .execute(
                "constant",
                &ctx,
                &[],
                r#"{"color":[1.0,0.0,0.0,1.0],"width":4,"height":4}"#,
            )
            .expect("registry execute failed");

        assert_eq!(img.width(), 4);
        assert_eq!(img.height(), 4);

        let pixels = img.to_cpu().expect("readback failed");
        for chunk in pixels.chunks_exact(4) {
            assert!(
                (chunk[0] - 1.0).abs() < 1e-4,
                "expected red=1.0, got {}",
                chunk[0]
            );
        }
    }

    #[test]
    fn unknown_cop_returns_error() {
        let ctx = match make_ctx() {
            Some(c) => c,
            None => return,
        };
        let reg = default_cop_registry();

        let result = reg.execute("does_not_exist", &ctx, &[], "{}");
        assert!(result.is_err(), "expected error for unknown COP");
    }
}
