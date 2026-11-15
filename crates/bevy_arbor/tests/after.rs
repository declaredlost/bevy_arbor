//! A task system after `ArborSystems` sees what every part did, the same tick.
//!
//! `ArborSystems` holds every part's systems.
#![cfg(all(feature = "eqs", feature = "nav"))]

use bevy::ecs::schedule::{InternedSystemSet, NodeId, SystemSet};
use bevy::prelude::*;
use bevy_arbor::bt::BehaviorTreeSystems;
use bevy_arbor::eqs::EnvQuerySystems;
use bevy_arbor::nav::NavSystems;
use bevy_arbor::perception::PerceptionSystems;
use bevy_arbor::prelude::*;
use bevy_harness::TestApp;

/// Somewhere to go.
struct Spot;

/// What the system after `ArborSystems` saw of the agent, each tick.
///
/// Whether it had `Found<Spot>`, and a `Destination`.
#[derive(Resource, Default)]
struct Seen(Vec<(bool, bool)>);

fn look(
    agents: Query<(Has<Found<Spot>>, Has<Destination>), With<BehaviorTree>>,
    mut seen: ResMut<Seen>,
) {
    seen.0.extend(&agents);
}

fn app() -> TestApp {
    TestApp::builder()
        .plugins(ArborPlugins)
        .with(|app| {
            app.init_resource::<Seen>()
                .add_env_query(Spot, EnvQuery::new(Donut::around(Querier)))
                .add_systems(FixedUpdate, look.after(ArborSystems));
        })
        .build()
}

/// Whether `set` is in `ArborSystems`, in `FixedUpdate`.
///
/// That's where every part runs unless told otherwise.
fn in_arbor(app: &TestApp, set: impl SystemSet) -> bool {
    let graph = app
        .world()
        .resource::<Schedules>()
        .get(FixedUpdate)
        .expect("the parts' schedule")
        .graph();
    let key = |set: InternedSystemSet| graph.system_sets.get_key(set).map(NodeId::Set);
    key(ArborSystems.intern())
        .zip(key(set.intern()))
        .is_some_and(|(arbor, set)| graph.hierarchy().graph().contains_edge(arbor, set))
}

#[test]
fn every_part_is_in_arbor_systems() {
    let app = app();
    assert!(in_arbor(&app, PerceptionSystems::Commit), "perception");
    assert!(in_arbor(&app, BehaviorTreeSystems), "trees");
    assert!(in_arbor(&app, EnvQuerySystems), "queries");
    assert!(in_arbor(&app, NavSystems), "navigation");
}

#[test]
fn a_task_system_sees_what_was_found_and_where_to_go_as_soon_as_asked() {
    let mut app = app();
    app.spawn((
        GlobalTransform::IDENTITY,
        BehaviorTree::new(behavior_tree! {
            Sequence => { task(Survey::of(Spot)), task(MoveTo::found::<Spot>()) }
        }),
    ));
    app.tick();
    assert_eq!(
        app.world().resource::<Seen>().0,
        [(true, false)],
        "found, the tick the tree asked"
    );
    app.tick();
    assert_eq!(
        app.world().resource::<Seen>().0.last(),
        Some(&(true, true)),
        "asked to go, the tick the tree moved on"
    );
}
