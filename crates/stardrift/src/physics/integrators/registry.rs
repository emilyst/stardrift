//! Registry pattern for dynamic integrator management

use super::{
    Heun, Integrator, Pefrl, RungeKuttaFourthOrder, RungeKuttaSecondOrderMidpoint, SymplecticEuler,
    VelocityVerlet,
};
use bevy::prelude::*;
use std::collections::HashMap;

/// Registry for runtime integrator registration
#[derive(Resource)]
pub struct IntegratorRegistry {
    aliases: HashMap<String, String>,
}

impl IntegratorRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            aliases: HashMap::new(),
        };

        // Short aliases for convenience
        registry.add_alias("euler", "symplectic_euler");
        registry.add_alias("semi_implicit_euler", "symplectic_euler");
        registry.add_alias("verlet", "velocity_verlet");
        registry.add_alias("rk4", "runge_kutta_fourth_order");
        registry.add_alias("rk2", "runge_kutta_second_order_midpoint");
        registry.add_alias("midpoint", "runge_kutta_second_order_midpoint");
        registry.add_alias("improved_euler", "heun");
        registry.add_alias("forest_ruth", "pefrl");

        registry
    }

    pub fn add_alias(&mut self, alias: &str, target: &str) {
        self.aliases.insert(alias.to_string(), target.to_string());
    }

    pub fn create(&self, name: &str) -> Result<Box<dyn Integrator>, String> {
        let resolved_name = self.aliases.get(name).map(|s| s.as_str()).unwrap_or(name);

        // Simple match statement instead of factory pattern
        match resolved_name {
            "symplectic_euler" => Ok(Box::new(SymplecticEuler)),
            "velocity_verlet" => Ok(Box::new(VelocityVerlet)),
            "runge_kutta_fourth_order" => Ok(Box::new(RungeKuttaFourthOrder)),
            "runge_kutta_second_order_midpoint" => Ok(Box::new(RungeKuttaSecondOrderMidpoint)),
            "heun" => Ok(Box::new(Heun)),
            "pefrl" => Ok(Box::new(Pefrl)),
            _ => {
                let available = self.list_available();
                let aliases: Vec<String> = self.aliases.keys().cloned().collect();
                Err(format!(
                    "Unknown integrator: '{}'. Available integrators: {}. Aliases: {}",
                    name,
                    available.join(", "),
                    aliases.join(", ")
                ))
            }
        }
    }

    pub fn list_available(&self) -> Vec<String> {
        vec![
            "symplectic_euler".to_string(),
            "velocity_verlet".to_string(),
            "runge_kutta_fourth_order".to_string(),
            "runge_kutta_second_order_midpoint".to_string(),
            "heun".to_string(),
            "pefrl".to_string(),
        ]
    }

    pub fn list_aliases(&self) -> Vec<(String, String)> {
        let mut aliases: Vec<(String, String)> = self
            .aliases
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        aliases.sort_by(|a, b| a.0.cmp(&b.0));
        aliases
    }
}

impl Default for IntegratorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integrator_registry() {
        let registry = IntegratorRegistry::new();

        // Test that built-in integrators are registered
        let available = registry.list_available();
        assert!(available.contains(&"symplectic_euler".to_string()));
        assert!(available.contains(&"velocity_verlet".to_string()));

        // Test creating integrators
        let _ = registry.create("symplectic_euler").unwrap();
        let _ = registry.create("velocity_verlet").unwrap();

        // Test aliases
        let _ = registry.create("euler").unwrap();
        let _ = registry.create("verlet").unwrap();

        // Test unknown integrator
        let result = registry.create("unknown_integrator");
        assert!(result.is_err());
    }
}
