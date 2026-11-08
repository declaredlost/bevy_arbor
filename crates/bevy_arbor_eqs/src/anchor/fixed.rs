use bevy::prelude::*;

use super::{Locate, Located};
use crate::query::QueryContext;

/// The agent whose query it is.
#[derive(Debug, Clone, Copy, Default)]
pub struct Agent;

impl Locate for Agent {
    fn locate(&self, cx: &QueryContext, out: &mut Vec<Located>) {
        let agent = cx.agent();
        if let Some(position) = cx.position(agent) {
            out.push(Located {
                position,
                ahead: cx.ahead(agent),
                entity: Some(agent),
            });
        }
    }
}

/// A fixed place, looking nowhere.
#[derive(Debug, Clone, Copy)]
pub struct At(pub Vec3);

impl Locate for At {
    fn locate(&self, cx: &QueryContext, out: &mut Vec<Located>) {
        out.push(Located {
            position: cx.space().flatten(self.0),
            ahead: Vec3::ZERO,
            entity: None,
        });
    }
}

/// A fixed entity, where it is now.
#[derive(Debug, Clone, Copy)]
pub struct Thing(pub Entity);

impl Locate for Thing {
    fn locate(&self, cx: &QueryContext, out: &mut Vec<Located>) {
        if let Some(position) = cx.position(self.0) {
            out.push(Located {
                position,
                ahead: cx.ahead(self.0),
                entity: Some(self.0),
            });
        }
    }
}
