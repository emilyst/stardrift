//! System-level tests for the bodies plugin's visual attachment
//! (`attach_body_visuals`): the real `BodiesPlugin` on a headless App, with
//! Bevy's real `calculate_bounds` added to `PostUpdate` exactly where
//! `VisibilityPlugin` puts it in production.
//!
//! What these pin:
//! - every entity gaining `PhysicsBody` gets the shared quad `Mesh3d`, the
//!   shared `MeshMaterial3d<BodyMaterial>`, and a `MeshTag` carrying its
//!   `BodyColor` packed as 10:10:10 linear RGB;
//! - the attach inserts a manual unit-sphere `Aabb` (center zero,
//!   half_extents one) AND `NoAutoAabb`, and that Aabb survives
//!   `calculate_bounds`. The billboarded quad's mesh-derived AABB is a flat
//!   plate (half_extents.z == 0), and the `Changed<Mesh3d>` raised by the
//!   attach itself triggers the overwrite in the same frame — without
//!   `NoAutoAabb`, bodies pass frustum culling only while the plate happens
//!   to face the camera and pop out of view at screen edges;
//! - a control entity without `NoAutoAabb` shows this harness really runs
//!   the overwrite path (the guard above is not vacuously green);
//! - a spawn path that forgot `BodyColor` falls back to white (visible),
//!   and bodies spawned after startup (the restart path) still attach.

use bevy::asset::{AssetApp, AssetPlugin};
use bevy::camera::primitives::Aabb;
use bevy::camera::visibility::{NoAutoAabb, calculate_bounds};
use bevy::math::Vec3A;
use bevy::mesh::MeshTag;
use bevy::prelude::*;
use stardrift::config::SimulationConfig;
use stardrift::physics::components::{BodyColor, PhysicsBodyBundle};
use stardrift::physics::math::Vector;
use stardrift::plugins::bodies::{BodiesPlugin, BodyMaterial, BodyMaterialHandle, BodyMesh};

/// Headless app running the real plugin. No `RenderPlugin`, so `Mesh` must
/// be registered as an asset by hand; `SimulationConfig` feeds the shared
/// material's bloom intensity (`BodyMaterialHandle::from_world`).
fn body_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()));
    app.init_asset::<Mesh>();
    app.insert_resource(SimulationConfig::default());
    app.add_plugins(BodiesPlugin);
    // Production schedules calculate_bounds in PostUpdate (VisibilityPlugin);
    // including it here makes the NoAutoAabb assertions fail for the real
    // reason (the Aabb gets flattened) if the guard component is ever lost.
    app.add_systems(PostUpdate, calculate_bounds);
    app
}

/// Spawn a body the way the simulation does: full `PhysicsBodyBundle`, with
/// `BodyColor` optional to exercise the fallback path.
fn spawn_body(app: &mut App, color: Option<Color>) -> Entity {
    let mut entity =
        app.world_mut()
            .spawn(PhysicsBodyBundle::new(Vector::ZERO, 1.0, 1.0, Vector::ZERO));
    if let Some(color) = color {
        entity.insert(BodyColor(color));
    }
    entity.id()
}

/// White at 10 bits per channel: 1023 (0x3FF) in each of the three fields.
const PACKED_WHITE: u32 = 0x3FFF_FFFF;

// ---------------------------------------------------------------------------
// Bounding volume
// ---------------------------------------------------------------------------

#[test]
fn attach_installs_unit_sphere_aabb_that_survives_calculate_bounds() {
    let mut app = body_app();
    let body = spawn_body(&mut app, Some(Color::WHITE));
    app.update();

    let aabb = app
        .world()
        .get::<Aabb>(body)
        .expect("attach must insert a manual Aabb");
    assert_eq!(aabb.center, Vec3A::ZERO);
    // z is the load-bearing channel: the quad's own AABB has z extent zero,
    // so a flattened value here means calculate_bounds won the race.
    assert_eq!(
        aabb.half_extents,
        Vec3A::splat(1.0),
        "Aabb must be the unit-sphere bound the billboard shader can reach"
    );
    assert!(
        app.world().get::<NoAutoAabb>(body).is_some(),
        "NoAutoAabb must accompany the manual Aabb or calculate_bounds \
         overwrites it on the Changed<Mesh3d> the attach itself raises"
    );

    // Later frames must not regress it either (Changed/AssetChanged paths).
    app.update();
    let aabb = app.world().get::<Aabb>(body).unwrap();
    assert_eq!(aabb.half_extents, Vec3A::splat(1.0));
}

