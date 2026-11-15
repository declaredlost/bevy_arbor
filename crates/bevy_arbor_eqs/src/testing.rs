//! What this crate's tests share.

use std::time::Duration;

use bevy::prelude::*;
use bevy_arbor_bt::prelude::*;
use bevy_arbor_perception::prelude::*;
use bevy_harness::TestApp;

use crate::generate::Generate;
use crate::prelude::*;
use crate::query::{EnvQueryContext, EnvQueryItem, EnvQueryKey};
use crate::test::Test;

/// An app with trees, perception in `space`, and queries.
pub(crate) fn app(space: Space) -> TestApp {
    TestApp::builder()
        .plugins((
            BehaviorTreePlugin::default(),
            PerceptionPlugin::default().with_space(space),
            EnvQueryPlugin::default(),
        ))
        .build()
}

/// An agent at the origin, of allegiance 1, looking ahead every tick.
pub(crate) fn looker(app: &mut TestApp) -> Entity {
    app.spawn((
        Sight::new(20.0).with_interval(Duration::ZERO),
        Allegiance(1),
        GlobalTransform::IDENTITY,
    ))
}

/// An enemy at `at`, of allegiance 2.
pub(crate) fn enemy(app: &mut TestApp, at: Vec3) -> Entity {
    app.spawn((
        Perceivable,
        Allegiance(2),
        GlobalTransform::from_translation(at),
    ))
}

/// Has `agent` carry `query`, for `key`, and run it as a tree's task: what it
/// found.
pub(crate) fn survey<K: EnvQueryKey>(
    app: &mut TestApp,
    agent: Entity,
    key: K,
    query: EnvQuery<K>,
) -> Option<Vec<ScoredItem>> {
    let tree = behavior_tree! { task(Survey::of(key)) };
    app.entity_mut(agent)
        .insert((query, BehaviorTree::new(tree)));
    app.tick();
    app.world()
        .get::<Found<K>>(agent)
        .map(|found| found.iter().copied().collect())
}

/// `test`'s numbers for `items`, for `agent`, its state made afresh.
pub(crate) fn tested<T: Test>(
    world: &mut World,
    agent: Entity,
    test: &T,
    items: &[EnvQueryItem],
) -> Vec<Option<f32>> {
    let mut state = T::State::from_world(world);
    test.prepare(&mut state, world);
    let space = world.get_resource::<Space>().copied().unwrap_or_default();
    test.test(
        &mut state,
        &EnvQueryContext::new(world, agent, space, 1),
        items,
    )
}

/// What `generator` makes for `agent`, its state made afresh.
pub(crate) fn generated<G: Generate>(
    world: &mut World,
    agent: Entity,
    generator: &G,
) -> Vec<EnvQueryItem> {
    let mut state = G::State::from_world(world);
    generator.prepare(&mut state, world);
    let space = world.get_resource::<Space>().copied().unwrap_or_default();
    let mut out = Vec::new();
    generator.generate(
        &mut state,
        &EnvQueryContext::new(world, agent, space, 1),
        &mut out,
    );
    out
}
