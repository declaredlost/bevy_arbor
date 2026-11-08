//! What this crate's tests share.

use std::time::Duration;

use bevy::prelude::*;
use bevy_arbor_bt::prelude::*;
use bevy_arbor_perception::prelude::*;
use bevy_harness::TestApp;

use crate::prelude::*;

/// An app with trees, perception in `space`, and queries.
pub(crate) fn app(space: Space) -> TestApp {
    TestApp::builder()
        .plugins((
            TreePlugin::default(),
            PerceptionPlugin::default().with_space(space),
            EqsPlugin::default(),
        ))
        .build()
}

/// An agent at the origin, on team 1, looking ahead every tick.
pub(crate) fn looker(app: &mut TestApp) -> Entity {
    app.spawn((
        Sight::new(20.0).with_interval(Duration::ZERO),
        Team(1),
        GlobalTransform::IDENTITY,
    ))
}

/// An enemy at `at`, on team 2.
pub(crate) fn enemy(app: &mut TestApp, at: Vec3) -> Entity {
    app.spawn((Perceivable, Team(2), GlobalTransform::from_translation(at)))
}

/// Runs `query`, keyed by `Q`, for `agent` directly, and gives what it found.
pub(crate) fn survey<Q: Send + Sync + 'static>(
    app: &mut TestApp,
    agent: Entity,
    key: Q,
    query: EnvQuery,
) -> Option<Vec<Scored>> {
    app.app_mut().add_env_query(key, query);
    app.entity_mut(agent).insert(Survey::<Q>::default());
    app.tick();
    app.world()
        .get::<Found<Q>>(agent)
        .map(|found| found.iter().copied().collect())
}
