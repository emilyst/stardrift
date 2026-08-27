//! Bodies plugin - Self-contained plugin pattern
//!
//! Renders every physics body with one shared unit-sphere mesh and one shared
//! `BodyMaterial`; per-body color is packed into `MeshTag` (see
//! `material.rs`), so all bodies collapse into a single instanced draw.
//! Visuals attach reactively on `Added<PhysicsBody>` — the simulation plugin
//! spawns bodies without touching any render asset.

mod material;

pub use material::{BodyMaterial, BodyMaterialHandle};

use crate::physics::components::{BodyColor, PhysicsBody, Radius};
use crate::plugins::simulation::SimulationSet;
use crate::prelude::*;
use bevy::asset::embedded_asset;
use bevy::mesh::{MeshTag, SphereKind};
use bevy::pbr::MaterialPlugin;
use material::pack_body_color;

/// Shared unit-sphere mesh for all celestial bodies.
///
/// Radius is carried by `Transform::scale` (set at spawn, synced from
/// `Radius` by this plugin), so every body shares this one mesh asset and
/// batches with bodies sharing the material.
#[derive(Resource, Deref)]
pub struct BodyMesh(pub Handle<Mesh>);

impl FromWorld for BodyMesh {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        Self(
            meshes.add(
                Sphere::new(1.0)
                    .mesh()
                    .kind(SphereKind::Ico {
                        // The default vertex shader needs ATTRIBUTE_NORMAL
                        // (ico meshes carry it) for world_normal.
                        subdivisions: if cfg!(target_arch = "wasm32") { 1 } else { 4 },
                    })
                    .build(),
            ),
        )
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum BodySet {
    Attach,
}

pub struct BodiesPlugin;

impl Plugin for BodiesPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "body.wgsl");
        app.add_plugins(MaterialPlugin::<BodyMaterial>::default());
        app.init_resource::<BodyMesh>();
        app.init_resource::<BodyMaterialHandle>();

        // Restart despawns and respawns bodies from SimulationSet::Input;
        // this edge (and the sync point Bevy inserts for it) guarantees the
        // fresh bodies are visible the same frame. The attach system is
        // deliberately ungated by AppState: Added<PhysicsBody> is a one-shot
        // window, and missing it would leave a body permanently invisible.
        app.configure_sets(Update, SimulationSet::Input.before(BodySet::Attach));
        app.add_systems(
            Update,
            (Self::attach_body_visuals, Self::sync_scale_from_radius).in_set(BodySet::Attach),
        );
    }
}

impl BodiesPlugin {
    fn attach_body_visuals(
        mut commands: Commands,
        bodies: Query<(Entity, Option<&BodyColor>), Added<PhysicsBody>>,
        mesh: Res<BodyMesh>,
        material: Res<BodyMaterialHandle>,
    ) {
        for (entity, color) in bodies.iter() {
            // White fallback mirrors initialize_trails: a spawn path that
            // forgot BodyColor gets a visible body, not a silent invisible
            // one (Added is a one-shot window; there is no second chance).
            let color = color.map_or(Color::WHITE, |c| c.0);
            commands.entity(entity).insert((
                Mesh3d(mesh.0.clone()),
                MeshMaterial3d(material.0.clone()),
                MeshTag(pack_body_color(color)),
            ));
        }
    }

    /// With a unit mesh, scale is literally the radius. Spawn sets it in
    /// `PhysicsBodyBundle`; this system owns changes (collision merges).
    /// The write is absolute, not multiplicative, so repeated merges cannot
    /// accumulate drift. `sync_transform_from_position` writes only
    /// translation, so the two never conflict. The unordered overlap with
    /// trails' `update_trails` (reads body Transform) is benign for the same
    /// reason: trails read only translation.
    fn sync_scale_from_radius(
        mut bodies: Query<(&Radius, &mut Transform), (With<PhysicsBody>, Changed<Radius>)>,
    ) {
        for (radius, mut transform) in bodies.iter_mut() {
            transform.scale = Vec3::splat(radius.value() as f32);
        }
    }
}
