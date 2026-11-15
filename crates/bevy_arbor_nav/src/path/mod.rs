//! Path costs in environment queries: your pathfinder, as [`PathDistance`].
//!
//! Set it once with [`set_path_costs`](PathCostsApp::set_path_costs).

use bevy::ecs::change_detection::Tick;
use bevy::ecs::system::ReadOnlySystem;
use bevy::prelude::*;
use bevy_arbor_eqs::query::{EnvQueryContext, EnvQueryItem};
use bevy_arbor_eqs::test::{Batch, Cost, Measure, Measuring, Test};

/// Your pathfinder, as a test: what [`PathDistance`] runs.
#[derive(Resource)]
struct PathCosts(Measure);

/// Setting the app's pathfinder: the path costs [`PathDistance`] measures by.
pub trait PathCostsApp {
    /// Measures path costs with `system`, a pathfinder of any kind.
    ///
    /// Given the agent and the items, it gives the cost of the way from the
    /// agent to each, or `None` where there's none. Read-only, and asked for a
    /// whole batch at once, so a graph search from the agent answers every
    /// item. Any pathfinding: a navmesh, a grid, a platformer's jump graph.
    fn set_path_costs<S, M>(&mut self, system: S) -> &mut Self
    where
        S: IntoSystem<Batch<'static>, Vec<Option<f32>>, M> + Clone + Send + Sync + 'static,
        S::System: ReadOnlySystem;
}

impl PathCostsApp for App {
    fn set_path_costs<S, M>(&mut self, system: S) -> &mut Self
    where
        S: IntoSystem<Batch<'static>, Vec<Option<f32>>, M> + Clone + Send + Sync + 'static,
        S::System: ReadOnlySystem,
    {
        self.insert_resource(PathCosts(Measure::by(system)))
    }
}

/// How far the way from the agent to an item is, as the app's pathfinder says.
///
/// No answer where there's no way, so an unreachable item fails a filter on
/// it. Set the pathfinder with
/// [`set_path_costs`](PathCostsApp::set_path_costs); without one, it warns
/// and has no answers.
///
/// Costly: put cheap filters first, a straight-line
/// [`Distance`](bevy_arbor_eqs::test::Distance) to the agent, so it measures
/// fewer items.
#[derive(Default)]
pub struct PathDistance;

impl PathDistance {
    /// The way from the agent to each item.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

/// The pathfinder a [`PathDistance`] measures with in a world.
///
/// As it was set then: one set since is taken up as it runs next.
#[doc(hidden)]
#[derive(Default)]
pub struct Paths(Option<(Tick, Measure, Measuring)>);

impl Test for PathDistance {
    type State = Paths;

    fn test(
        &self,
        Paths(paths): &mut Paths,
        cx: &EnvQueryContext,
        items: &[EnvQueryItem],
    ) -> Vec<Option<f32>> {
        match paths {
            Some((_, measure, measuring)) => measure.test(measuring, cx, items),
            None => vec![None; items.len()],
        }
    }

    fn cost(&self) -> Cost {
        Cost::High
    }

    fn prepare(&self, Paths(paths): &mut Paths, world: &mut World) {
        let Some(costs) = world.get_resource_ref::<PathCosts>() else {
            warn_once!("`PathDistance` without a pathfinder: set one with `set_path_costs`");
            *paths = None;
            return;
        };
        // The pathfinder as it is now: one replaced since measures wrong.
        let set = costs.last_changed();
        if paths.as_ref().is_none_or(|(at, ..)| *at != set) {
            *paths = Some((set, costs.0.clone(), Measuring::default()));
        }
        if let Some((_, measure, measuring)) = paths {
            measure.prepare(measuring, world);
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use bevy_arbor_eqs::prelude::*;
    use bevy_arbor_eqs::query::{EnvQueryContext, EnvQueryItem};
    use bevy_arbor_eqs::test::Test;
    use bevy_arbor_perception::prelude::*;

    use super::Paths;
    use crate::prelude::*;
    use crate::testing::*;

    /// A pathfinder over a wall at x = 5: what's past it is unreachable.
    ///
    /// The rest is as far as it is.
    fn walled(batch: Batch, places: Query<&GlobalTransform>) -> Vec<Option<f32>> {
        let from = places
            .get(batch.querier)
            .map_or(Vec3::ZERO, GlobalTransform::translation);
        batch
            .items
            .iter()
            .map(|item| (item.position.x < 5.0).then(|| from.distance(item.position)))
            .collect()
    }

    /// `PathDistance` run for `agent` with `paths`, what it kept from before.
    fn run(
        app: &mut bevy_harness::TestApp,
        agent: Entity,
        paths: &mut Paths,
        items: &[EnvQueryItem],
    ) -> Vec<Option<f32>> {
        let test = PathDistance::new();
        test.prepare(paths, app.world_mut());
        let cx = EnvQueryContext::new(app.world(), agent, Space::Volume, 1);
        test.test(paths, &cx, items)
    }

    fn measured(app: &mut bevy_harness::TestApp, items: &[EnvQueryItem]) -> Vec<Option<f32>> {
        let agent = app.spawn(GlobalTransform::IDENTITY);
        run(app, agent, &mut Paths::default(), items)
    }

    #[test]
    fn it_is_the_app_pathfinders_cost_and_none_where_there_is_no_way() {
        let mut app = app();
        app.app_mut().set_path_costs(walled);
        let items = [
            EnvQueryItem::place(Vec3::X * 2.0),
            EnvQueryItem::place(Vec3::X * 8.0),
        ];
        assert_eq!(measured(&mut app, &items), [Some(2.0), None]);
    }

    #[test]
    fn a_pathfinder_set_anew_is_the_one_it_measures_with() {
        fn doubled(batch: Batch) -> Vec<Option<f32>> {
            batch
                .items
                .iter()
                .map(|item| Some(item.position.x * 2.0))
                .collect()
        }
        let mut app = app();
        app.app_mut().set_path_costs(walled);
        let agent = app.spawn(GlobalTransform::IDENTITY);
        let mut paths = Paths::default();
        let items = [EnvQueryItem::place(Vec3::X * 2.0)];
        assert_eq!(run(&mut app, agent, &mut paths, &items), [Some(2.0)]);
        app.app_mut().set_path_costs(doubled);
        assert_eq!(
            run(&mut app, agent, &mut paths, &items),
            [Some(4.0)],
            "the new one"
        );
    }

    #[test]
    fn without_a_pathfinder_it_has_no_answers() {
        let mut app = app();
        assert_eq!(measured(&mut app, &[EnvQueryItem::place(Vec3::X)]), [None]);
    }

    #[test]
    fn a_query_drops_what_it_cannot_reach() {
        let mut app = app();
        app.app_mut().set_path_costs(walled);
        let reachable = EnvQuery::new(
            Spot,
            Grid::around(At(Vec3::new(5.0, 0.0, 0.0)))
                .extent(4.0)
                .spacing(2.0),
        )
        .filter(PathDistance::new(), Filter::at_most(100.0))
        .pick(Pick::All);
        let agent = app.spawn(GlobalTransform::IDENTITY);
        app.tick();
        let found = reachable.run(app.world_mut(), agent);
        assert_ne!(found.len(), 0);
        assert!(found.iter().all(|scored| scored.item.position.x < 5.0));
    }
}
