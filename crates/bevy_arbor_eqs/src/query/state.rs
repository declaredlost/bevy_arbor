//! What each query keeps in a world between runs, as Bevy keeps a system's.
//!
//! Its tests' and generators' own state.

use std::any::Any;
use std::sync::Arc;

use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use super::{EnvQueryContext, EnvQueryItem, Parts};
use crate::generate::Generate;
use crate::test::{Cost, Test};

/// A test's or generator's state, without its type.
pub(crate) type State = Box<dyn Any + Send + Sync>;

/// A [`Test`], without its state's type, as a query keeps it.
pub(crate) trait AnyTest: Send + Sync + 'static {
    fn cost(&self) -> Cost;
    fn init(&self, world: &mut World) -> State;
    fn prepare(&self, state: &mut State, world: &mut World);
    fn test(
        &self,
        state: &mut State,
        cx: &EnvQueryContext,
        items: &[EnvQueryItem],
    ) -> Vec<Option<f32>>;
}

impl<T: Test> AnyTest for T {
    fn cost(&self) -> Cost {
        Test::cost(self)
    }

    fn init(&self, world: &mut World) -> State {
        Box::new(T::State::from_world(world))
    }

    fn prepare(&self, state: &mut State, world: &mut World) {
        Test::prepare(self, own::<T::State>(state), world);
    }

    fn test(
        &self,
        state: &mut State,
        cx: &EnvQueryContext,
        items: &[EnvQueryItem],
    ) -> Vec<Option<f32>> {
        Test::test(self, own::<T::State>(state), cx, items)
    }
}

/// A [`Generate`], without its state's type, as a query keeps it.
pub(crate) trait AnyGenerate: Send + Sync + 'static {
    fn init(&self, world: &mut World) -> State;
    fn prepare(&self, state: &mut State, world: &mut World);
    fn generate(&self, state: &mut State, cx: &EnvQueryContext, out: &mut Vec<EnvQueryItem>);
}

impl<G: Generate> AnyGenerate for G {
    fn init(&self, world: &mut World) -> State {
        Box::new(G::State::from_world(world))
    }

    fn prepare(&self, state: &mut State, world: &mut World) {
        Generate::prepare(self, own::<G::State>(state), world);
    }

    fn generate(&self, state: &mut State, cx: &EnvQueryContext, out: &mut Vec<EnvQueryItem>) {
        Generate::generate(self, own::<G::State>(state), cx, out);
    }
}

/// `state` as the `S` it was made as.
fn own<S: 'static>(state: &mut State) -> &mut S {
    state
        .downcast_mut::<S>()
        .expect("a query's state is the one its own test or generator made")
}

/// One query's state in a world: each generator's, filter's and score's, in
/// order.
pub(crate) struct Prepared {
    pub(crate) generators: Vec<State>,
    pub(crate) filters: Vec<State>,
    pub(crate) scores: Vec<State>,
}

impl Prepared {
    /// Each part of `parts` made its state, in `world`.
    pub(crate) fn of(parts: &Parts, world: &mut World) -> Self {
        Self {
            generators: parts.generators.iter().map(|g| g.init(world)).collect(),
            filters: parts.filters.iter().map(|(t, _)| t.init(world)).collect(),
            scores: parts.scores.iter().map(|(t, _)| t.init(world)).collect(),
        }
    }

    /// Each part ready to run, with the world to change.
    pub(crate) fn prepare(&mut self, parts: &Parts, world: &mut World) {
        for (generator, state) in parts.generators.iter().zip(&mut self.generators) {
            generator.prepare(state, world);
        }
        for ((test, _), state) in parts.filters.iter().zip(&mut self.filters) {
            test.prepare(state, world);
        }
        for ((test, _), state) in parts.scores.iter().zip(&mut self.scores) {
            test.prepare(state, world);
        }
    }
}

/// Every query's state in this world, by the query it's for.
///
/// Each query is kept with its state, so its address isn't another's
/// meanwhile; one no one else holds any more is let go.
#[derive(Resource, Default)]
pub(crate) struct EnvQueryStates(HashMap<usize, (Arc<Parts>, Prepared)>);

impl EnvQueryStates {
    /// Takes `parts`' state out, made if it's new: put it back with
    /// [`Self::put`].
    pub(crate) fn take(world: &mut World, parts: &Arc<Parts>) -> Prepared {
        let key = Arc::as_ptr(parts) as usize;
        let kept = world
            .get_resource_or_init::<Self>()
            .0
            .remove(&key)
            .map(|(_, prepared)| prepared);
        kept.unwrap_or_else(|| Prepared::of(parts, world))
    }

    /// Puts `parts`' state back, letting go of queries no one else holds.
    pub(crate) fn put(world: &mut World, parts: &Arc<Parts>, prepared: Prepared) {
        let mut states = world.get_resource_or_init::<Self>();
        states.0.retain(|_, (kept, _)| Arc::strong_count(kept) > 1);
        states
            .0
            .insert(Arc::as_ptr(parts) as usize, (Arc::clone(parts), prepared));
    }
}
