use std::marker::PhantomData;

use bevy_arbor_perception::perceived::Perceived;
use bevy_arbor_perception::sense::Sense;

use super::Which;
use crate::query::{EnvQueryContext, EnvQueryItem};
use crate::test::{Cost, Test};

/// Whether the agent perceives a thing by `S`: 1 or 0, and no answer for a
/// place.
///
/// [`active`](Self::active) asks whether it's active (in view, for sight),
/// [`hostile`](Self::hostile) whether it's hostile too, as [`Percepts`] means.
///
/// [`Percepts`]: super::Percepts
pub struct Perceives<S: Sense> {
    which: Which,
    sense: PhantomData<fn() -> S>,
}

impl<S: Sense> Perceives<S> {
    /// Whether it's perceived or remembered at all.
    #[must_use]
    pub fn all() -> Self {
        Self {
            which: Which::default(),
            sense: PhantomData,
        }
    }

    /// Whether it's active, too.
    #[must_use]
    pub fn active(mut self) -> Self {
        self.which = self.which.active();
        self
    }

    /// Whether it's hostile, too.
    #[must_use]
    pub fn hostile(mut self) -> Self {
        self.which = self.which.hostile();
        self
    }
}

impl<S: Sense> Test for Perceives<S> {
    type State = ();

    fn test(&self, (): &mut (), cx: &EnvQueryContext, items: &[EnvQueryItem]) -> Vec<Option<f32>> {
        let perceived = cx.world().get::<Perceived<S>>(cx.querier());
        items
            .iter()
            .map(|item| {
                let entity = item.entity?;
                let known = perceived
                    .and_then(|perceived| perceived.get(entity))
                    .is_some_and(|percept| self.which.keeps(percept));
                Some(f32::from(u8::from(known)))
            })
            .collect()
    }

    fn cost(&self) -> Cost {
        Cost::Medium
    }
}
