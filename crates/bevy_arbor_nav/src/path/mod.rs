//! Path costs in environment queries: your pathfinder, set once with
//! [`set_path_costs`](PathCostsApp::set_path_costs), measured by
//! [`PathDistance`].

use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use bevy::ecs::system::ReadOnlySystem;
use bevy::ecs::world::WorldId;
use bevy::prelude::*;
use bevy_arbor_eqs::query::{Item, QueryContext};
use bevy_arbor_eqs::test::{Batch, Cost, Measure, Test};

/// Your pathfinder, as a test: what [`PathDistance`] runs, and which one it
/// is, so a `PathDistance` sees it replaced.
#[derive(Resource)]
struct PathCosts {
    measure: Measure,
    set: u64,
}

/// Which pathfinder each [`set_path_costs`](PathCostsApp::set_path_costs)
/// set, counting up.
static SET: AtomicU64 = AtomicU64::new(0);

/// Setting the app's pathfinder for [`PathDistance`].
pub trait PathCostsApp {
    /// Measures path costs with `system`: given the agent and the items, the
    /// cost of the way from the agent to each, or `None` where there's none.
    /// Read-only, and asked for a whole batch at once, so a graph search
    /// from the agent answers every item. Any pathfinding: a navmesh, a
    /// grid, a platformer's jump graph.
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
        self.insert_resource(PathCosts {
            measure: Measure::by(system),
            set: SET.fetch_add(1, Ordering::Relaxed),
        })
    }
}

/// How far the way from the agent to an item is, as the app's pathfinder
/// says: no answer where there's no way, so an unreachable item fails a
/// filter on it. Set the pathfinder with
/// [`set_path_costs`](PathCostsApp::set_path_costs); without one, it warns
/// and has no answers.
///
/// Costly: put cheap filters first, a straight-line
/// [`Distance`](bevy_arbor_eqs::test::Distance) to the agent, so it measures
/// fewer items.
#[derive(Default)]
pub struct PathDistance {
    /// The pathfinder it measures with: for which world, which one it is.
    costs: Mutex<Option<(WorldId, u64, Measure)>>,
}

impl PathDistance {
    /// The way from the agent to each item.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Test for PathDistance {
    fn test(&self, cx: &QueryContext, item: &Item) -> Option<f32> {
        self.test_all(cx, std::slice::from_ref(item))
            .into_iter()
            .next()
            .flatten()
    }

    fn test_all(&self, cx: &QueryContext, items: &[Item]) -> Vec<Option<f32>> {
        self.costs
            .lock()
            .ok()
            .and_then(|costs| {
                costs
                    .as_ref()
                    .filter(|(world, ..)| *world == cx.world().id())
                    .map(|(.., measure)| measure.test_all(cx, items))
            })
            .unwrap_or_else(|| vec![None; items.len()])
    }

    fn cost(&self) -> Cost {
        Cost::High
    }

    fn prepare(&self, world: &mut World) {
        let Ok(mut costs) = self.costs.lock() else {
            return;
        };
        let Some(set) = world.get_resource::<PathCosts>() else {
            warn_once!("`PathDistance` without a pathfinder: set one with `set_path_costs`");
            *costs = None;
            return;
        };
        // This world's pathfinder, as it is now: another world's, or one
        // replaced since, measures wrong.
        let current = costs
            .as_ref()
            .is_some_and(|(id, which, _)| *id == world.id() && *which == set.set);
        if !current {
            *costs = Some((world.id(), set.set, set.measure.clone()));
        }
        if let Some((.., measure)) = costs.as_ref() {
            measure.prepare(world);
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use bevy_arbor_eqs::prelude::*;
    use bevy_arbor_eqs::query::{Item, QueryContext};
    use bevy_arbor_eqs::test::Test;
    use bevy_arbor_perception::prelude::*;

    use crate::prelude::*;
    use crate::testing::*;

    /// A pathfinder over a wall at x = 5: what's past it is unreachable, the
    /// rest as far as it is.
    fn walled(batch: Batch, places: Query<&GlobalTransform>) -> Vec<Option<f32>> {
        let from = places
            .get(batch.agent)
            .map_or(Vec3::ZERO, GlobalTransform::translation);
        batch
            .items
            .iter()
            .map(|item| (item.position().x < 5.0).then(|| from.distance(item.position())))
            .collect()
    }

    fn measured(app: &mut bevy_harness::TestApp, items: &[Item]) -> Vec<Option<f32>> {
        let agent = app.spawn(GlobalTransform::IDENTITY);
        let test = PathDistance::new();
        test.prepare(app.world_mut());
        let cx = QueryContext::new(app.world(), agent, Space::Volume, 1);
        test.test_all(&cx, items)
    }

    #[test]
    fn it_is_the_app_pathfinders_cost_and_none_where_there_is_no_way() {
        let mut app = app();
        app.app_mut().set_path_costs(walled);
        let items = [Item::Point(Vec3::X * 2.0), Item::Point(Vec3::X * 8.0)];
        assert_eq!(measured(&mut app, &items), [Some(2.0), None]);
    }

    #[test]
    fn a_pathfinder_set_anew_is_the_one_it_measures_with() {
        fn doubled(batch: Batch) -> Vec<Option<f32>> {
            batch
                .items
                .iter()
                .map(|item| Some(item.position().x * 2.0))
                .collect()
        }
        let mut app = app();
        app.app_mut().set_path_costs(walled);
        let agent = app.spawn(GlobalTransform::IDENTITY);
        let test = PathDistance::new();
        let items = [Item::Point(Vec3::X * 2.0)];
        let measured = |app: &mut bevy_harness::TestApp| {
            test.prepare(app.world_mut());
            let cx = QueryContext::new(app.world(), agent, Space::Volume, 1);
            test.test_all(&cx, &items)
        };
        assert_eq!(measured(&mut app), [Some(2.0)]);
        app.app_mut().set_path_costs(doubled);
        assert_eq!(measured(&mut app), [Some(4.0)], "the new one");
    }

    #[test]
    fn without_a_pathfinder_it_has_no_answers() {
        let mut app = app();
        assert_eq!(measured(&mut app, &[Item::Point(Vec3::X)]), [None]);
    }

    #[test]
    fn a_query_drops_what_it_cannot_reach() {
        let mut app = app();
        app.app_mut().set_path_costs(walled).add_env_query(
            Spot,
            EnvQuery::new(
                Grid::around(At(Vec3::new(5.0, 0.0, 0.0)))
                    .extent(4.0)
                    .spacing(2.0),
            )
            .filter(PathDistance::new(), Filter::at_most(100.0))
            .pick(Pick::All),
        );
        let agent = app.spawn((GlobalTransform::IDENTITY, Survey::of(Spot)));
        app.ticks(2);
        let found = app.world().get::<Found<Spot>>(agent).expect("found");
        assert!(found.iter().count() > 0);
        assert!(found.iter().all(|scored| scored.item.position().x < 5.0));
    }
}
