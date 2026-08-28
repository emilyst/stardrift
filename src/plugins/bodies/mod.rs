//! Bodies plugin - Self-contained plugin pattern
//!
//! Renders every physics body as a camera-facing disc impostor: one shared
//! quad mesh billboarded and shaded in `body.wgsl`, one shared
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
use bevy::camera::primitives::Aabb;
use bevy::camera::visibility::NoAutoAabb;
use bevy::math::Vec3A;
use bevy::mesh::MeshTag;
use bevy::pbr::MaterialPlugin;
use material::pack_body_color;

/// Shared quad mesh for all celestial bodies, billboarded in the vertex
/// shader (`body.wgsl`) into a camera-facing disc.
///
/// Radius is carried by `Transform::scale` (set at spawn, synced from
/// `Radius` by this plugin), so every body shares this one mesh asset and
/// batches with bodies sharing the material. Sized 2×2 so local positions
/// are the [-1, 1] disc coordinates the shader expects; NORMAL and UV_0 stay
/// on the mesh because the pipeline derives its vertex layout and shader-defs
/// from mesh attributes, even though the shader ignores their values.
#[derive(Resource, Deref)]
pub struct BodyMesh(pub Handle<Mesh>);

impl FromWorld for BodyMesh {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        Self(meshes.add(Rectangle::new(2.0, 2.0)))
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
                // The quad billboards along camera axes, so the mesh-derived
                // AABB (a flat plate in local XY) is wrong in a
                // camera-dependent way — bodies would pop out at screen
                // edges. This unit-sphere bound matches what the shader can
                // reach. NoAutoAabb is required, not decorative: without it
                // the Changed<Mesh3d> from this very insert triggers
                // calculate_bounds to overwrite the Aabb with the flat one.
                Aabb {
                    center: Vec3A::ZERO,
                    half_extents: Vec3A::splat(1.0),
                },
                NoAutoAabb,
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