#[test]
fn control_calculate_bounds_flattens_the_aabb_without_no_auto_aabb() {
    // Control for the test above: an entity carrying the body quad and the
    // unit Aabb but no NoAutoAabb. If calculate_bounds did NOT flatten this,
    // the harness would not be exercising the overwrite the suite guards
    // against, and the previous test would be vacuously green.
    let mut app = body_app();
    let mesh = app.world().resource::<BodyMesh>().0.clone();
    let entity = app
        .world_mut()
        .spawn((
            Mesh3d(mesh),
            Aabb {
                center: Vec3A::ZERO,
                half_extents: Vec3A::splat(1.0),
            },
        ))
        .id();
    app.update();

    let aabb = app.world().get::<Aabb>(entity).unwrap();
    assert_eq!(
        aabb.half_extents,
        Vec3A::new(1.0, 1.0, 0.0),
        "the 2x2 quad's mesh-derived AABB is a flat plate; if this stops \
         flattening, the survives-calculate-bounds test proves nothing"
    );
}

// ---------------------------------------------------------------------------
// Draw-call components
// ---------------------------------------------------------------------------

#[test]
fn attach_inserts_shared_mesh_shared_material_and_packed_color_tag() {
    let mut app = body_app();
    // Linear channels chosen exactly representable in f32 so the expected
    // packing is exact: 0.5 * 1023 = 511.5 rounds (half away from zero) to
    // 512; 0.25 * 1023 = 255.75 rounds to 256; 1.0 * 1023 = 1023.
    let body = spawn_body(&mut app, Some(Color::linear_rgb(0.5, 0.25, 1.0)));
    app.update();

    let mesh = app
        .world()
        .get::<Mesh3d>(body)
        .expect("attach must insert Mesh3d");
    assert_eq!(
        mesh.0,
        app.world().resource::<BodyMesh>().0,
        "all bodies must share the one BodyMesh handle (instancing key)"
    );

    let material = app
        .world()
        .get::<MeshMaterial3d<BodyMaterial>>(body)
        .expect("attach must insert MeshMaterial3d<BodyMaterial>");
    assert_eq!(
        material.0,
        app.world().resource::<BodyMaterialHandle>().0,
        "all bodies must share the one BodyMaterial handle (instancing key)"
    );

    let tag = app
        .world()
        .get::<MeshTag>(body)
        .expect("attach must insert MeshTag");
    let expected = (512u32 << 20) | (256 << 10) | 1023;
    assert_eq!(
        tag.0, expected,
        "MeshTag must pack linear RGB at 10:10:10 (red high)"
    );
}

#[test]
fn missing_body_color_falls_back_to_white_not_invisible() {
    let mut app = body_app();
    let body = spawn_body(&mut app, None);
    app.update();

    // Added<PhysicsBody> is a one-shot window: a spawn path that forgot
    // BodyColor must still produce a visible (white) body.
    assert!(app.world().get::<Mesh3d>(body).is_some());
    let tag = app.world().get::<MeshTag>(body).unwrap();
    assert_eq!(tag.0, PACKED_WHITE);
}

// ---------------------------------------------------------------------------
// Attach timing
// ---------------------------------------------------------------------------

#[test]
fn bodies_spawned_after_startup_still_get_visuals() {
    // The restart path despawns and respawns bodies mid-run; attachment must
    // react to Added<PhysicsBody> every frame, not only at startup.
    let mut app = body_app();
    app.update();

    let body = spawn_body(&mut app, Some(Color::WHITE));
    app.update();

    assert!(app.world().get::<Mesh3d>(body).is_some());
    assert!(app.world().get::<NoAutoAabb>(body).is_some());
    let aabb = app.world().get::<Aabb>(body).unwrap();
    assert_eq!(aabb.half_extents, Vec3A::splat(1.0));
}
