//! What this crate's tests share: each test runs in the dimension it's built
//! for, with avian's physics stepping headless.

use std::f32::consts::PI;
use std::time::Duration;

use bevy::prelude::*;
use bevy_arbor_perception::prelude::*;
use bevy_harness::TestApp;

use crate::avian::prelude::*;
pub(crate) use crate::dim::SPACE;
use crate::prelude::*;

/// Perception and physics, with this crate's plugins.
pub(crate) fn app() -> TestApp {
    TestApp::builder()
        .plugins((
            TransformPlugin,
            PhysicsPlugins::default(),
            PerceptionPlugin::default().with_space(SPACE),
            AvianPerceptionPlugins,
        ))
        .build()
}

/// `distance` ahead of an agent at the origin looking its own way.
pub(crate) fn ahead(distance: f32) -> Vec3 {
    #[cfg(dim = "2d")]
    return Vec3::new(distance, 0.0, 0.0);
    #[cfg(dim = "3d")]
    return Vec3::new(0.0, 0.0, -distance);
}

/// A collider a unit thick along the line ahead, a unit wide across it, and
/// `height` high: the same block in either dimension.
pub(crate) fn block(height: f32) -> Collider {
    #[cfg(dim = "2d")]
    return Collider::rectangle(1.0, height);
    #[cfg(dim = "3d")]
    return Collider::cuboid(1.0, height, 1.0);
}

/// A wall `height` high, standing `distance` ahead, on `layers`.
pub(crate) fn wall(
    app: &mut TestApp,
    distance: f32,
    height: f32,
    layers: CollisionLayers,
) -> Entity {
    app.spawn((
        RigidBody::Static,
        block(height),
        layers,
        Transform::from_translation(ahead(distance)),
    ))
}

/// An agent at the origin that sees all round, every tick.
pub(crate) fn looker(app: &mut TestApp) -> Entity {
    app.spawn((
        Sight::new(20.0)
            .with_half_angle(PI)
            .with_interval(Duration::ZERO),
        Transform::default(),
    ))
}

/// Something that can be seen, `distance` ahead: with a small collider, for
/// sight to find it through avian.
pub(crate) fn thing(app: &mut TestApp, distance: f32) -> Entity {
    app.spawn((
        Perceivable,
        crate::dim::ball(0.25),
        Transform::from_translation(ahead(distance)),
    ))
}

/// Whether `agent` has `thing` in view.
pub(crate) fn sees(app: &TestApp, agent: Entity, thing: Entity) -> bool {
    app.world()
        .get::<Perceived<Sight>>(agent)
        .and_then(|seen| seen.get(thing))
        .is_some_and(Percept::is_active)
}

/// The avian plugins without avian's physics: they say so, and do nothing.
#[test]
fn without_physics_it_warns_and_does_nothing() {
    let mut app = TestApp::builder()
        .plugins((
            TransformPlugin,
            PerceptionPlugin::default().with_space(SPACE),
            AvianPerceptionPlugins,
        ))
        .build();
    let agent = looker(&mut app);
    let target = thing(&mut app, 10.0);
    app.ticks(2);
    assert!(
        sees(&app, agent, target),
        "nothing blocks sight, nothing panics"
    );
}
