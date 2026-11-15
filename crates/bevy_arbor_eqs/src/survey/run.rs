use std::sync::Arc;

use bevy::platform::time::Instant;
use bevy::prelude::*;
use bevy_arbor_bt::behavior::Outcome;
use bevy_arbor_bt::task::{Task, TaskOf};

use super::request::{Asked, Pending, Request};
use super::trace::EnvQueryTrace;
use crate::EnvQueryBudget;
use crate::query::Parts;

/// Runs the requests waiting, oldest first, within the budget.
pub(crate) fn run(world: &mut World) {
    let budget = world.resource::<EnvQueryBudget>().0;
    let mut pending = std::mem::take(&mut world.resource_mut::<Pending>().0);
    let start = Instant::now();
    let mut ran = 0;
    while let Some(request) = pending.pop_front() {
        if ran > 0 && start.elapsed() >= budget {
            pending.push_front(request);
            break;
        }
        let Some(agent) = world.get::<TaskOf>(request.entity).map(TaskOf::agent) else {
            // Gone, or not a task.
            if world.get_entity(request.entity).is_ok() {
                warn!(
                    "{} has a `Survey<{}>` but isn't a task: run one in a tree as `task(Survey::of(..))`, or a query at once with `EnvQuery::run`",
                    request.entity,
                    request.key.name()
                );
                request.key.take(world, request.entity);
            }
            continue;
        };
        let carrier = match request.key.carrier(world, request.entity, agent) {
            Asked::Of(carrier) => carrier,
            // Nothing to ask it of: what it'd be asked of isn't found.
            Asked::OfNothing => {
                settle(world, &request, false);
                continue;
            }
            // Cancelled, or gone, meanwhile.
            Asked::No => continue,
        };
        let Some(query) = request.key.query(world, carrier) else {
            warn!(
                "{agent} asked {carrier} for its query for {}, but it carries none: give it an `EnvQuery`",
                request.key.name()
            );
            settle(world, &request, false);
            continue;
        };
        ran += 1;
        survey(world, &request, agent, carrier, &query);
    }
    // What's left goes before anything asked for meanwhile.
    let mut queue = world.resource_mut::<Pending>();
    pending.extend(queue.0.drain(..));
    queue.0 = pending;
}

/// Runs `query`, carried by `carrier`, for `agent`, as `request` asked.
fn survey(
    world: &mut World,
    request: &Request,
    agent: Entity,
    carrier: Entity,
    query: &Arc<Parts>,
) {
    let ran = Parts::run_in(query, world, agent, carrier);
    let found = !ran.picked.is_empty();
    if let Some(mut trace) = world.get_mut::<EnvQueryTrace>(agent) {
        trace.set_if_neq(EnvQueryTrace {
            query: request.key.name(),
            items: ran.weighed,
        });
    }
    request.key.write(world, agent, ran.picked);
    settle(world, request, found);
}

