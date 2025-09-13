//! Registry pattern for dynamic integrator management
//!
//! The registry serves as a discovery and factory mechanism for integrators.
//! Each integrator is self-describing, providing its own name, aliases, and
//! convergence order. The registry queries this metadata during initialization
//! to build lookup tables for name resolution and instantiation.
//!
//! The registry stores integrator instances indexed by name. Since all integrators
//! are zero-sized types (ZSTs), cloning simply creates new Box allocations
//! without any state copying.

use super::Integrator;
use bevy::prelude::*;
use std::collections::HashMap;

/// Registry for runtime integrator registration
///
/// The registry maintains instances of each integrator indexed by name. When an integrator
/// is requested, the registry creates a new boxed instance via `clone_box()`. Since all
/// integrators are stateless ZSTs, this is essentially just creating a new Box allocation.
#[derive(Resource)]
pub struct IntegratorRegistry {
    /// Maps names (canonical and aliases) to integrator instances
    integrators: HashMap<String, Box<dyn Integrator>>,
}

impl IntegratorRegistry {
    /// Create an empty registry without any pre-registered integrators.
    pub fn new() -> Self {
        Self {
            integrators: HashMap::new(),
        }
    }

    /// Register all standard integrators.
    ///
    /// This populates the registry with all the built-in integrators
    /// that ship with the application.
    /// Returns self for method chaining.
    pub fn with_standard_integrators(mut self) -> Self {
        use super::{
            ExplicitEuler, Heun, Pefrl, RungeKuttaFourthOrder, RungeKuttaSecondOrderMidpoint,
            SymplecticEuler, VelocityVerlet,
        };

        self.register_integrator(Box::new(ExplicitEuler));
        self.register_integrator(Box::new(SymplecticEuler));
        self.register_integrator(Box::new(VelocityVerlet));
        self.register_integrator(Box::new(Heun));
        self.register_integrator(Box::new(RungeKuttaSecondOrderMidpoint));
        self.register_integrator(Box::new(RungeKuttaFourthOrder));
        self.register_integrator(Box::new(Pefrl));

        self
    }

    /// Register a single integrator.
    ///
    /// Returns self for method chaining.
    pub fn with_integrator(mut self, integrator: Box<dyn Integrator>) -> Self {
        self.register_integrator(integrator);
        self
    }

    pub fn register_integrator(&mut self, integrator: Box<dyn Integrator>) {
        let name = integrator.name();

        // Store the integrator with its canonical name
        self.integrators
            .insert(name.to_string(), integrator.clone_box());

        // Also store integrators for each alias
        for alias in integrator.aliases() {
            self.integrators
                .insert(alias.to_string(), integrator.clone_box());
        }
    }

    pub fn create(&self, name: &str) -> Result<Box<dyn Integrator>, String> {
        self.integrators
            .get(name)
            .map(|integrator| integrator.clone_box())
            .ok_or_else(|| {
                let available = self.list_available();
                let aliases = self.list_aliases();
                let alias_names: Vec<String> = aliases.iter().map(|(a, _)| a.clone()).collect();
                format!(
                    "Unknown integrator: '{}'. Available integrators: {}. Aliases: {}",
                    name,
                    available.join(", "),
                    alias_names.join(", ")
                )
            })
    }

    pub fn list_available(&self) -> Vec<String> {
        let mut canonical_names = std::collections::HashSet::new();

        // Get unique canonical names by querying each integrator
        for integrator in self.integrators.values() {
            canonical_names.insert(integrator.name().to_string());
        }

        let mut names: Vec<String> = canonical_names.into_iter().collect();
        names.sort();
        names
    }

    pub fn list_aliases(&self) -> Vec<(String, String)> {
        let mut aliases: Vec<(String, String)> = Vec::new();

        // Check each entry to see if it's an alias
        for (key, integrator) in &self.integrators {
            let canonical_name = integrator.name();
            if key != canonical_name {
                // It's an alias, not a canonical name
                aliases.push((key.clone(), canonical_name.to_string()));
            }
        }

        aliases.sort_by(|a, b| a.0.cmp(&b.0));
        aliases
    }
}

impl Default for IntegratorRegistry {
    fn default() -> Self {
        Self::new().with_standard_integrators()
    }
}
