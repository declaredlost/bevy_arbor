use std::time::Duration;

use bevy::ecs::schedule::{InternedScheduleLabel, ScheduleLabel};
use bevy::prelude::*;
use bevy_arbor_bt::TreeSystems;
use bevy_arbor_perception::Space;

use crate::survey::{self, EnvQueries, Pending, Seed, SurveyTrace};

/// The system set queries run in: after trees tick, so a query a tree asks
/// for is run the same tick.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EqsSystems;

/// How long queries may run each tick, all together: what's left waits for
/// the next, oldest first. At least one runs each tick, however long.
#[derive(Resource, Reflect, Debug, Clone, Copy, PartialEq, Eq)]
#[reflect(Resource, Debug, Default, PartialEq)]
pub struct EqsBudget(pub Duration);

impl Default for EqsBudget {
    /// A millisecond.
    fn default() -> Self {
        Self(Duration::from_millis(1))
    }
}

/// Runs queries asked for with [`Survey`](crate::survey::Survey), in
/// `FixedUpdate` unless told otherwise. Add queries with
/// [`add_env_query`](crate::survey::EnvQueryApp::add_env_query).
pub struct EqsPlugin {
    schedule: InternedScheduleLabel,
}

impl EqsPlugin {
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

impl Default for EqsPlugin {
    /// Runs queries in `FixedUpdate`.
    fn default() -> Self {
        Self::new(FixedUpdate)
    }
}

impl Plugin for EqsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EqsBudget>()
            .init_resource::<EnvQueries>()
            .init_resource::<Pending>()
            .init_resource::<Seed>()
            // Perception's, if it's there; 3D if not.
            .init_resource::<Space>()
            .register_type::<EqsBudget>()
            .register_type::<SurveyTrace>()
            .configure_sets(self.schedule, EqsSystems.after(TreeSystems))
            .add_systems(self.schedule, survey::run.in_set(EqsSystems));
    }
}
