use std::borrow::Cow;

use bevy::prelude::*;

use crate::behaviour::{Behave, Context, Outcome, Status};
use crate::task::{Graceful, Task, TaskEnded, TaskOf};
use crate::tree::TreeOutline;

type Insert = Box<dyn Fn(&mut EntityWorldMut) + Send + Sync>;

/// Spawns a task entity, and waits for it to report.
pub(super) struct Spawn {
    name: Name,
    insert: Insert,
    entity: Option<Entity>,
    /// Whether what's running has [`Graceful`] in its bundle.
    graceful: bool,
}

impl Spawn {
    pub(super) fn new<B: Bundle + Clone>(name: Cow<'static, str>, bundle: B) -> Self {
        Self {
            name: Name::new(name),
            insert: Box::new(move |entity| {
                entity.insert(bundle.clone());
            }),
            entity: None,
            graceful: false,
        }
    }

    /// The task, spawned if it hasn't been.
    fn started(&mut self, cx: &mut Context) -> Entity {
        if let Some(task) = self.entity {
            return task;
        }
        let (agent, now) = (cx.agent(), cx.now());
        let mut spawned =
            cx.world_mut()
                .spawn((Task::starting(now), TaskOf::new(agent), self.name.clone()));
        (self.insert)(&mut spawned);
        let task = spawned.id();
        self.graceful = spawned.contains::<Graceful>();
        self.entity = Some(task);
        task
    }
}

/// Tells `task` it's ended, if it's still there, and despawns it.
///
/// It's told `outcome`, or with none, what it reported, if it has.
fn end(cx: &mut Context, task: Entity, outcome: Option<Outcome>) {
    let agent = cx.agent();
    let world = cx.world_mut();
    let Ok(entity) = world.get_entity(task) else {
        return;
    };
    let outcome = outcome.or_else(|| entity.get::<Task>().and_then(Task::outcome));
    world.trigger(TaskEnded {
        entity: task,
        agent,
        outcome,
    });
    let _ = world.try_despawn(task);
}

impl Behave for Spawn {
    fn polls(&self) -> bool {
        false
    }

    fn tick(&mut self, cx: &mut Context) -> Status {
        let task = self.started(cx);
        let interrupted = self.graceful && cx.interrupted();
        let outcome = match cx.world_mut().get_mut::<Task>(task) {
            Some(mut task) => {
                // Asked again each tick: what wanted it stopped may have
                // stopped.
                if task.interrupted() != interrupted {
                    task.set_interrupted(interrupted);
                }
                task.outcome()
            }
            // One that's been despawned behind the tree's back failed.
            None => Some(Outcome::Failure),
        };
        let Some(outcome) = outcome else {
            if self.graceful {
                cx.hold();
            }
            // Its report, or its going, wakes the tree.
            cx.watch::<Task>(task);
            return Status::Running;
        };
        self.entity = None;
        end(cx, task, Some(outcome));
        outcome.into()
    }

    fn cancel(&mut self, cx: &mut Context) {
        if let Some(task) = self.entity.take() {
            end(cx, task, None);
        }
    }

    fn outline(&self, out: &mut TreeOutline) {
        out.line(self.name.as_str());
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use crate::prelude::*;
    use crate::testing::*;

    #[test]
    fn a_task_is_spawned_for_the_agent() {
        let mut app = app();
        let npc = agent(&mut app, behaviour_tree! { task("Work", Work) });
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 1);
        let of = app
            .world_mut()
            .query_filtered::<&TaskOf, With<Task>>()
            .single(app.world())
            .copied()
            .unwrap();
        assert_eq!(of.agent(), npc);
        let tasks: Vec<Entity> = app.world().get::<AgentTasks>(npc).unwrap().iter().collect();
        assert_eq!(tasks.len(), 1, "the agent knows its task");
        assert_eq!(outcome(&app, npc), None);
    }

