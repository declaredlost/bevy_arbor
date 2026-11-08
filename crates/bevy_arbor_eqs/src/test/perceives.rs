use std::marker::PhantomData;

use bevy_arbor_perception::perceived::Perceived;
use bevy_arbor_perception::sense::Sense;

use super::{Cost, Test};
use crate::query::{Item, QueryContext};

/// Whether the agent perceives a thing by `S`: 1 or 0, and no answer for a
/// place. [`active`](Self::active) asks whether it's active (in view, for
/// sight), [`hostile`](Self::hostile) whether it's hostile too.
pub struct Perceives<S: Sense> {
    active: bool,
    hostile: bool,
    sense: PhantomData<fn() -> S>,
}

impl<S: Sense> Perceives<S> {
    /// Whether it's perceived or remembered at all.
    #[must_use]
    pub fn at_all() -> Self {
        Self {
            active: false,
            hostile: false,
            sense: PhantomData,
        }
    }

    /// Whether it's active, too.
    #[must_use]
    pub fn active(mut self) -> Self {
        self.active = true;
        self
    }

    /// Whether it's hostile, too.
    #[must_use]
    pub fn hostile(mut self) -> Self {
        self.hostile = true;
        self
    }
}

impl<S: Sense> Test for Perceives<S> {
    fn test(&self, cx: &QueryContext, item: &Item) -> Option<f32> {
        let entity = item.entity()?;
        let perceives = cx
            .world()
            .get::<Perceived<S>>(cx.agent())
            .and_then(|perceived| perceived.get(entity))
            .is_some_and(|percept| {
                (!self.active || percept.is_active()) && (!self.hostile || percept.is_hostile())
            });
        Some(f32::from(u8::from(perceives)))
    }

    fn cost(&self) -> Cost {
        Cost::Medium
    }
}
