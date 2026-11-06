//! What this crate's tests share.

use bevy::prelude::*;
use bevy_harness::TestApp;

use crate::prelude::*;

/// Guards read this off the agent.
#[derive(Component, Default)]
pub(crate) struct Flags {
    pub(crate) high: bool,
    pub(crate) low: bool,
}

#[derive(Component, Clone)]
pub(crate) struct Work;

#[derive(Component, Clone)]
pub(crate) struct Other;

/// On a task: how it reports, the tick it's ticked.
#[derive(Component, Clone, Copy, PartialEq)]
pub(crate) enum Report {
    Succeed,
    Fail,
}

#[derive(Resource, Default)]
pub(crate) struct Acts(pub(crate) u32);

pub(crate) fn high(In(agent): In<Entity>, flags: Query<&Flags>) -> bool {
    flags.get(agent).is_ok_and(|flags| flags.high)
}

pub(crate) fn low(In(agent): In<Entity>, flags: Query<&Flags>) -> bool {
    flags.get(agent).is_ok_and(|flags| flags.low)
}

pub(crate) fn count(In(_): In<Entity>, mut acts: ResMut<Acts>) -> bool {
    acts.0 += 1;
    true
}

pub(crate) fn refuse(In(_): In<Entity>, mut acts: ResMut<Acts>) -> bool {
    acts.0 += 1;
    false
}

fn report(mut tasks: Query<(&mut Task, &Report)>) {
    for (mut task, report) in &mut tasks {
        match report {
            Report::Succeed => task.succeed(),
            Report::Fail => task.fail(),
        }
    }
}

pub(crate) fn app() -> TestApp {
    TestApp::builder()
        .plugins(ArborPlugin::default())
        .systems(FixedUpdate, report.after(ArborSystems))
        .with(|app| {
            app.init_resource::<Acts>();
        })
        .build()
}

pub(crate) fn agent(app: &mut TestApp, tree: Behaviour) -> Entity {
    app.spawn((Flags::default(), BehaviourTree::new(tree)))
}

pub(crate) fn set(app: &mut TestApp, agent: Entity, high: bool, low: bool) {
    app.entity_mut(agent).insert(Flags { high, low });
}

pub(crate) fn outcome(app: &TestApp, agent: Entity) -> Option<bool> {
    app.world()
        .get::<BehaviourTree>(agent)
        .unwrap()
        .outcome()
        .map(Outcome::is_success)
}

pub(crate) fn acts(app: &TestApp) -> u32 {
    app.world().resource::<Acts>().0
}
