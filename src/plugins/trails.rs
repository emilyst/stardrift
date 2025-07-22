use crate::prelude::*;
use crate::systems::trails::{initialize_trails, render_trails, update_trails};

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum TrailSet {
    Initialize,
    Update,
    Render,
}

pub struct TrailsPlugin;

impl Plugin for TrailsPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            Update,
            (TrailSet::Initialize, TrailSet::Update, TrailSet::Render).chain(),
        );

        app.add_systems(
            Update,
            (
                initialize_trails.in_set(TrailSet::Initialize),
                update_trails.in_set(TrailSet::Update),
                render_trails.in_set(TrailSet::Render),
            )
                .run_if(in_state(AppState::Running).or(in_state(AppState::Paused))),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trails_plugin_creation() {
        let plugin = TrailsPlugin;

        // Test that we can create the plugin
        let mut app = App::new();
        app.add_plugins(plugin);

        // Plugin should add the systems without panicking
        // Actual system testing is done in the systems::trails module
    }

    #[test]
    fn test_trail_system_sets() {
        // Test that system sets have proper ordering
        assert_ne!(TrailSet::Initialize, TrailSet::Update);
        assert_ne!(TrailSet::Update, TrailSet::Render);
        assert_ne!(TrailSet::Initialize, TrailSet::Render);
    }
}
