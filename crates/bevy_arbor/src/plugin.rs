use bevy::ecs::intern::Interned;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;

use crate::behaviour::{Outcome, Status};
use crate::task::{AgentTasks, Graceful, Task, TaskOf};
use crate::tree::{self, TreePaused, TreeTrace};

/// The system set trees tick in: what guards read goes before it, tasks after.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArborSystems;

/// Ticks every [`BehaviourTree`](crate::tree::BehaviourTree), in `FixedUpdate`
/// unless told otherwise.
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
            .register_type::<TreePaused>()
            .register_type::<Task>()
            .register_type::<TaskOf>()
            .register_type::<AgentTasks>()
            .register_type::<Graceful>()
            .register_type::<Outcome>()
            .register_type::<Status>()
            .add_systems(self.schedule, tree::tick.in_set(ArborSystems));
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
        app.add_plugins(ArborPlugin::default());
        let registry = app.world().resource::<AppTypeRegistry>().read();
        for ty in [
            TypeId::of::<Task>(),
            TypeId::of::<TaskOf>(),
            TypeId::of::<AgentTasks>(),
            TypeId::of::<Graceful>(),
            TypeId::of::<TreePaused>(),
            TypeId::of::<TreeTrace>(),
        ] {
            assert!(
                registry
                    .get(ty)
                    .is_some_and(|registration| registration.data::<ReflectComponent>().is_some()),
                "{ty:?}"
            );
        }
    }
}
