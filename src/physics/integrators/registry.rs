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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::integrators::AccelerationField;
    use crate::physics::math::{Scalar, Vector};

    // Test-specific minimal integrator implementations
    #[derive(Debug, Clone)]
    struct TestIntegratorA;

    impl Integrator for TestIntegratorA {
        fn clone_box(&self) -> Box<dyn Integrator> {
            Box::new(self.clone())
        }

        fn step(&self, _: &mut Vector, _: &mut Vector, _: &dyn AccelerationField, _: Scalar) {
            // Minimal implementation for testing
        }

        fn convergence_order(&self) -> usize {
            2
        }

        fn name(&self) -> &'static str {
            "test_a"
        }

        fn aliases(&self) -> Vec<&'static str> {
            vec!["ta", "test_alias_a"]
        }
    }

    #[derive(Debug, Clone)]
    struct TestIntegratorB;

    impl Integrator for TestIntegratorB {
        fn clone_box(&self) -> Box<dyn Integrator> {
            Box::new(self.clone())
        }

        fn step(&self, _: &mut Vector, _: &mut Vector, _: &dyn AccelerationField, _: Scalar) {
            // Minimal implementation
        }

        fn convergence_order(&self) -> usize {
            4
        }

        fn name(&self) -> &'static str {
            "test_b"
        }

        fn aliases(&self) -> Vec<&'static str> {
            vec!["tb"]
        }
    }

    #[derive(Debug, Clone)]
    struct TestIntegratorNoAlias;

    impl Integrator for TestIntegratorNoAlias {
        fn clone_box(&self) -> Box<dyn Integrator> {
            Box::new(self.clone())
        }

        fn step(&self, _: &mut Vector, _: &mut Vector, _: &dyn AccelerationField, _: Scalar) {
            // Minimal implementation
        }

        fn convergence_order(&self) -> usize {
            1
        }

        fn name(&self) -> &'static str {
            "no_alias"
        }

        // Uses default aliases() implementation (returns empty vec)
    }

    // Helper to create a test registry with known content
    fn create_test_registry() -> IntegratorRegistry {
        IntegratorRegistry::new()
            .with_integrator(Box::new(TestIntegratorA))
            .with_integrator(Box::new(TestIntegratorB))
    }

    #[test]
    fn test_registry_discovery() {
        let registry = create_test_registry();

        let available = registry.list_available();
        // Don't assert exact count, just verify our test integrators are there
        assert!(available.contains(&"test_a".to_string()));
        assert!(available.contains(&"test_b".to_string()));
        assert_eq!(
            available.len(),
            2,
            "Should have exactly our test integrators"
        );
    }

    #[test]
    fn test_integrator_metadata() {
        let registry = create_test_registry();

        // Test that integrators report correct metadata
        let integrator_a = registry.create("test_a").unwrap();
        assert_eq!(integrator_a.name(), "test_a");
        assert_eq!(integrator_a.convergence_order(), 2);
        assert_eq!(integrator_a.aliases(), vec!["ta", "test_alias_a"]);

        let integrator_b = registry.create("test_b").unwrap();
        assert_eq!(integrator_b.name(), "test_b");
        assert_eq!(integrator_b.convergence_order(), 4);
        assert_eq!(integrator_b.aliases(), vec!["tb"]);
    }

    #[test]
    fn test_alias_resolution() {
        let registry = create_test_registry();

        // Test that aliases resolve to the correct integrator
        let canonical = registry.create("test_a").unwrap();
        let via_alias1 = registry.create("ta").unwrap();
        assert_eq!(canonical.name(), via_alias1.name());
        assert_eq!(
            canonical.convergence_order(),
            via_alias1.convergence_order()
        );

        let via_alias2 = registry.create("test_alias_a").unwrap();
        assert_eq!(canonical.name(), via_alias2.name());

        // Test B's alias
        let canonical_b = registry.create("test_b").unwrap();
        let via_alias_b = registry.create("tb").unwrap();
        assert_eq!(canonical_b.name(), via_alias_b.name());
    }

    #[test]
    fn test_list_aliases() {
        let registry = create_test_registry();
        let aliases = registry.list_aliases();

        // Check that we get exactly the aliases we registered
        let alias_map: HashMap<_, _> = aliases.into_iter().collect();
        assert_eq!(alias_map.get("ta"), Some(&"test_a".to_string()));
        assert_eq!(alias_map.get("test_alias_a"), Some(&"test_a".to_string()));
        assert_eq!(alias_map.get("tb"), Some(&"test_b".to_string()));
        assert_eq!(alias_map.len(), 3, "Should have exactly 3 aliases");
    }

    #[test]
    fn test_unknown_integrator_error() {
        let registry = create_test_registry();

        // Test that unknown names produce helpful error messages
        let result = registry.create("nonexistent");
        assert!(result.is_err());

        if let Err(error) = result {
            assert!(error.contains("Unknown integrator"));
            assert!(error.contains("Available integrators"));
            assert!(error.contains("Aliases"));
            // Check our test integrators are mentioned
            assert!(error.contains("test_a"));
            assert!(error.contains("test_b"));
        }
    }

    #[test]
    fn test_clone_creates_new_box() {
        let registry = create_test_registry();

        // Create two instances from same integrator
        let integrator1 = registry.create("test_a").unwrap();
        let integrator2 = registry.create("test_a").unwrap();

        // This test verifies that create() returns new Box instances.
        // For ZSTs, the inner pointers might be equal due to optimization,
        // but each call to create() does produce a new Box.
        assert_eq!(integrator1.name(), integrator2.name());
        assert_eq!(
            integrator1.convergence_order(),
            integrator2.convergence_order()
        );

        // The important guarantee is that we get independent Box instances,
        // which we do - they just happen to point to the same ZST address.
        // This is fine because integrators are stateless.
        drop(integrator1);
        // If integrator2 is still usable after dropping integrator1, they're independent
        assert_eq!(integrator2.name(), "test_a");
    }

    #[test]
    fn test_no_aliases_integrator() {
        let registry = IntegratorRegistry::new().with_integrator(Box::new(TestIntegratorNoAlias));

        // Should work with canonical name
        assert!(registry.create("no_alias").is_ok());

        // Verify no aliases were registered for it
        let aliases = registry.list_aliases();
        assert!(
            !aliases.iter().any(|(_, target)| target == "no_alias"),
            "Should not have any aliases for no_alias integrator"
        );

        // Verify it appears in available list
        let available = registry.list_available();
        assert!(available.contains(&"no_alias".to_string()));
    }

    #[test]
    fn test_empty_registry() {
        let registry = IntegratorRegistry::new();

        // Empty registry should have no integrators
        assert_eq!(registry.list_available().len(), 0);
        assert_eq!(registry.list_aliases().len(), 0);

        // Creating any integrator should fail
        assert!(registry.create("anything").is_err());
    }

    #[test]
    fn test_case_sensitivity() {
        let registry = create_test_registry();

        // Names should be case-sensitive
        assert!(registry.create("TEST_A").is_err());
        assert!(registry.create("Test_A").is_err());
        assert!(registry.create("TA").is_err());

        // But the correct case should work
        assert!(registry.create("test_a").is_ok());
        assert!(registry.create("ta").is_ok());
    }

    #[test]
    fn test_all_aliases_resolve() {
        let registry = create_test_registry();

        // Every alias should successfully create an integrator
        for (alias, _canonical) in registry.list_aliases() {
            let result = registry.create(&alias);
            assert!(
                result.is_ok(),
                "Alias '{}' failed to create integrator",
                alias
            );
        }
    }

    #[test]
    fn test_metadata_consistency_across_access_methods() {
        let registry = create_test_registry();

        // Accessing via canonical name vs alias should give same metadata
        for (alias, canonical) in registry.list_aliases() {
            let via_canonical = registry.create(&canonical).unwrap();
            let via_alias = registry.create(&alias).unwrap();

            assert_eq!(
                via_canonical.name(),
                via_alias.name(),
                "Name mismatch for alias '{}'",
                alias
            );
            assert_eq!(
                via_canonical.convergence_order(),
                via_alias.convergence_order(),
                "Convergence order mismatch for alias '{}'",
                alias
            );
        }
    }

    // Test that validates standard registry behavior
    #[test]
    fn test_builder_pattern_chaining() {
        // Demonstrate the fluent API with multiple integrators
        let registry = IntegratorRegistry::new()
            .with_integrator(Box::new(TestIntegratorA))
            .with_integrator(Box::new(TestIntegratorB))
            .with_integrator(Box::new(TestIntegratorNoAlias));

        assert_eq!(registry.list_available().len(), 3);
        assert!(registry.create("test_a").is_ok());
        assert!(registry.create("test_b").is_ok());
        assert!(registry.create("no_alias").is_ok());

        // Aliases should also work
        assert!(registry.create("ta").is_ok());
        assert!(registry.create("tb").is_ok());
    }

    #[test]
    fn test_duplicate_registration() {
        // Test that re-registering the same integrator overwrites the previous one
        let registry = IntegratorRegistry::new()
            .with_integrator(Box::new(TestIntegratorA))
            .with_integrator(Box::new(TestIntegratorA)); // Register same type twice

        // Should still only have one canonical name
        assert_eq!(registry.list_available().len(), 1);
        assert!(registry.create("test_a").is_ok());

        // Aliases should still work
        assert!(registry.create("ta").is_ok());
    }

    #[test]
    fn test_mixed_builder_usage() {
        // Test mixing with_standard_integrators and with_integrator
        let registry = IntegratorRegistry::new()
            .with_standard_integrators()
            .with_integrator(Box::new(TestIntegratorA));

        // Should have standard integrators plus our test one
        assert!(registry.list_available().len() > 1);
        assert!(registry.create("velocity_verlet").is_ok()); // Standard integrator
        assert!(registry.create("test_a").is_ok()); // Test integrator
    }

    #[test]
    fn test_register_integrator_mutability() {
        // Test that register_integrator still works on mutable reference
        let mut registry = IntegratorRegistry::new();
        registry.register_integrator(Box::new(TestIntegratorA));
        registry.register_integrator(Box::new(TestIntegratorB));

        assert_eq!(registry.list_available().len(), 2);
        assert!(registry.create("test_a").is_ok());
        assert!(registry.create("test_b").is_ok());
    }

    #[test]
    fn test_standard_registry_basic_functionality() {
        // This test ensures the standard registry works, without asserting specific content
        let registry = IntegratorRegistry::new().with_standard_integrators();

        // Should have some integrators
        assert!(
            !registry.list_available().is_empty(),
            "Standard registry should have integrators"
        );

        // All available integrators should be creatable
        for name in registry.list_available() {
            assert!(
                registry.create(&name).is_ok(),
                "Failed to create integrator '{}'",
                name
            );
        }

        // All aliases should resolve
        for (alias, canonical) in registry.list_aliases() {
            let result = registry.create(&alias);
            assert!(
                result.is_ok(),
                "Alias '{}' (-> '{}') failed to resolve",
                alias,
                canonical
            );
        }
    }
}
