//! What this crate's tests share.

use bevy::prelude::*;
use bevy_harness::TestApp;

use crate::behavior::Context;
use crate::decorator::Decorate;
use crate::prelude::*;

/// Decorators read this off the agent.
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

/// How many `Work` tasks have been spawned, ever.
#[derive(Resource, Default)]
pub(crate) struct WorkStarted(pub(crate) u32);

pub(crate) fn high(In(agent): In<Entity>, flags: Query<&Flags>) -> bool {
    flags.get(agent).is_ok_and(|flags| flags.high)
}

pub(crate) fn low(In(agent): In<Entity>, flags: Query<&Flags>) -> bool {
    flags.get(agent).is_ok_and(|flags| flags.low)
}

/// [`high`], typed.
pub(crate) fn is_high(flags: &Flags) -> bool {
    flags.high
}

/// [`low`], typed.
pub(crate) fn is_low(flags: &Flags) -> bool {
    flags.low
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
        .plugins(BehaviorTreePlugin::default())
        .systems(FixedUpdate, report.after(BehaviorTreeSystems))
        .with(|app| {
            app.init_resource::<Acts>()
                .init_resource::<WorkStarted>()
                .init_resource::<Inside>()
                .add_observer(|_: On<Add<Work>>, mut started: ResMut<WorkStarted>| {
                    started.0 += 1;
                });
        })
        .build()
}

pub(crate) fn agent(app: &mut TestApp, tree: Behavior) -> Entity {
    app.spawn((Flags::default(), BehaviorTree::new(tree)))
}

/// An agent, flagged before its first tick.
pub(crate) fn with_flags(app: &mut TestApp, tree: Behavior, high: bool, low: bool) -> Entity {
    let agent = agent(app, tree);
    set(app, agent, high, low);
    agent
}

pub(crate) fn set(app: &mut TestApp, agent: Entity, high: bool, low: bool) {
    app.entity_mut(agent).insert(Flags { high, low });
}

pub(crate) fn outcome(app: &TestApp, agent: Entity) -> Option<bool> {
    app.world()
        .get::<BehaviorTree>(agent)
        .unwrap()
        .outcome()
        .map(Outcome::is_success)
}

pub(crate) fn acts(app: &TestApp) -> u32 {
    app.world().resource::<Acts>().0
}

pub(crate) fn works_started(app: &TestApp) -> u32 {
    app.world().resource::<WorkStarted>().0
}

/// Whether the one `Work` task has been told to stop.
pub(crate) fn work_interrupted(app: &mut TestApp) -> bool {
    let mut tasks = app.world_mut().query_filtered::<&Task, With<Work>>();
    tasks.single(app.world()).unwrap().is_interrupted()
}

/// How many are in a branch: `Counted` started, less ended.
#[derive(Resource, Default)]
pub(crate) struct Inside(pub(crate) i32);

/// Counts its branch's runs in and out, in `Inside`.
#[derive(Inspect)]
pub(crate) struct Counted;

impl Decorate for Counted {
    fn start(&mut self, cx: &mut Context) {
        cx.world_mut().resource_mut::<Inside>().0 += 1;
    }

    fn passes(&mut self, cx: &mut Context) -> bool {
        cx.wait_on_nothing();
        true
    }

    fn end(&mut self, cx: &mut Context, _outcome: Option<Outcome>) {
        cx.world_mut().resource_mut::<Inside>().0 -= 1;
    }
}

/// How many are in a branch now.
pub(crate) fn inside(app: &TestApp) -> i32 {
    app.world().resource::<Inside>().0
}
