use std::marker::PhantomData;

use bevy::prelude::*;

use super::Generate;
use crate::anchor::{EnvQueryAnchor, Locate};
use crate::query::{EnvQueryContext, EnvQueryItem};

/// Things with a `C` within `range` of an anchor: a health pack, an ally.
///
/// It looks through every `C`, so keep `C` to what's worth weighing; for what
/// the agent perceives, [`Percepts`](crate::perceived::Percepts) is cheaper.
pub struct Nearby<C: Component> {
    around: EnvQueryAnchor,
    range: f32,
    things: PhantomData<fn() -> C>,
}

impl<C: Component> Nearby<C> {
    /// Things with a `C` within `range` of `around`.
    pub fn within(around: impl Locate, range: f32) -> Self {
        Self {
            around: EnvQueryAnchor::new(around),
            range,
            things: PhantomData,
        }
    }
}

impl<C: Component> Generate for Nearby<C> {
    type State = QueryState<(Entity, &'static GlobalTransform), With<C>>;

    fn generate(
        &self,
        things: &mut Self::State,
        cx: &EnvQueryContext,
        out: &mut Vec<EnvQueryItem>,
    ) {
        let anchors = cx.locate(&self.around);
        if anchors.is_empty() {
            return;
        }
        let agent = cx.querier();
        for (entity, transform) in cx.query(things) {
            if entity == agent {
                continue;
            }
            let position = cx.space().flatten(transform.translation());
            if anchors.iter().any(|at| at.reaches(position, self.range)) {
                out.push(EnvQueryItem::thing(entity, position));
            }
        }
    }
}
