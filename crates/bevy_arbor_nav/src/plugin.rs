use bevy::ecs::schedule::{InternedScheduleLabel, ScheduleLabel};
use bevy::prelude::*;
use bevy_arbor_bt::TreeSystems;

use crate::move_to;
use crate::travel::{Destination, Route, Travel};

/// The system set `MoveTo` tasks are kept in: after trees tick, so a task a
/// tree starts asks for its destination the same tick, and after queries,
/// where they run in the same schedule.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NavSystems;

/// Navigation's side of AI: [`MoveTo`](crate::move_to::MoveTo) tasks, in
/// `FixedUpdate` unless told otherwise. It moves nothing: your movement reads
/// each agent's [`Destination`] and writes how it goes as [`Travel`].
pub struct NavPlugin {
    schedule: InternedScheduleLabel,
}

impl NavPlugin {
    /// Keeps `MoveTo` tasks in `schedule`: the trees'.
    pub fn new(schedule: impl ScheduleLabel) -> Self {
        Self {
            schedule: schedule.intern(),
        }
    }

    /// The schedule it keeps `MoveTo` tasks in.
    #[must_use]
    pub fn schedule(&self) -> InternedScheduleLabel {
        self.schedule
    }
}

impl Default for NavPlugin {
    /// Keeps `MoveTo` tasks in `FixedUpdate`, as trees tick.
    fn default() -> Self {
        Self::new(FixedUpdate)
    }
}

impl Plugin for NavPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Destination>()
            .register_type::<Travel>()
            .register_type::<Route>()
            .configure_sets(self.schedule, NavSystems.after(TreeSystems))
            .add_systems(self.schedule, move_to::follow.in_set(NavSystems))
            .add_observer(move_to::start)
            .add_observer(move_to::stop);
        #[cfg(feature = "eqs")]
        app.configure_sets(self.schedule, NavSystems.after(bevy_arbor_eqs::EqsSystems));
    }

    fn finish(&self, app: &mut App) {
        // Ordered after trees and queries only where they share a schedule.
        let trees = app
            .get_added_plugins::<bevy_arbor_bt::TreePlugin>()
            .iter()
            .map(|plugin| plugin.schedule())
            .collect::<Vec<_>>();
        if !trees.is_empty() && !trees.contains(&self.schedule) {
            warn!(
                "NavPlugin runs in another schedule than TreePlugin: MoveTo tasks aren't ordered after trees"
            );
        }
        #[cfg(feature = "eqs")]
        {
            let queries = app
                .get_added_plugins::<bevy_arbor_eqs::EqsPlugin>()
                .iter()
                .map(|plugin| plugin.schedule())
                .collect::<Vec<_>>();
            if !queries.is_empty() && !queries.contains(&self.schedule) {
                warn!(
                    "NavPlugin runs in another schedule than EqsPlugin: MoveTo::found may read what a query found before it ran"
                );
            }
        }
    }
}
