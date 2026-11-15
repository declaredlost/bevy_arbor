use bevy::prelude::*;

use super::{Locate, Located};
use crate::query::EnvQueryContext;

/// The agent whose query it is: where it is now, and which way it looks.
#[derive(Debug, Clone, Copy, Default)]
pub struct Querier;

impl Locate for Querier {
    fn locate(&self, cx: &EnvQueryContext, out: &mut Vec<Located>) {
        let agent = cx.querier();
        if let Some(position) = cx.position(agent) {
            out.push(Located {
                position,
                ahead: cx.ahead(agent),
                entity: Some(agent),
            });
        }
    }
}

/// What carries the query: where it is now, and which way it looks.
///
/// A turret's "where to stand to use me" measures from it. For a query the
/// agent carries itself, it's the agent, as [`Querier`] is.
#[derive(Debug, Clone, Copy, Default)]
pub struct Carrier;

impl Locate for Carrier {
    fn locate(&self, cx: &EnvQueryContext, out: &mut Vec<Located>) {
        Thing(cx.carrier()).locate(cx, out);
    }
}

/// A fixed place, looking nowhere: in a planar space, its z is dropped.
#[derive(Debug, Clone, Copy)]
pub struct At(pub Vec3);

impl Locate for At {
    fn locate(&self, cx: &EnvQueryContext, out: &mut Vec<Located>) {
        out.push(Located {
            position: cx.space().flatten(self.0),
            ahead: Vec3::ZERO,
            entity: None,
        });
    }
}

/// A fixed entity: where it is now, which way it looks, and the thing it
/// is.
#[derive(Debug, Clone, Copy)]
pub struct Thing(pub Entity);

impl Locate for Thing {
    fn locate(&self, cx: &EnvQueryContext, out: &mut Vec<Located>) {
        if let Some(position) = cx.position(self.0) {
            out.push(Located {
                position,
                ahead: cx.ahead(self.0),
                entity: Some(self.0),
            });
        }
    }
}
