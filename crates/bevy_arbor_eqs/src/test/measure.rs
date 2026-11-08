use std::sync::{Arc, Mutex};

use bevy::ecs::system::{BoxedReadOnlySystem, ReadOnlySystem, SystemInput};
use bevy::ecs::world::WorldId;
use bevy::prelude::*;

use super::{Cost, Test};
use crate::query::{Item, QueryContext};

/// What a [`Measure`]'s system is asked about: the agent running the query,
/// and every item to measure. A system takes it as its first parameter, as
/// an observer takes `On<E>`.
#[derive(Debug, Clone, Copy)]
pub struct Batch<'a> {
    /// The agent running the query.
    pub agent: Entity,
    /// The items, in order: the system answers each, in the same order.
    pub items: &'a [Item],
}

impl SystemInput for Batch<'_> {
    type Inner<'i> = Batch<'i>;
    type Param<'i> = Batch<'i>;

    fn wrap(this: Self::Inner<'_>) -> Self::Param<'_> {
        this
    }
}

/// What a [`Measure`] runs: a read-only system answering a [`Batch`].
type Measuring = BoxedReadOnlySystem<Batch<'static>, Vec<Option<f32>>>;

/// A test whose numbers come from a system of yours: the game's own
/// pathfinder, its territory, its lights. Made by [`Measure::by`].
///
/// The system is read-only and answers a whole [`Batch`] at once, one number
/// or `None` an item, in order, so what's costly to set up is set up once a
/// query: a single search from the agent to every item. An answer of the
/// wrong length, or a system that can't run (a resource missing), is no
/// answer for any item.
pub struct Measure {
    make: Arc<dyn Fn() -> Measuring + Send + Sync>,
    cost: Cost,
    kept: Mutex<Option<(WorldId, Measuring)>>,
}

impl Measure {
    /// A test answered by `system`: `fn(batch: Batch, ..) -> Vec<Option<f32>>`,
    /// with any read-only system parameters after the batch.
    #[must_use]
    pub fn by<S, M>(system: S) -> Self
    where
        S: IntoSystem<Batch<'static>, Vec<Option<f32>>, M> + Clone + Send + Sync + 'static,
        S::System: ReadOnlySystem,
    {
        Self {
            make: Arc::new(move || Box::new(IntoSystem::into_system(system.clone())) as Measuring),
            cost: Cost::High,
            kept: Mutex::new(None),
        }
    }

    /// How costly it is, [`Cost::High`] unless set: a cheap one runs before
    /// others.
    #[must_use]
    pub fn costing(mut self, cost: Cost) -> Self {
        self.cost = cost;
        self
    }
}

impl Clone for Measure {
    /// The same system, set up afresh: each clone keeps its own state.
    fn clone(&self) -> Self {
        Self {
            make: Arc::clone(&self.make),
            cost: self.cost,
            kept: Mutex::new(None),
        }
    }
}

impl Test for Measure {
    fn test(&self, cx: &QueryContext, item: &Item) -> Option<f32> {
        self.test_all(cx, std::slice::from_ref(item))
            .into_iter()
            .next()
            .flatten()
    }

    fn test_all(&self, cx: &QueryContext, items: &[Item]) -> Vec<Option<f32>> {
        let none = || vec![None; items.len()];
        let Ok(mut kept) = self.kept.lock() else {
            return none();
        };
        let Some((_, system)) = kept.as_mut().filter(|(world, _)| *world == cx.world().id()) else {
            warn_once!("a `Measure` asked before it was prepared for this world: no answers");
            return none();
        };
        let batch = Batch {
            agent: cx.agent(),
            items,
        };
        match system.run_readonly(batch, cx.world()) {
            Ok(answers) if answers.len() == items.len() => answers,
            _ => none(),
        }
    }

    fn cost(&self) -> Cost {
        self.cost
    }

    fn prepare(&self, world: &mut World) {
        if let Ok(mut kept) = self.kept.lock()
            && kept.as_ref().is_none_or(|(id, _)| *id != world.id())
        {
            // Built for this world: one built for another would fail it.
            let mut system = (self.make)();
            system.initialize(world);
            *kept = Some((world.id(), system));
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use bevy_arbor_perception::prelude::*;

    use super::{Batch, Measure};
    use crate::query::{Item, QueryContext};
    use crate::test::Test;
    use crate::testing::*;

    #[derive(Resource)]
    struct Scale(f32);

    /// Each item's x, scaled: a system reading the world.
    fn scaled(batch: Batch, scale: Res<Scale>) -> Vec<Option<f32>> {
        batch
            .items
            .iter()
            .map(|item| Some(item.position().x * scale.0))
            .collect()
    }

    fn measured(
        app: &mut bevy_harness::TestApp,
        test: &Measure,
        items: &[Item],
    ) -> Vec<Option<f32>> {
        let agent = app.spawn(GlobalTransform::IDENTITY);
        test.prepare(app.world_mut());
        let cx = QueryContext::new(app.world(), agent, Space::Volume, 1);
        test.test_all(&cx, items)
    }

    #[test]
    fn a_system_answers_every_item_at_once() {
        let mut app = app(Space::Volume);
        app.world_mut().insert_resource(Scale(2.0));
        let items = [Item::Point(Vec3::X), Item::Point(Vec3::X * 3.0)];
        assert_eq!(
            measured(&mut app, &Measure::by(scaled), &items),
            [Some(2.0), Some(6.0)]
        );
    }

    #[test]
    fn a_system_that_cannot_run_is_no_answer() {
        let mut app = app(Space::Volume);
        let items = [Item::Point(Vec3::X)];
        assert_eq!(
            measured(&mut app, &Measure::by(scaled), &items),
            [None],
            "no Scale"
        );
    }

    #[test]
    fn an_answer_of_the_wrong_length_is_no_answer() {
        fn short(_: Batch) -> Vec<Option<f32>> {
            vec![Some(1.0)]
        }
        let mut app = app(Space::Volume);
        let items = [Item::Point(Vec3::X), Item::Point(Vec3::Y)];
        assert_eq!(
            measured(&mut app, &Measure::by(short), &items),
            [None, None]
        );
    }

    #[test]
    fn one_test_runs_in_two_worlds() {
        let test = Measure::by(scaled);
        for scale in [1.0, 5.0] {
            let mut app = app(Space::Volume);
            app.world_mut().insert_resource(Scale(scale));
            assert_eq!(
                measured(&mut app, &test, &[Item::Point(Vec3::X)]),
                [Some(scale)]
            );
        }
    }
}
