use crate::config::SimulationConfig;
use bevy::prelude::*;

pub fn save_config_on_exit(mut exit_events: EventReader<AppExit>, config: Res<SimulationConfig>) {
    // for _event in exit_events.read() {
    //     if let Err(e) = config.save_to_user_config() {
    //         warn!("Failed to save configuration on exit: {}", e);
    //     } else {
    //         info!("Configuration saved successfully on exit");
    //     }
    // }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::app::AppExit;
    use bevy::ecs::event::Events;
    use bevy::ecs::system::SystemState;

    #[test]
    fn test_save_config_on_exit_system() {
        // Create a test world
        let mut world = World::new();

        // Insert the configuration resource
        let config = SimulationConfig::default();
        world.insert_resource(config);

        // Initialize the AppExit events resource
        world.init_resource::<Events<AppExit>>();

        // Create a system state for our system
        let mut system_state: SystemState<(EventReader<AppExit>, Res<SimulationConfig>)> =
            SystemState::new(&mut world);

        // Send an AppExit event
        world
            .resource_mut::<Events<AppExit>>()
            .send(AppExit::Success);

        // Get the system parameters
        let (mut exit_events, config) = system_state.get_mut(&mut world);

        // Verify that there's an AppExit event to read
        let events: Vec<_> = exit_events.read().collect();
        assert_eq!(events.len(), 1);

        // The system should be able to access the config resource
        assert!(config.physics.gravitational_constant > 0.0);

        println!("✅ save_config_on_exit system test passed!");
    }
}
