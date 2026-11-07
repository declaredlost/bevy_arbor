use bevy::prelude::*;

use crate::behaviour::Outcome;

/// Triggered on an agent as its tree ends. One under `Forever` never does.
#[derive(EntityEvent, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeFinished {
    /// The agent whose tree ended.
    pub entity: Entity,
    /// How it ended.
    pub outcome: Outcome,
}
