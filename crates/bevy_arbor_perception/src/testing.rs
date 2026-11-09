//! What this crate's tests share: each test runs in 2D and in 3D.

use std::time::Duration;

use bevy::prelude::*;
use bevy_harness::TestApp;

use crate::PerceptionSystems;
use crate::prelude::*;

pub(crate) const SPACES: [Space; 2] = [Space::Planar, Space::Volume];

/// How many agents' `Perceived<Sight>` changed, over every tick so far.
#[derive(Resource, Default)]
pub(crate) struct Changes(pub(crate) usize);

fn count_changes(perceived: Query<(), Changed<Perceived<Sight>>>, mut changes: ResMut<Changes>) {
    changes.0 += perceived.iter().count();
}

pub(crate) fn app(space: Space) -> TestApp {
    TestApp::builder()
        .plugins(PerceptionPlugin::default().with_space(space))
        .systems(FixedUpdate, count_changes.after(PerceptionSystems::Commit))
        .with(|app| {
            app.init_resource::<Changes>();
        })
        .build()
}

/// `distance` ahead of an agent at the origin looking its own way. In 2D, on
/// another z layer, which doesn't count.
pub(crate) fn ahead(space: Space, distance: f32) -> Vec3 {
    match space {
        Space::Planar => Vec3::new(distance, 0.0, 3.0),
        Space::Volume => Vec3::new(0.0, 0.0, -distance),
    }
}

/// `distance` behind it.
pub(crate) fn behind(space: Space, distance: f32) -> Vec3 {
    match space {
        Space::Planar => Vec3::new(-distance, 0.0, 3.0),
        Space::Volume => Vec3::new(0.0, 0.0, distance),
    }
}

/// `distance` away, `angle` off ahead.
pub(crate) fn off_ahead(space: Space, distance: f32, angle: f32) -> Vec3 {
    let (sin, cos) = angle.sin_cos();
    match space {
        Space::Planar => Vec3::new(cos * distance, sin * distance, 0.0),
        Space::Volume => Vec3::new(sin * distance, 0.0, -cos * distance),
    }
}

/// An agent at the origin, looking every tick.
pub(crate) fn looker(app: &mut TestApp, sight: Sight) -> Entity {
    app.spawn((
        sight.with_interval(Duration::ZERO),
        GlobalTransform::IDENTITY,
    ))
}

pub(crate) fn thing(app: &mut TestApp, at: Vec3) -> Entity {
    app.spawn((Perceivable, GlobalTransform::from_translation(at)))
}

/// What `agent` perceives by `S`.
pub(crate) fn perceived<S: Sense>(app: &TestApp, agent: Entity) -> &Perceived<S> {
    app.world().get::<Perceived<S>>(agent).unwrap()
}

/// When and where `agent` sensed what it perceives by `S`.
pub(crate) fn traces<S: Sense>(app: &TestApp, agent: Entity) -> &Traces<S> {
    app.world().get::<Traces<S>>(agent).unwrap()
}

/// Whether `agent` has `thing` in view.
pub(crate) fn sees(app: &TestApp, agent: Entity, thing: Entity) -> bool {
    app.world()
        .get::<Perceived<Sight>>(agent)
        .and_then(|seen| seen.get(thing))
        .is_some_and(Percept::is_active)
}

/// What was triggered, in order: which event, by which sense, about what.
#[derive(Resource, Default)]
pub(crate) struct Log(pub(crate) Vec<(&'static str, &'static str, Entity)>);

/// Logs the events of `S`, under `name`.
fn log_sense<S: Sense>(app: &mut App, name: &'static str) {
    app.add_observer(move |on: On<Noticed<S>>, mut log: ResMut<Log>| {
        log.0.push(("noticed", name, on.what));
    })
    .add_observer(move |on: On<Lost<S>>, mut log: ResMut<Log>| {
        log.0.push(("lost", name, on.what));
    })
    .add_observer(move |on: On<Forgotten<S>>, mut log: ResMut<Log>| {
        log.0.push(("forgotten", name, on.what));
    });
}

pub(crate) fn logged(space: Space) -> TestApp {
    let mut app = app(space);
    let inner = app.app_mut();
    inner.init_resource::<Log>();
    log_sense::<Sight>(inner, "sight");
    log_sense::<Hearing>(inner, "hearing");
    app
}

pub(crate) fn log(app: &TestApp) -> &[(&'static str, &'static str, Entity)] {
    &app.world().resource::<Log>().0
}

/// An agent at the origin that sees every tick and hears, remembering for
/// `sight` and `hearing` seconds.
pub(crate) fn remembering(app: &mut TestApp, sight: f32, hearing: f32) -> Entity {
    let memory = |seconds: f32| {
        if seconds.is_finite() {
            Duration::from_secs_f32(seconds)
        } else {
            Duration::MAX
        }
    };
    app.spawn((
        Sight::new(10.0).with_interval(Duration::ZERO),
        Hearing::new(10.0),
        Perceived::<Sight>::remembering(memory(sight)),
        Perceived::<Hearing>::remembering(memory(hearing)),
        GlobalTransform::IDENTITY,
    ))
}
