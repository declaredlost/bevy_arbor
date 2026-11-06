use bevy::ecs::intern::Interned;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;

use crate::tree::{self, TreeTrace};

/// Where trees tick.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArborSystems;

/// Ticks every [`BehaviourTree`](crate::BehaviourTree).
pub struct ArborPlugin {
    schedule: Interned<dyn ScheduleLabel>,
}

impl ArborPlugin {
    /// Ticks trees in `schedule`.
    pub fn new(schedule: impl ScheduleLabel) -> Self {
        Self {
            schedule: schedule.intern(),
        }
    }
}

impl Default for ArborPlugin {
    /// Ticks trees in `FixedUpdate`.
    fn default() -> Self {
        Self::new(FixedUpdate)
    }
}

impl Plugin for ArborPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<TreeTrace>()
            .add_systems(self.schedule, tree::tick.in_set(ArborSystems));
    }
}
