use std::collections::HashMap;

use serde::de::DeserializeOwned;
use serde::Serialize;

use procgeo_core::Geometry;

use crate::SopError;

/// A type-erased SOP executor that takes JSON params.
pub trait DynSop: Send + Sync {
    fn execute_json(
        &self,
        inputs: &[&Geometry],
        params_json: &str,
    ) -> Result<Geometry, SopError>;

    fn name(&self) -> &'static str;
    fn input_count(&self) -> (usize, usize);
}

/// Generic wrapper that bridges any `Sop + Params` (with Deserialize) to `DynSop`.
struct SopWrapper<S, P> {
    sop: S,
    _params: std::marker::PhantomData<P>,
}

impl<S, P> DynSop for SopWrapper<S, P>
where
    S: crate::Sop<Params = P> + Send + Sync + 'static,
    P: Default + Serialize + DeserializeOwned + Send + Sync + 'static,
{
    fn execute_json(
        &self,
        inputs: &[&Geometry],
        params_json: &str,
    ) -> Result<Geometry, SopError> {
        let params: P = if params_json.is_empty() || params_json == "{}" {
            P::default()
        } else {
            // Merge provided JSON on top of serialized defaults so callers
            // can supply partial parameter objects.
            let mut base = serde_json::to_value(P::default())
                .map_err(|e| SopError::Other(format!("failed to serialize defaults: {e}")))?;
            let overrides: serde_json::Value = serde_json::from_str(params_json)
                .map_err(|e| SopError::InvalidParam(format!("{e}")))?;
            if let (Some(base_obj), Some(over_obj)) =
                (base.as_object_mut(), overrides.as_object())
            {
                for (k, v) in over_obj {
                    base_obj.insert(k.clone(), v.clone());
                }
            }
            serde_json::from_value(base)
                .map_err(|e| SopError::InvalidParam(format!("{e}")))?
        };
        self.sop.execute(inputs, &params)
    }

    fn name(&self) -> &'static str {
        self.sop.name()
    }

    fn input_count(&self) -> (usize, usize) {
        self.sop.input_count()
    }
}

/// Registry of all available SOPs, keyed by name.
pub struct SopRegistry {
    sops: HashMap<&'static str, Box<dyn DynSop>>,
}

impl SopRegistry {
    pub fn new() -> Self {
        Self {
            sops: HashMap::new(),
        }
    }

    /// Register a SOP with its params type. The params must impl Serialize + Deserialize
    /// so that partial JSON can be merged with serialized defaults.
    pub fn add<S, P>(&mut self, sop: S)
    where
        S: crate::Sop<Params = P> + Send + Sync + 'static,
        P: Default + Serialize + DeserializeOwned + Send + Sync + 'static,
    {
        let wrapper = SopWrapper {
            sop,
            _params: std::marker::PhantomData,
        };
        self.sops.insert(wrapper.name(), Box::new(wrapper));
    }

    pub fn register(&mut self, sop: Box<dyn DynSop>) {
        let name = sop.name();
        self.sops.insert(name, sop);
    }

    pub fn execute(
        &self,
        name: &str,
        inputs: &[&Geometry],
        params_json: &str,
    ) -> Result<Geometry, SopError> {
        let sop = self
            .sops
            .get(name)
            .ok_or_else(|| SopError::Other(format!("unknown SOP: '{name}'")))?;
        sop.execute_json(inputs, params_json)
    }

    pub fn list(&self) -> Vec<&'static str> {
        let mut names: Vec<_> = self.sops.keys().copied().collect();
        names.sort();
        names
    }

    pub fn has(&self, name: &str) -> bool {
        self.sops.contains_key(name)
    }
}

impl Default for SopRegistry {
    fn default() -> Self {
        default_registry()
    }
}

