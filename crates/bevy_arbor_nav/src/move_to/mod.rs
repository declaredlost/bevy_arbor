//! [`MoveTo`]: a behavior tree task asking for an agent to be somewhere.

use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;
use bevy_arbor_bt::task::{TaskCommands, TaskOf, Tasks};

use crate::travel::{Destination, Goal, Route, Travel};

/// A task: get the agent to a place. As it starts, the agent's
/// [`Destination`] is set and its old [`Travel`] cleared; it succeeds once
/// your movement says [`Travel::Arrived`], and fails once it says
/// [`Travel::Unreachable`]. However it ends, cancelled too, the destination
/// is taken off, so the agent stops.
///
/// Asked to stop, it stops at once, unless it's `Graceful`: then it carries
/// on until it arrives, or can't get there, as a move you'd rather finish.
/// If it's going to an entity that's despawned, it fails.
///
/// `task(MoveTo::point(post))`, `task(MoveTo::entity(leader))`, or, with the
/// `eqs` feature, `task(MoveTo::found::<Cover>())`: where the query keyed by
/// `Cover` found.
#[derive(Component, Debug, Clone, Copy)]
pub struct MoveTo {
    to: To,
    within: f32,
}

/// Where a [`MoveTo`] goes, found as it starts.
#[derive(Debug, Clone, Copy)]
enum To {
    Goal(Goal),
    /// Read off the agent: what a query found.
    #[cfg(feature = "eqs")]
    Found(fn(EntityRef) -> Option<Goal>),
}

impl MoveTo {
    /// To a point.
    #[must_use]
    pub fn point(point: Vec3) -> Self {
        Self::to(To::Goal(Goal::Point(point)))
    }

    /// To an entity, wherever it goes.
    #[must_use]
    pub fn entity(entity: Entity) -> Self {
        Self::to(To::Goal(Goal::Entity(entity)))
    }

    /// To what the environment query keyed by `Q` found for the agent: the
    /// thing, or the place. It fails if the query found nothing.
    #[cfg(feature = "eqs")]
    #[cfg_attr(docsrs, doc(cfg(feature = "eqs")))]
    #[must_use]
    pub fn found<Q: Send + Sync + 'static>() -> Self {
        Self::to(To::Found(|agent| {
            let found = agent.get::<bevy_arbor_eqs::survey::Found<Q>>()?;
            found
                .entity()
                .map(Goal::Entity)
                .or_else(|| found.position().map(Goal::Point))
        }))
    }

    /// There when within `distance`: [`Destination::WITHIN`] unless set.
    #[must_use]
    pub fn within(mut self, distance: f32) -> Self {
        self.within = distance;
        self
    }

    fn to(to: To) -> Self {
        Self {
            to,
            within: Destination::WITHIN,
        }
    }
}

/// On a `MoveTo` task: the destination it set, so it takes off only that one,
/// not one set since by another task or by you.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct Asked(Destination);

/// A `MoveTo` task starting sets its agent's destination, and clears how the
/// last went: an old `Arrived` isn't this one's. One with nowhere to go
/// fails. An observer, so it reads what a query found without holding every
/// component every tick.
pub(crate) fn start(added: On<Add<MoveTo>>, mut world: DeferredWorld) {
    let task = added.entity;
    let (Some(&move_to), Some(of)) = (world.get::<MoveTo>(task), world.get::<TaskOf>(task)) else {
        return;
    };
    let agent = of.agent();
    let goal = match move_to.to {
        To::Goal(goal) => Some(goal),
        #[cfg(feature = "eqs")]
        To::Found(find) => world.get_entity(agent).ok().and_then(find),
    };
    let mut commands = world.commands();
    let Some(goal) = goal else {
        commands.entity(task).fail_task();
        return;
    };
    let destination = Destination {
        goal,
        within: move_to.within,
    };
    commands.entity(task).try_insert(Asked(destination));
    // The agent may be going this tick: then there's nothing to ask.
    if let Ok(mut agent) = commands.get_entity(agent) {
        agent
            .try_remove::<(Travel, Route)>()
            .try_insert(destination);
    }
}

/// Each `MoveTo` task reports as its agent's travel says, and fails if the
/// entity it's going to is gone: nothing could say it's unreachable.
pub(crate) fn follow(
    mut tasks: Tasks<(&MoveTo, Option<&Asked>), Option<&Travel>>,
    places: Query<&GlobalTransform>,
) {
    tasks.for_each(|(_, asked), mut task, travel| {
        let gone = asked.is_some_and(|Asked(to)| match to.goal {
            Goal::Entity(entity) => !places.contains(entity),
            Goal::Point(_) => false,
        });
        match travel {
            Some(Travel::Arrived) => task.succeed(),
            Some(Travel::Unreachable) => task.fail(),
            Some(Travel::Moving) | None if gone => task.fail(),
            Some(Travel::Moving) | None => {}
        }
    });
}

/// A `MoveTo` task ending, however it ends, takes its agent's destination
/// off, if it's still the one it set, with how it went and the way it took:
/// the agent stops.
pub(crate) fn stop(
    ended: On<Remove<MoveTo>>,
    tasks: Query<(&TaskOf, &Asked)>,
    destinations: Query<&Destination>,
    mut commands: Commands,
) {
    let Ok((of, Asked(asked))) = tasks.get(ended.entity) else {
        return;
    };
    let agent = of.agent();
    if destinations.get(agent) == Ok(asked)
        && let Ok(mut agent) = commands.get_entity(agent)
    {
        agent.try_remove::<(Destination, Travel, Route)>();
    }
}

