use std::marker::PhantomData;
use std::sync::Arc;

use bevy::prelude::*;
use bevy_arbor_perception::perceived::Perceived;
use bevy_arbor_perception::sense::Sense;

use super::Generate;
use crate::anchor::{Locate, Located};
use crate::query::{Item, QueryContext};

/// What the agent perceives by `S`, where each is now, or was last sensed if
/// it's gone: among what's in view, the best target.
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
    fn generate(&self, cx: &QueryContext, out: &mut Vec<Item>) {
        let Some(perceived) = cx.world().get::<Perceived<S>>(cx.agent()) else {
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
                percept.trace().map(|trace| trace.position)
            };
            if let Some(position) = position {
                out.push(Item::Entity { entity, position });
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
    component: PhantomData<fn() -> C>,
}

impl<C: Component> Nearby<C> {
    /// Things with a `C` within `range` of `around`.
    pub fn within(around: impl Locate, range: f32) -> Self {
        Self {
            around: Arc::new(around),
            range,
            component: PhantomData,
        }
    }
}

impl<C: Component> Generate for Nearby<C> {
    fn generate(&self, cx: &QueryContext, out: &mut Vec<Item>) {
        let anchors = cx.locate(&self.around);
        if anchors.is_empty() {
            return;
        }
        let world = cx.world();
        let Some(mut things) = world.try_query_filtered::<(Entity, &GlobalTransform), With<C>>()
        else {
            return;
        };
        let agent = cx.agent();
        for (entity, transform) in things.iter(world) {
            if entity == agent {
                continue;
            }
            let position = cx.space().flatten(transform.translation());
            let near = anchors
                .iter()
                .any(|Located { position: at, .. }| at.distance(position) <= self.range);
            if near {
                out.push(Item::Entity { entity, position });
            }
        }
    }
}
