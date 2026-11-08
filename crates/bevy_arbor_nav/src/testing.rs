//! What this crate's tests share.

use bevy::prelude::*;
use bevy_arbor_bt::prelude::*;
use bevy_arbor_eqs::prelude::*;
use bevy_arbor_perception::prelude::*;
use bevy_harness::TestApp;

use crate::prelude::*;

/// A query finding [`SPOT`].
pub(crate) struct Spot;

/// Where [`Spot`] finds.
pub(crate) const SPOT: Vec3 = Vec3::new(3.0, 0.0, 0.0);

/// What interrupts moving in tests.
#[derive(Component)]
pub(crate) struct Alarmed;

/// A task: hold still.
#[derive(Component, Clone)]
pub(crate) struct Hold;

pub(crate) fn alarmed(_: &Alarmed) -> bool {
    true
}

/// Trees, queries and navigation, in 3D, with `Spot` added.
pub(crate) fn app() -> TestApp {
    TestApp::builder()
        .plugins((
            TreePlugin::default(),
            PerceptionPlugin::default(),
            EqsPlugin::default(),
            NavPlugin::default(),
        ))
        .with(|app| {
            app.add_env_query(Spot, EnvQuery::new(Grid::around(At(SPOT)).extent(0.0)));
        })
        .build()
}

pub(crate) fn agent(app: &mut TestApp, tree: Behavior) -> Entity {
    app.spawn((GlobalTransform::IDENTITY, BehaviorTree::new(tree)))
}

/// How its tree ended, if it has: succeeded or not.
pub(crate) fn outcome(app: &TestApp, agent: Entity) -> Option<bool> {
    app.world()
        .get::<BehaviorTree>(agent)?
        .outcome()
        .map(Outcome::is_success)
}