#[cfg(all(test, feature = "eqs"))]
mod tests {
    use bevy::prelude::*;
    use bevy_arbor_bt::prelude::*;
    use bevy_arbor_eqs::prelude::*;

    use crate::prelude::*;
    use crate::testing::*;

    #[test]
    fn it_asks_for_the_destination_and_succeeds_on_arrival() {
        let mut app = app();
        let agent = agent(
            &mut app,
            behavior_tree! { task(MoveTo::point(Vec3::X).within(2.0)) },
        );
        app.tick();
        assert_eq!(
            app.world().get::<Destination>(agent),
            Some(&Destination::point(Vec3::X).within(2.0))
        );
        app.entity_mut(agent).insert(Travel::Moving);
        app.ticks(2);
        assert_eq!(outcome(&app, agent), None, "on its way");
        app.entity_mut(agent).insert(Travel::Arrived);
        app.ticks(2);
        assert_eq!(outcome(&app, agent), Some(true));
        assert!(app.world().get::<Destination>(agent).is_none(), "taken off");
    }

    #[test]
    fn it_fails_when_there_is_no_way() {
        let mut app = app();
        let agent = agent(&mut app, behavior_tree! { task(MoveTo::point(Vec3::X)) });
        app.tick();
        app.entity_mut(agent).insert(Travel::Unreachable);
        app.ticks(2);
        assert_eq!(outcome(&app, agent), Some(false));
    }

    #[test]
    fn an_old_arrival_is_not_this_ones() {
        let mut app = app();
        let agent = agent(&mut app, behavior_tree! { task(MoveTo::point(Vec3::X)) });
        app.entity_mut(agent).insert(Travel::Arrived);
        app.ticks(3);
        assert_eq!(outcome(&app, agent), None, "still going");
        assert!(app.world().get::<Travel>(agent).is_none(), "cleared");
    }

    #[test]
    fn cancelled_it_takes_the_destination_off() {
        let mut app = app();
        let agent = agent(
            &mut app,
            behavior_tree! {
                Select => {
                    when(alarmed) => task(Hold),
                    task(MoveTo::point(Vec3::X)),
                }
            },
        );
        app.tick();
        assert!(app.world().get::<Destination>(agent).is_some());
        app.entity_mut(agent).insert(Alarmed);
        app.ticks(2);
        assert!(app.world().get::<Destination>(agent).is_none(), "stopped");
    }

    #[test]
    fn ending_it_leaves_a_destination_set_since_by_someone_else() {
        let mut app = app();
        let agent = agent(
            &mut app,
            behavior_tree! {
                Select => {
                    when(alarmed) => task(Hold),
                    task(MoveTo::point(Vec3::X)),
                }
            },
        );
        app.tick();
        app.entity_mut(agent).insert(Destination::point(Vec3::Y));
        app.entity_mut(agent).insert(Alarmed);
        app.ticks(2);
        assert_eq!(
            app.world().get::<Destination>(agent),
            Some(&Destination::point(Vec3::Y)),
            "not its own: left"
        );
    }

    #[test]
    fn it_takes_its_route_and_travel_off_as_it_ends() {
        let mut app = app();
        let agent = agent(&mut app, behavior_tree! { task(MoveTo::point(Vec3::X)) });
        app.tick();
        app.entity_mut(agent)
            .insert((Travel::Arrived, Route(vec![Vec3::X])));
        app.ticks(2);
        assert_eq!(outcome(&app, agent), Some(true));
        assert!(app.world().get::<Route>(agent).is_none());
        assert!(app.world().get::<Travel>(agent).is_none());
    }

    #[test]
    fn it_fails_when_the_entity_it_goes_to_is_gone() {
        let mut app = app();
        let leader = app.spawn(GlobalTransform::IDENTITY);
        let agent = agent(&mut app, behavior_tree! { task(MoveTo::entity(leader)) });
        app.tick();
        app.entity_mut(agent).insert(Travel::Moving);
        app.ticks(2);
        assert_eq!(outcome(&app, agent), None, "following");
        app.entity_mut(leader).despawn();
        app.ticks(2);
        assert_eq!(outcome(&app, agent), Some(false));
    }

    #[test]
    fn it_goes_to_what_a_query_found_and_fails_if_nothing_was() {
        let mut app = app();
        let agent = agent(&mut app, behavior_tree! { task(MoveTo::found::<Spot>()) });
        app.ticks(2);
        assert_eq!(outcome(&app, agent), Some(false), "nothing found yet");

        let mut app = self::app();
        let agent = app.spawn((
            GlobalTransform::IDENTITY,
            BehaviorTree::new(behavior_tree! {
                Sequence => { task(Survey::of(Spot)), task(MoveTo::found::<Spot>()) }
            }),
        ));
        app.ticks(4);
        let goal = app.world().get::<Destination>(agent).map(|to| to.goal);
        assert_eq!(goal, Some(Goal::Point(SPOT)), "the found place");
    }
}
