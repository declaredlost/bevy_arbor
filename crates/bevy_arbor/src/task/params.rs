use bevy::ecs::query::{IterQueryData, QueryData, QueryItem};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::components::Task;
use crate::tree::TreePaused;

/// Tasks with `T`, each with `A` of its agent. A paused tree's tasks are
/// skipped.
#[derive(SystemParam)]
pub struct Tasks<'w, 's, T: QueryData + 'static, A: QueryData + 'static> {
    tasks: Query<'w, 's, (T, &'static mut Task)>,
    agents: Query<'w, 's, A, (Without<Task>, Without<TreePaused>)>,
}

impl<'s, T: IterQueryData + 'static, A: QueryData + 'static> Tasks<'_, 's, T, A> {
    /// Runs `tick` for each task that hasn't reported and whose agent has
    /// `A`.
    pub fn each(
        &mut self,
        mut tick: impl for<'a, 'b> FnMut(QueryItem<'a, 's, T>, Mut<'a, Task>, QueryItem<'b, 's, A>),
    ) {
        for (data, task) in self.tasks.iter_mut() {
            if task.outcome().is_some() {
                continue;
            }
            if let Ok(agent) = self.agents.get_mut(task.agent()) {
                tick(data, task, agent);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use bevy_harness::TestApp;

    use crate::prelude::*;
    use crate::testing::*;

    #[test]
    fn tasks_pairs_each_task_with_its_agent_and_skips_reported_ones() {
        fn tick(mut tasks: Tasks<&Work, &mut Flags>) {
            tasks.each(|_, mut task, mut flags| {
                flags.low = true;
                task.succeed();
            });
        }
        let mut app = TestApp::builder()
            .plugins(ArborPlugin::default())
            .systems(FixedUpdate, tick.after(ArborSystems))
            .build();
        let npc = agent(&mut app, behaviour_tree! { task("Work", Work) });
        app.tick();
        assert!(app.world().get::<Flags>(npc).unwrap().low);
        app.entity_mut(npc).insert(Flags::default());
        app.tick();
        assert_eq!(outcome(&app, npc), Some(true));
        assert!(!app.world().get::<Flags>(npc).unwrap().low);
    }
}
