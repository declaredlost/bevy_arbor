//! What this crate's tests share.

use bevy::prelude::*;
use bevy_arbor_bt::prelude::*;
use bevy_arbor_eqs::prelude::*;
use bevy_arbor_perception::prelude::*;
use bevy_harness::TestApp;

use crate::prelude::*;

/// What slots offer in tests.
#[derive(Component)]
pub(crate) struct Sit;

/// The query finding the nearest free seat.
pub(crate) struct Seat;

/// What an agent does once it has a seat.
#[derive(Component, Clone)]
pub(crate) struct GoThere;

/// The behavior a bench's seats bring.
#[derive(Component, Clone)]
pub(crate) struct Rest;

pub(crate) fn rest() -> Behavior {
    behavior_tree! { task(Rest) }
}

/// Trees, queries and smart objects, in 3D, with the seat query added.
pub(crate) fn app() -> TestApp {
    TestApp::builder()
        .plugins((
            TreePlugin::default(),
            PerceptionPlugin::default(),
            EqsPlugin::default(),
            SmartPlugin,
        ))
        .with(|app| {
            app.add_env_query(
                Seat,
                EnvQuery::new(FreeSlots::<Sit>::within(Agent, 20.0))
                    .score(Distance::to(Agent), Score::lower()),
            );
        })
        .build()
}

/// A bench with a seat at each of `seats`, bringing [`rest`].
pub(crate) fn bench(app: &mut TestApp, seats: &[Vec3]) -> Vec<Entity> {
    let bench = app.spawn(GlobalTransform::IDENTITY);
    seats
        .iter()
        .map(|&at| {
            app.spawn((
                SlotOf(bench),
                Sit,
                Plugged::new(Using, rest),
                GlobalTransform::from_translation(at),
            ))
        })
        .collect()
}

/// An agent at the origin that finds a seat, claims it, goes there and uses
/// it, over and over.
pub(crate) fn sitter(app: &mut TestApp) -> Entity {
    app.spawn((
        GlobalTransform::IDENTITY,
        BehaviorTree::new(behavior_tree! {
            Forever => {
                Select => {
                    when(found::<Seat> && claim::<Seat>()) => Sequence => {
                        task(GoThere),
                        Socket(Using),
                    },
                    task(Survey::of(Seat)),
                }
            }
        }),
    ))
}

/// The slot `agent` holds, if any.
pub(crate) fn held(app: &TestApp, agent: Entity) -> Option<Entity> {
    app.world()
        .get::<Claims>(agent)
        .and_then(|claims| claims.iter().next())
}

/// Reports the agent's running `GoThere` task as done.
pub(crate) fn arrive(app: &mut TestApp) {
    let mut tasks = app.world_mut().query_filtered::<&mut Task, With<GoThere>>();
    for mut task in tasks.iter_mut(app.world_mut()) {
        task.succeed();
    }
}