/// Ends `request`: its task reported, as `found` says.
fn settle(world: &mut World, request: &Request, found: bool) {
    if let Some(mut task) = world.get_mut::<Task>(request.entity) {
        task.report(Outcome::from(found));
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::prelude::*;
    use bevy_arbor_bt::prelude::*;
    use bevy_arbor_perception::prelude::*;
    use bevy_harness::TestApp;

    use crate::prelude::*;
    use crate::query::EnvQueryKey;
    use crate::testing::*;

    struct Spot;

    /// The point 3 out along +X, or nothing.
    fn spot(reach: f32) -> EnvQuery<Spot> {
        EnvQuery::new(Spot, Donut::around(Querier).radius(3.0, 3.0).points(4))
            .filter(Distance::to(Querier), Filter::at_most(reach))
            .score(Dot::of(Querier), Score::lower())
    }

    /// An agent at the origin whose tree runs `survey` as a task.
    fn asker(app: &mut TestApp, survey: Survey<impl EnvQueryKey>) -> Entity {
        app.spawn((
            GlobalTransform::IDENTITY,
            BehaviorTree::new(behavior_tree! { task(survey) }),
        ))
    }

    #[test]
    fn a_survey_writes_what_it_found_on_the_agent() {
        let mut app = app(Space::Volume);
        let agent = app.spawn(GlobalTransform::IDENTITY);
        let found = survey(&mut app, agent, Spot, spot(5.0)).unwrap();
        assert_eq!(found.len(), 1);
        let behind = found[0].item.position;
        assert!(
            (behind - Vec3::new(0.0, 0.0, 3.0)).length() < 1e-4,
            "{behind}"
        );
    }

    #[test]
    fn a_survey_finding_nothing_takes_found_away() {
        let mut app = app(Space::Volume);
        let agent = app.spawn(GlobalTransform::IDENTITY);
        assert!(survey(&mut app, agent, Spot, spot(5.0)).is_some());
        assert!(survey(&mut app, agent, Spot, spot(1.0)).is_none());
    }

    #[test]
    fn a_survey_task_succeeds_if_it_found_something_and_fails_if_not() {
        for (reach, found) in [(5.0, true), (1.0, false)] {
            let mut app = app(Space::Volume);
            let agent = app.spawn((
                GlobalTransform::IDENTITY,
                spot(reach),
                BehaviorTree::new(behavior_tree! { task(Survey::of(Spot)) }),
            ));
            app.tick();
            assert_eq!(
                app.world().get::<Found<Spot>>(agent).is_some(),
                found,
                "the same tick the tree asked"
            );
            app.tick();
            let outcome = app.world().get::<BehaviorTree>(agent).unwrap().outcome();
            assert_eq!(outcome, Some(Outcome::from(found)));
        }
    }

    #[test]
    fn a_tree_reads_what_was_found_with_a_typed_condition() {
        #[derive(Component, Clone)]
        struct GoThere;
        let mut app = app(Space::Volume);
        app.spawn((
            GlobalTransform::IDENTITY,
            spot(5.0),
            BehaviorTree::new(behavior_tree! {
                Sequence => {
                    task(Survey::of(Spot)),
                    Select => { when(found::<Spot>) => task(GoThere) },
                }
            }),
        ));
        app.ticks(2);
        assert_eq!(app.count::<With<GoThere>>(), 1);
    }

    #[test]
    fn a_survey_cancelled_while_it_waits_is_not_run() {
        let mut app = app(Space::Volume);
        app.app_mut()
            .insert_resource(EnvQueryBudget(Duration::ZERO));
        let first = asker(&mut app, Survey::of(Spot));
        let second = asker(&mut app, Survey::of(Spot));
        app.entity_mut(first).insert(spot(5.0));
        app.entity_mut(second).insert(spot(5.0));
        app.tick();
        // The second waits its turn; its tree going cancels its task.
        app.entity_mut(second).remove::<BehaviorTree>();
        app.tick();
        assert!(app.world().get::<Found<Spot>>(first).is_some());
        assert!(app.world().get::<Found<Spot>>(second).is_none());
    }

    #[test]
    fn past_the_budget_the_rest_wait_their_turn() {
        let mut app = app(Space::Volume);
        app.app_mut()
            .insert_resource(EnvQueryBudget(Duration::ZERO));
        let agents: Vec<Entity> = (0..3)
            .map(|_| {
                let agent = asker(&mut app, Survey::of(Spot));
                app.entity_mut(agent).insert(spot(5.0));
                agent
            })
            .collect();
        let found = |app: &TestApp| {
            agents
                .iter()
                .map(|&agent| app.world().get::<Found<Spot>>(agent).is_some())
                .collect::<Vec<_>>()
        };
        app.tick();
        assert_eq!(found(&app), [true, false, false], "one a tick, at least");
        app.tick();
        assert_eq!(found(&app), [true, true, false], "oldest first");
        app.tick();
        assert_eq!(found(&app), [true, true, true]);
    }

    #[test]
    fn found_changes_only_when_what_was_found_does() {
        let mut app = app(Space::Volume);
        let agent = app.spawn(GlobalTransform::IDENTITY);
        survey(&mut app, agent, Spot, spot(5.0));
        let changed = |app: &TestApp| {
            app.world()
                .entity(agent)
                .get_ref::<Found<Spot>>()
                .unwrap()
                .last_changed()
        };
        let first = changed(&app);
        survey(&mut app, agent, Spot, spot(5.0));
        assert_eq!(changed(&app), first);
    }

    #[test]
    fn a_trace_keeps_how_every_item_was_weighed() {
        let mut app = app(Space::Volume);
        let agent = app.spawn((GlobalTransform::IDENTITY, EnvQueryTrace::default()));
        survey(&mut app, agent, Spot, spot(5.0));
        let trace = app.world().get::<EnvQueryTrace>(agent).unwrap();
        assert_eq!(trace.items.len(), 4);
        assert_eq!(
            trace.items.iter().filter(|weighed| weighed.picked).count(),
            1
        );
        assert_eq!(trace.query, "Spot");
    }

    #[test]
    fn a_survey_of_a_query_not_carried_fails_rather_than_hang() {
        let mut app = app(Space::Volume);
        let agent = app.spawn((
            GlobalTransform::IDENTITY,
            BehaviorTree::new(behavior_tree! { task(Survey::of(Spot)) }),
        ));
        app.ticks(2);
        let outcome = app.world().get::<BehaviorTree>(agent).unwrap().outcome();
        assert_eq!(outcome, Some(Outcome::Failure));
    }

    #[test]
    fn a_survey_outside_a_tree_is_taken_off() {
        let mut app = app(Space::Volume);
        let direct = app.spawn((GlobalTransform::IDENTITY, spot(5.0), Survey::of(Spot)));
        app.tick();
        assert!(
            app.world().get::<Survey<Spot>>(direct).is_none(),
            "taken off"
        );
        assert!(app.world().get::<Found<Spot>>(direct).is_none(), "not run");
    }

    /// Where to stand to use a turret: 2 in front of it, by the turret.
    struct UseSpot;

    fn use_spot() -> EnvQuery<UseSpot> {
        EnvQuery::new(UseSpot, Donut::around(Carrier).radius(2.0, 2.0).points(4))
            .score(Dot::of(Carrier), Score::higher())
    }

    /// A turret at `at`, looking along -Z, carrying where to stand to use it.
    fn turret(app: &mut TestApp, at: Vec3) -> Entity {
        app.spawn((GlobalTransform::from_translation(at), use_spot()))
    }

    #[test]
    fn a_survey_on_a_thing_runs_the_query_it_carries_from_it() {
        let mut app = app(Space::Volume);
        let turret = turret(&mut app, Vec3::new(10.0, 0.0, 0.0));
        let agent = asker(&mut app, Survey::of(UseSpot).on(turret));
        app.tick();
        let found = app.world().get::<Found<UseSpot>>(agent).unwrap();
        let at = found.position().unwrap();
        assert!(
            (at - Vec3::new(10.0, 0.0, -2.0)).length() < 1e-4,
            "in front of the turret, not the agent: {at}"
        );
    }

    #[test]
    fn a_survey_on_what_was_found_asks_it_as_it_runs() {
        /// The nearest turret.
        struct Turrets;
        #[derive(Component)]
        struct Turret;
        let mut app = app(Space::Volume);
        let near = turret(&mut app, Vec3::new(5.0, 0.0, 0.0));
        app.entity_mut(near).insert(Turret);
        let far = turret(&mut app, Vec3::new(50.0, 0.0, 0.0));
        app.entity_mut(far).insert(Turret);
        let turrets = EnvQuery::new(Turrets, Nearby::<Turret>::within(Querier, 100.0))
            .score(Distance::to(Querier), Score::lower());
        let agent = app.spawn((
            GlobalTransform::IDENTITY,
            turrets,
            BehaviorTree::new(behavior_tree! {
                Sequence => {
                    task(Survey::of(Turrets)),
                    task(Survey::of(UseSpot).on_found::<Turrets>()),
                }
            }),
        ));
        app.ticks(3);
        let at = app
            .world()
            .get::<Found<UseSpot>>(agent)
            .unwrap()
            .position()
            .unwrap();
        assert!(
            (at - Vec3::new(5.0, 0.0, -2.0)).length() < 1e-4,
            "by the nearest turret: {at}"
        );
    }

    #[test]
    fn a_survey_on_what_was_found_fails_while_nothing_is() {
        struct Turrets;
        let mut app = app(Space::Volume);
        let agent = app.spawn((
            GlobalTransform::IDENTITY,
            BehaviorTree::new(behavior_tree! { task(Survey::of(UseSpot).on_found::<Turrets>()) }),
        ));
        app.ticks(2);
        let outcome = app.world().get::<BehaviorTree>(agent).unwrap().outcome();
        assert_eq!(outcome, Some(Outcome::Failure));
    }

    #[test]
    fn one_query_serves_every_thing_carrying_it() {
        let mut app = app(Space::Volume);
        let first = turret(&mut app, Vec3::new(10.0, 0.0, 0.0));
        let second = turret(&mut app, Vec3::new(-10.0, 0.0, 0.0));
        let one = asker(&mut app, Survey::of(UseSpot).on(first));
        let other = asker(&mut app, Survey::of(UseSpot).on(second));
        app.tick();
        let at = |agent| {
            app.world()
                .get::<Found<UseSpot>>(agent)
                .unwrap()
                .position()
                .unwrap()
        };
        assert!((at(one) - Vec3::new(10.0, 0.0, -2.0)).length() < 1e-4);
        assert!((at(other) - Vec3::new(-10.0, 0.0, -2.0)).length() < 1e-4);
    }

    #[test]
    fn a_query_runs_at_once_for_a_system_asking() {
        let mut app = app(Space::Volume);
        let turret = turret(&mut app, Vec3::new(10.0, 0.0, 0.0));
        let agent = app.spawn(GlobalTransform::IDENTITY);
        app.tick();
        let found = use_spot().run_on(app.world_mut(), agent, turret);
        let at = found[0].item.position;
        assert!(
            (at - Vec3::new(10.0, 0.0, -2.0)).length() < 1e-4,
            "from the turret: {at}"
        );
        assert_eq!(spot(5.0).run(app.world_mut(), agent).len(), 1);
    }
}
