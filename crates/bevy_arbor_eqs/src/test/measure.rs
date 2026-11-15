use std::sync::Arc;

use bevy::ecs::system::{BoxedReadOnlySystem, ReadOnlySystem, SystemInput};
use bevy::prelude::*;

use super::{Cost, Test};
use crate::query::{EnvQueryContext, EnvQueryItem};

/// What a [`Measure`]'s system is asked: the querier, and every item.
///
/// A system takes it as its first parameter, as an observer takes `On<E>`.
#[derive(Debug, Clone, Copy)]
pub struct Batch<'a> {
    /// The agent running the query.
    pub querier: Entity,
    /// The items, in order: the system answers each, in the same order.
    pub items: &'a [EnvQueryItem],
}

impl SystemInput for Batch<'_> {
    type Inner<'i> = Batch<'i>;
    type Param<'i> = Batch<'i>;

    fn wrap(this: Self::Inner<'_>) -> Self::Param<'_> {
        this
    }
}

/// A test whose numbers come from a system of yours, made by
/// [`Measure::by`].
///
/// The system can ask the game's own pathfinder, its territory, its lights.
/// It's read-only and answers a whole [`Batch`] at once, one number
/// or `None` an item, in order, so what's costly to set up is set up once a
/// query: a single search from the agent to every item. An answer of the
/// wrong length, or a system that can't run (a resource missing), is no
/// answer for any item.
#[derive(Clone)]
pub struct Measure {
    make: Arc<dyn Fn() -> System + Send + Sync>,
    cost: Cost,
}

impl Measure {
    /// A test answered by `system`, given a [`Batch`], then any read-only
    /// system parameters.
    ///
    /// It's a `fn(batch: Batch, ..) -> Vec<Option<f32>>`.
    #[must_use]
    pub fn by<S, M>(system: S) -> Self
    where
        S: IntoSystem<Batch<'static>, Vec<Option<f32>>, M> + Clone + Send + Sync + 'static,
        S::System: ReadOnlySystem,
    {
        Self {
            make: Arc::new(move || Box::new(IntoSystem::into_system(system.clone())) as System),
            cost: Cost::High,
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

/// A [`Measure`]'s system, in the world it runs in: made as it's first
/// prepared there.
#[doc(hidden)]
#[derive(Default)]
pub struct Measuring(Option<System>);

/// What a [`Measure`] runs: a read-only system answering a [`Batch`].
type System = BoxedReadOnlySystem<Batch<'static>, Vec<Option<f32>>>;

impl Test for Measure {
    type State = Measuring;

    fn test(
        &self,
        Measuring(system): &mut Measuring,
        cx: &EnvQueryContext,
        items: &[EnvQueryItem],
    ) -> Vec<Option<f32>> {
        let none = || vec![None; items.len()];
        let Some(system) = system else {
            return none();
        };
        let batch = Batch {
            querier: cx.querier(),
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

    fn prepare(&self, Measuring(system): &mut Measuring, world: &mut World) {
        if system.is_none() {
            let mut made = (self.make)();
            made.initialize(world);
            *system = Some(made);
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use bevy_arbor_perception::prelude::*;

    use super::{Batch, Measure};
    use crate::query::EnvQueryItem;
    use crate::testing::*;

    #[derive(Resource)]
    struct Scale(f32);

    /// Each item's x, scaled: a system reading the world.
    fn scaled(batch: Batch, scale: Res<Scale>) -> Vec<Option<f32>> {
        batch
            .items
            .iter()
            .map(|item| Some(item.position.x * scale.0))
            .collect()
    }

    fn measured(
        app: &mut bevy_harness::TestApp,
        test: &Measure,
        items: &[EnvQueryItem],
    ) -> Vec<Option<f32>> {
        let agent = app.spawn(GlobalTransform::IDENTITY);
        tested(app.world_mut(), agent, test, items)
    }

    #[test]
    fn a_system_answers_every_item_at_once() {
        let mut app = app(Space::Volume);
        app.world_mut().insert_resource(Scale(2.0));
        let items = [
            EnvQueryItem::place(Vec3::X),
            EnvQueryItem::place(Vec3::X * 3.0),
        ];
        assert_eq!(
            measured(&mut app, &Measure::by(scaled), &items),
            [Some(2.0), Some(6.0)]
        );
    }

    #[test]
    fn a_system_that_cannot_run_is_no_answer() {
        let mut app = app(Space::Volume);
        let items = [EnvQueryItem::place(Vec3::X)];
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
        let items = [EnvQueryItem::place(Vec3::X), EnvQueryItem::place(Vec3::Y)];
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
                measured(&mut app, &test, &[EnvQueryItem::place(Vec3::X)]),
                [Some(scale)]
            );
        }
    }
}