    #[test]
    fn a_task_that_reports_ends_the_tree_and_goes() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behaviour_tree! { task("Work", (Work, Report::Succeed)) },
        );
        app.ticks(2);
        assert_eq!(outcome(&app, npc), Some(true));
        assert_eq!(app.count::<With<Work>>(), 0);
    }

    #[test]
    fn a_task_despawned_from_outside_counts_as_failed() {
        let mut app = app();
        let npc = agent(&mut app, behaviour_tree! { task("Work", Work) });
        app.tick();
        let task = app
            .world_mut()
            .query_filtered::<Entity, With<Work>>()
            .single(app.world())
            .unwrap();
        app.entity_mut(task).despawn();
        app.tick();
        assert_eq!(outcome(&app, npc), Some(false));
    }

    /// Winds down a tick after being told.
    fn wind_down(mut tasks: Query<&mut Task, With<Work>>, mut told: Local<bool>) {
        for mut task in &mut tasks {
            if *told {
                task.succeed();
            }
            *told = task.interrupted();
        }
    }

    #[test]
    fn a_graceful_task_is_told_and_waited_for() {
        let mut app = app();
        app.app_mut()
            .add_systems(FixedUpdate, wind_down.after(TreeSystems));
        let npc = agent(
            &mut app,
            behaviour_tree! {
                Select => {
                    when(high) => task("Other", Other),
                    task("Work", (Work, Graceful)),
                }
            },
        );
        app.tick();
        set(&mut app, npc, true, false);
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 1, "told, still going");
        assert_eq!(app.count::<With<Other>>(), 0, "waited for");
        app.ticks(2);
        assert_eq!(app.count::<With<Work>>(), 0, "wound down");
        assert_eq!(app.count::<With<Other>>(), 1, "and on to the next");
    }

    #[test]
    fn a_task_is_told_how_it_ended_as_it_goes() {
        #[derive(Resource, Default)]
        struct Ended(Vec<(&'static str, Option<Outcome>)>);
        let mut app = app();
        app.app_mut().init_resource::<Ended>();
        app.app_mut().add_observer(
            |ended: On<TaskEnded>, names: Query<&Name>, mut seen: ResMut<Ended>| {
                let name = if names
                    .get(ended.entity)
                    .is_ok_and(|name| name.as_str() == "Work")
                {
                    "Work"
                } else {
                    "Other"
                };
                seen.0.push((name, ended.outcome));
            },
        );
        let npc = agent(
            &mut app,
            behaviour_tree! {
                Sequence => {
                    task("Work", (Work, Report::Succeed)),
                    Select => {
                        when(high) => Wait(100.0),
                        task("Other", Other),
                    },
                }
            },
        );
        app.ticks(2);
        set(&mut app, npc, true, false);
        app.tick();
        assert_eq!(
            app.world().resource::<Ended>().0,
            [("Work", Some(Outcome::Success)), ("Other", None)],
            "reported, then cancelled"
        );
    }

    #[test]
    fn a_graceful_task_still_goes_with_its_tree() {
        let mut app = app();
        let npc = agent(&mut app, behaviour_tree! { task("Work", (Work, Graceful)) });
        app.tick();
        app.entity_mut(npc).remove::<BehaviourTree>();
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 0);
    }

    #[test]
    fn a_graceful_task_kept_after_all_is_told_so() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behaviour_tree! {
                Select => {
                    when(high) => task("Other", Other),
                    task("Work", (Work, Graceful)),
                }
            },
        );
        app.tick();
        set(&mut app, npc, true, false);
        app.tick();
        assert!(work_interrupted(&mut app), "told to stop");
        set(&mut app, npc, false, false);
        app.tick();
        assert!(!work_interrupted(&mut app), "told it's kept");
        assert_eq!(app.count::<With<Other>>(), 0);
    }

    #[test]
    fn a_task_cancelled_after_it_reported_ends_with_its_report() {
        #[derive(Resource, Default)]
        struct Ended(Vec<Option<Outcome>>);
        let mut app = app();
        app.app_mut().init_resource::<Ended>();
        app.app_mut()
            .add_observer(|ended: On<TaskEnded>, mut seen: ResMut<Ended>| {
                seen.0.push(ended.outcome);
            });
        let npc = agent(
            &mut app,
            behaviour_tree! {
                Select => {
                    when(high) => Wait(100.0),
                    task("Work", (Work, Report::Succeed)),
                }
            },
        );
        app.tick();
        set(&mut app, npc, true, false);
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 0, "cancelled");
        assert_eq!(app.world().resource::<Ended>().0, [Some(Outcome::Success)]);
    }
}
