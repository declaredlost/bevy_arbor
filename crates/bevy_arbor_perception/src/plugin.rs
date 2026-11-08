use bevy::ecs::schedule::{InternedScheduleLabel, ScheduleLabel};
use bevy::prelude::*;

use crate::perceived::{Attitude, Team};
use crate::sense::{
    self, Facing, Hearing, Noise, Perceivable, PerceptionSchedule, SensePlugin, SensorOf, Sensors,
    Share, Shared, Sight, Touch, Touched,
};
use crate::space::{Grid, Space};

/// The stages perception runs in, in order. Add a system of your own to one to
/// take part: a raycast in `Occlude`, an attitude in `Classify`.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PerceptionSystems {
    /// Agents look, and noises, touches and reports come in: candidates.
    Gather,
    /// Candidates something's in the way of are blocked. Nothing built in.
    Occlude,
    /// Candidates are given their attitude. Nothing built in: what's left gets
    /// the [`Team`] fallback.
    Classify,
    /// What's left is written to each agent's
    /// [`Perceived<S>`](crate::perceived::Perceived), and what's faded is
    /// forgotten.
    Commit,
}

/// Perception, in `FixedUpdate` and 3D unless told otherwise.
pub struct PerceptionPlugin {
    schedule: InternedScheduleLabel,
    space: Space,
}

impl PerceptionPlugin {
    /// Perception in `schedule`.
    pub fn new(schedule: impl ScheduleLabel) -> Self {
        Self {
            schedule: schedule.intern(),
            space: Space::default(),
        }
    }

    /// The schedule perception runs in.
    #[must_use]
    pub fn schedule(&self) -> InternedScheduleLabel {
        self.schedule
    }

    /// Perception in `space`: [`Space::Planar`] for 2D.
    #[must_use]
    pub fn with_space(mut self, space: Space) -> Self {
        self.space = space;
        self
    }
}

impl Default for PerceptionPlugin {
    /// Perception in `FixedUpdate`, in 3D.
    fn default() -> Self {
        Self::new(FixedUpdate)
    }
}

impl Plugin for PerceptionPlugin {
    fn build(&self, app: &mut App) {
        use PerceptionSystems::{Classify, Commit, Gather, Occlude};

        app.insert_resource(self.space)
            .insert_resource(PerceptionSchedule(self.schedule))
            .init_resource::<Grid>()
            .add_message::<Noise>()
            .add_message::<Touched>()
            .add_message::<Share>()
            .register_type::<Space>()
            .register_type::<Perceivable>()
            .register_type::<Facing>()
            .register_type::<SensorOf>()
            .register_type::<Sensors>()
            .register_type::<Team>()
            .register_type::<Attitude>()
            .register_type::<Sight>()
            .register_type::<Hearing>()
            .register_type::<Touch>()
            .register_type::<Shared>()
            .configure_sets(self.schedule, (Gather, Occlude, Classify, Commit).chain())
            .add_systems(
                self.schedule,
                (
                    (sense::build_grid, sense::look::<sense::GridIndex>)
                        .chain()
                        .run_if(not(resource_exists::<sense::OwnSightIndex>)),
                    sense::hear,
                    sense::feel,
                )
                    .in_set(Gather),
            )
            .add_plugins((
                SensePlugin::<Sight>::default(),
                SensePlugin::<Hearing>::default(),
                SensePlugin::<Touch>::default(),
                SensePlugin::<Shared>::default(),
            ));
    }
}
