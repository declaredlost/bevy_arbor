use std::marker::PhantomData;

use bevy::prelude::*;
use bevy_arbor_perception::perceived::{Perceived, Stimuli};
use bevy_arbor_perception::sense::Sense;

use super::Which;
use crate::anchor::{Locate, Located};
use crate::generate::Generate;
use crate::query::{EnvQueryContext, EnvQueryItem};

/// What the agent perceives by `S`: an anchor, or a generator of things.
///
/// Each is where it is now while it's active (in view, for sight), looking as
/// it does; what's only remembered, where it was last sensed, looking no way
/// known. [`active`](Self::active) and [`hostile`](Self::hostile) narrow it:
/// `Percepts::<Sight>::all().hostile()` is every enemy seen, now or before.
pub struct Percepts<S: Sense> {
    which: Which,
    sense: PhantomData<fn() -> S>,
}

impl<S: Sense> Percepts<S> {
    /// Everything perceived or remembered.
    #[must_use]
    pub fn all() -> Self {
        Self {
            which: Which::default(),
            sense: PhantomData,
        }
    }

    /// Only what's active: in view, for sight.
    #[must_use]
    pub fn active(mut self) -> Self {
        self.which = self.which.active();
        self
    }

    /// Only what's hostile.
    #[must_use]
    pub fn hostile(mut self) -> Self {
        self.which = self.which.hostile();
        self
    }

    /// Each one meant, where it is, and which way it looks.
    fn each(&self, cx: &EnvQueryContext, mut found: impl FnMut(Located)) {
        let Ok(agent) = cx.world().get_entity(cx.querier()) else {
            return;
        };
        let (Some(perceived), Some(stimuli)) =
            (agent.get::<Perceived<S>>(), agent.get::<Stimuli<S>>())
        else {
            return;
        };
        for percept in perceived.iter().filter(|percept| self.which.keeps(percept)) {
            let entity = percept.entity();
            // What's in view is where it is, looking as it does; what's only
            // remembered, where it was last sensed, looking no way known.
            let (position, ahead) = if percept.is_active() {
                (cx.position(entity), cx.ahead(entity))
            } else {
                (
                    stimuli.get(entity).map(|stimulus| stimulus.position),
                    Vec3::ZERO,
                )
            };
            if let Some(position) = position {
                found(Located {
                    position,
                    ahead,
                    entity: Some(entity),
                });
            }
        }
    }
}

impl<S: Sense> Locate for Percepts<S> {
    fn locate(&self, cx: &EnvQueryContext, out: &mut Vec<Located>) {
        self.each(cx, |located| out.push(located));
    }
}

impl<S: Sense> Generate for Percepts<S> {
    type State = ();

    fn generate(&self, (): &mut (), cx: &EnvQueryContext, out: &mut Vec<EnvQueryItem>) {
        self.each(cx, |located| {
            if let Some(entity) = located.entity {
                out.push(EnvQueryItem::thing(entity, located.position));
            }
        });
    }
}
