use std::time::Duration;

use bevy::ecs::schedule::{InternedScheduleLabel, ScheduleLabel};
use bevy::prelude::*;
use bevy_arbor_bt::BehaviorTreeSystems;
use bevy_arbor_perception::Space;

use crate::query::Seed;
use crate::survey::{self, EnvQueryTrace, Pending};

/// The system set queries run in: after trees tick, so a tree's query runs
/// that tick.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EnvQuerySystems;

/// How long queries may run each tick, all together: what's left waits for
/// the next.
///
/// What waits runs oldest first. At least one runs each tick, however long.
#[derive(Resource, Reflect, Debug, Clone, Copy, PartialEq, Eq)]
#[reflect(Resource, Debug, Default, PartialEq)]
pub struct EnvQueryBudget(pub Duration);

impl Default for EnvQueryBudget {
    /// A millisecond.
    fn default() -> Self {
        Self(Duration::from_millis(1))
    }
}

/// Runs queries asked for with [`Survey`](crate::survey::Survey), in
/// `FixedUpdate` unless told otherwise.
///
/// The queries themselves are [`EnvQuery`](crate::query::EnvQuery)
/// components, on the agents and things that carry them.
pub struct EnvQueryPlugin {
    schedule: InternedScheduleLabel,
}

impl EnvQueryPlugin {
    /// Runs queries in `schedule`.
    pub fn new(schedule: impl ScheduleLabel) -> Self {
        Self {
            schedule: schedule.intern(),
        }
    }

    /// The schedule it runs queries in.
    #[must_use]
    pub fn schedule(&self) -> InternedScheduleLabel {
        self.schedule
    }
}

impl Default for EnvQueryPlugin {
    /// Runs queries in `FixedUpdate`.
    fn default() -> Self {
        Self::new(FixedUpdate)
    }
}

impl Plugin for EnvQueryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EnvQueryBudget>()
            .init_resource::<Pending>()
            .init_resource::<Seed>()
            // Perception's, if it's there; 3D if not.
            .init_resource::<Space>()
            .register_type::<EnvQueryBudget>()
            .register_type::<EnvQueryTrace>()
            .configure_sets(self.schedule, EnvQuerySystems.after(BehaviorTreeSystems))
            .add_systems(self.schedule, survey::run.in_set(EnvQuerySystems));
    }
}
