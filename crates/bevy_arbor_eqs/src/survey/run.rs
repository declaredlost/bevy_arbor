use bevy::platform::time::Instant;
use bevy::prelude::*;
use bevy_arbor_bt::task::{Task, TaskOf};
use bevy_arbor_perception::Space;

use super::registry::{EnvQueries, Pending, Registered, Request, Seed};
use super::trace::{SurveyTrace, Weighed};
use crate::EqsBudget;
use crate::query::QueryContext;

/// Runs the requests waiting, oldest first, within the budget.
pub(crate) fn run(world: &mut World) {
    let budget = world.resource::<EqsBudget>().0;
    let queries = world.resource::<EnvQueries>().clone();
    let mut pending = std::mem::take(&mut world.resource_mut::<Pending>().0);
    let start = Instant::now();
    let mut ran = 0;
    while let Some(request) = pending.pop_front() {
        if ran > 0 && start.elapsed() >= budget {
            pending.push_front(request);
            break;
        }
        // Cancelled, or gone, meanwhile.
        if !(request.asked)(world, request.entity) {
            continue;
        }
        let Some(registered) = queries.0.get(&request.key) else {
            warn!(
                "{} asked for the query keyed by {}, but none was added: add it with `add_env_query`",
                request.entity,
                (request.name)()
            );
            settle(world, &request, false);
            continue;
        };
        ran += 1;
        survey(world, &request, registered);
    }
    // What's left goes before anything asked for meanwhile.
    let mut queue = world.resource_mut::<Pending>();
    pending.extend(queue.0.drain(..));
    queue.0 = pending;
}

/// Runs `registered` for what `entity` asks for: its task's agent, or itself.
fn survey(world: &mut World, request: &Request, registered: &Registered) {
    let entity = request.entity;
    let agent = world.get::<TaskOf>(entity).map_or(entity, TaskOf::agent);
    let space = world.get_resource::<Space>().copied().unwrap_or_default();
    let seed = {
        let mut seed = world.resource_mut::<Seed>();
        seed.0 = seed
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        seed.0
    };
    registered.query.prepare(world);
    let ran = registered
        .query
        .run(&QueryContext::new(world, agent, space, seed));
    let found = !ran.picked.is_empty();
    if let Some(mut trace) = world.get_mut::<SurveyTrace>(agent) {
        let picked: Vec<_> = ran.picked.iter().map(|scored| scored.item).collect();
        let items = ran
            .weighed
            .iter()
            .map(|&(item, score)| Weighed {
                item,
                score,
                picked: picked.contains(&item),
            })
            .collect();
        trace.set_if_neq(SurveyTrace {
            query: (request.name)(),
            items,
        });
    }
    (registered.write)(world, agent, ran.picked);
    settle(world, request, found);
}

/// Ends `request`: its task reported, as `found` says, or else its survey
/// taken off.
fn settle(world: &mut World, request: &Request, found: bool) {
    match world.get_mut::<Task>(request.entity) {
        Some(mut task) if found => task.succeed(),
        Some(mut task) => task.fail(),
        None => (request.taken)(world, request.entity),
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
    use crate::testing::*;

    struct Spot;

    /// The point 3 out along +X, or nothing.
    fn spot(reach: f32) -> EnvQuery {
        EnvQuery::new(Donut::around(Agent).radius(3.0, 3.0).points(4))
            .filter(Distance::to(Agent), Filter::at_most(reach))
            .score(Dot::of(Agent), Score::lower())
    }

    #[test]
    fn a_survey_on_an_agent_writes_what_it_found_and_goes() {
        let mut app = app(Space::Volume);
        let agent = app.spawn(GlobalTransform::IDENTITY);
        let found = survey(&mut app, agent, Spot, spot(5.0)).unwrap();
        assert_eq!(found.len(), 1);
        assert!(
            app.world().get::<Survey<Spot>>(agent).is_none(),
            "taken off"
        );
        let behind = found[0].item.position();
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
            app.app_mut().add_env_query(Spot, spot(reach));
            let agent = app.spawn((
                GlobalTransform::IDENTITY,
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
        app.app_mut().add_env_query(Spot, spot(5.0));
        app.spawn((
            GlobalTransform::IDENTITY,
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
    fn a_cancelled_survey_is_not_run() {
        let mut app = app(Space::Volume);
        app.app_mut().add_env_query(Spot, spot(5.0));
        let agent = app.spawn(GlobalTransform::IDENTITY);
        app.entity_mut(agent).insert(Survey::of(Spot));
        app.entity_mut(agent).remove::<Survey<Spot>>();
        app.tick();
        assert!(app.world().get::<Found<Spot>>(agent).is_none());
    }

    #[test]
    fn past_the_budget_the_rest_wait_their_turn() {
        let mut app = app(Space::Volume);
        app.app_mut()
            .add_env_query(Spot, spot(5.0))
            .insert_resource(EqsBudget(Duration::ZERO));
        let agents: Vec<Entity> = (0..3)
            .map(|_| app.spawn((GlobalTransform::IDENTITY, Survey::of(Spot))))
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
        let agent = app.spawn((GlobalTransform::IDENTITY, SurveyTrace::default()));
        survey(&mut app, agent, Spot, spot(5.0));
        let trace = app.world().get::<SurveyTrace>(agent).unwrap();
        assert_eq!(trace.items.len(), 4);
        assert_eq!(
            trace.items.iter().filter(|weighed| weighed.picked).count(),
            1
        );
        assert_eq!(trace.query, "Spot");
    }

    struct Missing;

    #[test]
    fn a_survey_for_a_query_never_added_fails_rather_than_hang() {
        let mut app = app(Space::Volume);
        let agent = app.spawn((
            GlobalTransform::IDENTITY,
            BehaviorTree::new(behavior_tree! { task(Survey::of(Missing)) }),
        ));
        app.ticks(2);
        let outcome = app.world().get::<BehaviorTree>(agent).unwrap().outcome();
        assert_eq!(outcome, Some(Outcome::Failure));
        let direct = app.spawn((GlobalTransform::IDENTITY, Survey::of(Missing)));
        app.tick();
        assert!(
            app.world().get::<Survey<Missing>>(direct).is_none(),
            "taken off"
        );
    }

    #[test]
    fn a_survey_asked_for_before_its_query_is_added_still_runs() {
        let mut app = app(Space::Volume);
        let agent = app.spawn((GlobalTransform::IDENTITY, Survey::of(Spot)));
        app.app_mut().add_env_query(Spot, spot(5.0));
        app.tick();
        assert!(app.world().get::<Found<Spot>>(agent).is_some());
    }
}
