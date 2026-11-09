use std::marker::PhantomData;

use bevy::prelude::*;
use bevy_arbor_perception::perceived::{Perceived, Traces};
use bevy_arbor_perception::sense::Sense;

use super::{Locate, Located};
use crate::query::QueryContext;

/// What the agent perceives as hostile by `S`: where each is now, while it's
/// active, or, [`remembered`](Self::remembered), where it was last sensed.
pub struct Hostiles<S: Sense> {
    remembered: bool,
    sense: PhantomData<fn() -> S>,
}

impl<S: Sense> Hostiles<S> {
    /// What's hostile and active, where it is now: in view, for sight.
    #[must_use]
    pub fn active() -> Self {
        Self {
            remembered: false,
            sense: PhantomData,
        }
    }

    /// What's hostile and remembered too, where it was last sensed.
    #[must_use]
    pub fn remembered() -> Self {
        Self {
            remembered: true,
            sense: PhantomData,
        }
    }
}

impl<S: Sense> Locate for Hostiles<S> {
    fn locate(&self, cx: &QueryContext, out: &mut Vec<Located>) {
        let Ok(agent) = cx.world().get_entity(cx.agent()) else {
            return;
        };
        let (Some(perceived), Some(traces)) =
            (agent.get::<Perceived<S>>(), agent.get::<Traces<S>>())
        else {
            return;
        };
        for percept in perceived.iter().filter(|percept| percept.is_hostile()) {
            let entity = percept.entity();
            // What's in view is where it is, looking as it does; what's only
            // remembered, where it was last sensed, looking no way known.
            let (position, ahead) = if percept.is_active() {
                (cx.position(entity), cx.ahead(entity))
            } else if self.remembered {
                (traces.get(entity).map(|trace| trace.position), Vec3::ZERO)
            } else {
                (None, Vec3::ZERO)
            };
            if let Some(position) = position {
                out.push(Located {
                    position,
                    ahead,
                    entity: Some(entity),
                });
            }
        }
    }
}