/// Build a registry with ALL available SOPs.
///
/// This is the ONE place that needs updating when a new SOP is added.
pub fn default_registry() -> SopRegistry {
    let mut r = SopRegistry::new();

    // Creation
    #[cfg(feature = "creation")]
    {
        r.add(crate::creation::BoxSop);
        r.add(crate::creation::GridSop);
        r.add(crate::creation::SphereSop);
        r.add(crate::creation::LineSop);
        r.add(crate::creation::CircleSop);
        r.add(crate::creation::TubeSop);
        r.add(crate::creation::TorusSop);
        r.add(crate::creation::RevolveSop);
        r.add(crate::creation::MetaballSop);
    }

    // Transform
    #[cfg(feature = "transform")]
    {
        r.add(crate::transform::TransformSop);
    }

    // Normals
    #[cfg(feature = "normals")]
    {
        r.add(crate::normals::NormalSop);
    }

    // Merge
    #[cfg(feature = "merge")]
    {
        r.add(crate::merge::MergeSop);
    }

    // Attributes
    #[cfg(feature = "attributes")]
    {
        r.add(crate::attributes::AttribCreateSop);
        r.add(crate::attributes::AttribDeleteSop);
        r.add(crate::attributes::AttribRenameSop);
        r.add(crate::attributes::AttribPromoteSop);
        r.add(crate::attributes::AttribTransferSop);
        r.add(crate::attributes::AttribCopySop);
        r.add(crate::attributes::AttribRandomizeSop);
        r.add(crate::attributes::AttribSortSop);
        r.add(crate::attributes::AttribBlurSop);
        r.add(crate::attributes::AttribFillSop);
        r.add(crate::attributes::AttribNoiseSop);
    }

    // Groups
    #[cfg(feature = "groups_sops")]
    {
        r.add(crate::groups::GroupCreateSop);
        r.add(crate::groups::GroupCombineSop);
    }

    // Delete
    #[cfg(feature = "delete")]
    {
        r.add(crate::delete::BlastSop);
        r.add(crate::delete::DeleteSop);
    }

    // Copy
    #[cfg(feature = "copy")]
    {
        r.add(crate::copy::CopyToPointsSop);
    }

    // Reshape
    #[cfg(feature = "reshape")]
    {
        r.add(crate::reshape::SubdivideSop);
        r.add(crate::reshape::PolyExtrudeSop);
        r.add(crate::reshape::SmoothSop);
        r.add(crate::reshape::ClipSop);
        r.add(crate::reshape::PolyBevelSop);
        r.add(crate::reshape::PolyWireSop);
        r.add(crate::reshape::PolyReduceSop);
        r.add(crate::reshape::PolyFillSop);
    }

    // Scatter
    #[cfg(feature = "scatter")]
    {
        r.add(crate::scatter::ScatterSop);
    }

    // Topology
    #[cfg(feature = "topology")]
    {
        r.add(crate::topology::SortSop);
        r.add(crate::topology::FuseSop);
        r.add(crate::topology::ConnectivitySop);
        r.add(crate::topology::ReverseSop);
        r.add(crate::topology::ResampleSop);
    }

    // Voronoi
    #[cfg(feature = "voronoi")]
    {
        r.add(crate::voronoi::VoronoiFractureSop);
    }

    // Measure
    #[cfg(feature = "measure_sops")]
    {
        r.add(crate::measure::MeasureSop);
    }

    // Color
    #[cfg(feature = "color")]
    {
        r.add(crate::color::ColorSop);
    }

    // Utility
    #[cfg(feature = "utility_sops")]
    {
        r.add(crate::utility::EnumerateSop);
        r.add(crate::utility::NullSop);
    }

    // Deform
    #[cfg(feature = "deform")]
    {
        r.add(crate::deform::BendSop);
        r.add(crate::deform::PointDeformSop);
    }

    // Boolean
    #[cfg(feature = "boolean")]
    {
        r.add(crate::boolean::BooleanSop);
    }

    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_registry_has_box() {
        let reg = default_registry();
        assert!(reg.has("box"));
    }

    #[test]
    fn test_execute_box_via_registry() {
        let reg = default_registry();
        let geo = reg.execute("box", &[], "{}").unwrap();
        assert_eq!(geo.num_points(), 8);
        assert_eq!(geo.num_prims(), 6);
    }

    #[test]
    fn test_execute_box_with_params() {
        let reg = default_registry();
        let geo = reg
            .execute("box", &[], r#"{"size":[2.0,2.0,2.0]}"#)
            .unwrap();
        let bbox = geo.bounding_box();
        assert!((bbox.max.x - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_execute_transform_via_registry() {
        let reg = default_registry();
        let box_geo = reg.execute("box", &[], "{}").unwrap();
        let moved = reg
            .execute(
                "transform",
                &[&box_geo],
                r#"{"translate":[10.0,0.0,0.0]}"#,
            )
            .unwrap();
        let bbox = moved.bounding_box();
        assert!((bbox.center().x - 10.0).abs() < 1e-4);
    }

    #[test]
    fn test_unknown_sop_errors() {
        let reg = default_registry();
        let result = reg.execute("nonexistent_sop", &[], "{}");
        assert!(result.is_err());
    }

    #[test]
    fn test_list_sops() {
        let reg = default_registry();
        let names = reg.list();
        assert!(names.contains(&"box"));
        assert!(names.contains(&"grid"));
        assert!(names.contains(&"transform"));
        assert!(names.len() >= 30); // we have 40+ SOPs
    }

    #[test]
    fn test_execute_with_empty_params() {
        let reg = default_registry();
        let geo = reg.execute("box", &[], "").unwrap();
        assert_eq!(geo.num_points(), 8);
    }

    #[test]
    fn test_execute_with_invalid_json_errors() {
        let reg = default_registry();
        let result = reg.execute("box", &[], "not valid json");
        assert!(result.is_err());
    }
}
