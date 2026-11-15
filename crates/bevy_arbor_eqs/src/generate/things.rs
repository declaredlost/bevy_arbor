use std::marker::PhantomData;
use std::sync::Arc;

use bevy::prelude::*;
use bevy_arbor_perception::perceived::{Perceived, Stimuli};
use bevy_arbor_perception::sense::Sense;

use super::{CachedQuery, Generate};
use crate::anchor::{Locate, Located};
use crate::query::{EnvQueryContext, EnvQueryItem};

/// What the agent perceives by `S`, where each is now, or was last sensed if
/// it's gone.
///
/// Among what's in view, it finds the best target.
pub struct Percepts<S: Sense> {
    active: bool,
    hostile: bool,
    sense: PhantomData<fn() -> S>,
}

impl<S: Sense> Percepts<S> {
    /// Everything perceived or remembered.
    #[must_use]
    pub fn all() -> Self {
        Self {
            active: false,
            hostile: false,
            sense: PhantomData,
        }
    }

    /// Only what's active: in view, for sight.
    #[must_use]
    pub fn active(mut self) -> Self {
        self.active = true;
        self
    }

    /// Only what's hostile.
    #[must_use]
    pub fn hostile(mut self) -> Self {
        self.hostile = true;
        self
    }
}

impl<S: Sense> Generate for Percepts<S> {
    fn generate(&self, cx: &EnvQueryContext, out: &mut Vec<EnvQueryItem>) {
        let Ok(agent) = cx.world().get_entity(cx.agent()) else {
            return;
        };
        let (Some(perceived), Some(stimuli)) =
            (agent.get::<Perceived<S>>(), agent.get::<Stimuli<S>>())
        else {
            return;
        };
        for percept in perceived.iter() {
            if (self.active && !percept.is_active()) || (self.hostile && !percept.is_hostile()) {
                continue;
            }
            let entity = percept.entity();
            // What's in view is where it is; what's only remembered, where it
            // was last sensed, not where it really is now.
            let position = if percept.is_active() {
                cx.position(entity)
            } else {
                stimuli.get(entity).map(|stimulus| stimulus.position)
            };
            if let Some(position) = position {
                out.push(EnvQueryItem::thing(entity, position));
            }
        }
    }
}

/// Things with a `C` within `range` of an anchor: a health pack, an ally.
///
/// It looks through every `C`, so keep `C` to what's worth weighing; for what
/// the agent perceives, [`Percepts`] is cheaper.
pub struct Nearby<C: Component> {
    around: Arc<dyn Locate>,
    range: f32,
    things: CachedQuery<(Entity, &'static GlobalTransform), With<C>>,
}

impl<C: Component> Nearby<C> {
    /// Things with a `C` within `range` of `around`.
    pub fn within(around: impl Locate, range: f32) -> Self {
        Self {
            around: Arc::new(around),
            range,
            things: CachedQuery::default(),
        }
    }
}

impl<C: Component> Generate for Nearby<C> {
    fn generate(&self, cx: &EnvQueryContext, out: &mut Vec<EnvQueryItem>) {
        let anchors = cx.locate(&self.around);
        if anchors.is_empty() {
            return;
        }
        let agent = cx.agent();
        self.things.with(cx.world(), |things| {
            for (entity, transform) in &things {
                if entity == agent {
                    continue;
                }
                let position = cx.space().flatten(transform.translation());
                let near = anchors
                    .iter()
                    .any(|Located { position: at, .. }| at.distance(position) <= self.range);
                if near {
                    out.push(EnvQueryItem::thing(entity, position));
                }
            }
        });
    }
}
