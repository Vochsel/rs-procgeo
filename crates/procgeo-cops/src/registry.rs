// COP Registry — dynamic dispatch for compositing operators

/// Registry of all available COPs, keyed by name.
pub struct CopRegistry {
    names: Vec<&'static str>,
}

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

impl Default for CopRegistry {
    fn default() -> Self {
        default_cop_registry()
    }
}

/// Build a registry with all available COPs.
pub fn default_cop_registry() -> CopRegistry {
    CopRegistry::new()
}
