use bevy::ecs::schedule::{InternedScheduleLabel, ScheduleLabel};
use bevy::prelude::*;

use crate::behavior::{Outcome, Status};
use crate::task::{Graceful, Task, TaskOf, Tasks};
use crate::tree::{self, BehaviorTreePaused, BehaviorTreeTrace};

/// The system set trees tick in: what decorators read goes before it, tasks
/// after.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BehaviorTreeSystems;

/// Ticks every [`BehaviorTree`](crate::tree::BehaviorTree), in `FixedUpdate`
/// unless told otherwise.
pub struct BehaviorTreePlugin {
    schedule: InternedScheduleLabel,
}

impl BehaviorTreePlugin {
    /// Ticks trees in `schedule`.
    pub fn new(schedule: impl ScheduleLabel) -> Self {
        Self {
            schedule: schedule.intern(),
        }
    }

    /// The schedule it ticks trees in.
    #[must_use]
    pub fn schedule(&self) -> InternedScheduleLabel {
        self.schedule
    }
}

impl Default for BehaviorTreePlugin {
    /// Ticks trees in `FixedUpdate`.
    fn default() -> Self {
        Self::new(FixedUpdate)
    }
}

impl Plugin for BehaviorTreePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<BehaviorTreeTrace>()
            .register_type::<BehaviorTreePaused>()
            .register_type::<Task>()
            .register_type::<TaskOf>()
            .register_type::<Tasks>()
            .register_type::<Graceful>()
            .register_type::<Outcome>()
            .register_type::<Status>()
            .init_resource::<tree::Wakes>()
            .add_systems(self.schedule, tree::tick.in_set(BehaviorTreeSystems))
            .add_observer(tree::check_ticks)
            .add_observer(tree::unpaused);
        tree::Wakes::index_tasks(app.world_mut());
    }

    fn finish(&self, app: &mut App) {
        if !app.is_plugin_added::<bevy::time::TimePlugin>() {
            warn!(
                "BehaviorTreePlugin without Bevy's TimePlugin: waits, timeouts and cooldowns never run out"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;

    use bevy::prelude::*;

    use crate::prelude::*;

    #[test]
    fn its_components_are_registered_for_inspectors() {
        let mut app = App::new();
        app.add_plugins(BehaviorTreePlugin::default());
        let registry = app.world().resource::<AppTypeRegistry>().read();
        for ty in [
            TypeId::of::<Task>(),
            TypeId::of::<TaskOf>(),
            TypeId::of::<Tasks>(),
            TypeId::of::<Graceful>(),
            TypeId::of::<BehaviorTreePaused>(),
            TypeId::of::<BehaviorTreeTrace>(),
        ] {
            assert!(
                registry
                    .get(ty)
                    .is_some_and(|registration| registration.data::<ReflectComponent>().is_some()),
                "{ty:?}"
            );
        }
    }

    #[derive(Component, Clone)]
    struct Work;

    #[test]
    fn without_time_trees_run_rather_than_panic() {
        let mut app = App::new();
        app.add_plugins(BehaviorTreePlugin::new(Update));
        app.world_mut().spawn(BehaviorTree::new(behavior_tree! {
            Sequence => { task(Work), Wait(1.0) }
        }));
        app.update();
        app.update();
        let mut works = app.world_mut().query_filtered::<(), With<Work>>();
        assert_eq!(works.iter(app.world()).count(), 1);
    }
}
