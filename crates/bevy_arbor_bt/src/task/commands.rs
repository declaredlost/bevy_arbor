use bevy::prelude::*;

use super::components::Task;
use crate::behavior::Outcome;

/// Reporting a task through [`Commands`]: for an observer or a system that
/// doesn't query [`Task`].
///
/// As [`Task::report`] does, once the commands are applied: the first report
/// counts, and an entity that isn't a task, or has gone, is left as it is.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_arbor_bt::prelude::*;
/// #[derive(Component)]
/// struct Dig;
///
/// #[derive(Message)]
/// struct Struck(Entity);
///
/// fn struck(
///     mut struck: MessageReader<Struck>,
///     digs: Query<(), With<Dig>>,
///     mut commands: Commands,
/// ) {
///     for Struck(task) in struck.read() {
///         if digs.contains(*task) {
///             commands.entity(*task).fail_task();
///         }
///     }
/// }
/// ```
pub trait TaskCommands {
    /// Reports `outcome`, unless it's already reported.
    fn report_task(&mut self, outcome: Outcome) -> &mut Self;

    /// Reports success, unless it's already reported.
    fn succeed_task(&mut self) -> &mut Self {
        self.report_task(Outcome::Success)
    }

    /// Reports failure, unless it's already reported.
    fn fail_task(&mut self) -> &mut Self {
        self.report_task(Outcome::Failure)
    }
}

impl TaskCommands for EntityCommands<'_> {
    fn report_task(&mut self, outcome: Outcome) -> &mut Self {
        self.queue_silenced(move |mut entity: EntityWorldMut| {
            if let Some(mut task) = entity.get_mut::<Task>() {
                task.report(outcome);
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use bevy_harness::TestApp;

    use crate::prelude::*;
    use crate::testing::*;

    #[test]
    fn a_task_reported_through_commands_ends_its_node() {
        fn report(works: Query<Entity, With<Work>>, mut commands: Commands) {
            for work in &works {
                commands.entity(work).succeed_task();
            }
        }
        let mut app = TestApp::builder()
            .plugins(TreePlugin::default())
            .systems(FixedUpdate, report.after(TreeSystems))
            .build();
        let npc = agent(&mut app, behavior_tree! { task(Work) });
        app.tick();
        app.tick();
        assert_eq!(outcome(&app, npc), Some(true));
    }

    #[test]
    fn the_first_report_counts_and_a_gone_task_is_no_error() {
        let mut app = app();
        let npc = agent(&mut app, behavior_tree! { task(Work) });
        app.tick();
        let work = app
            .world_mut()
            .query_filtered::<Entity, With<Work>>()
            .single(app.world())
            .unwrap();
        let mut commands = app.world_mut().commands();
        commands.entity(work).fail_task().succeed_task();
        app.world_mut().flush();
        assert_eq!(
            app.world().get::<Task>(work).unwrap().outcome(),
            Some(Outcome::Failure)
        );
        app.tick();
        assert_eq!(outcome(&app, npc), Some(false));
        app.world_mut().commands().entity(work).succeed_task();
        app.world_mut().flush();
    }
}
