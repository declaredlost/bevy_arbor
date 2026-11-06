use std::borrow::Cow;
use std::time::Duration;

use bevy::prelude::*;

use crate::behave::{Behave, Context, Outcome, Status};
use crate::task::{Graceful, Task};
use crate::tree::TreeOutline;

type Insert = Box<dyn Fn(&mut EntityWorldMut) + Send + Sync>;

/// Spawns a task entity, and waits for it to report.
pub(super) struct Spawn {
    name: Name,
    insert: Insert,
    entity: Option<Entity>,
    /// When what's running started, by the tree's clock.
    started: Duration,
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
            started: Duration::ZERO,
            graceful: false,
        }
    }

    /// The task, spawned if it hasn't been.
    fn started(&mut self, cx: &mut Context) -> Entity {
        if let Some(task) = self.entity {
            return task;
        }
        let agent = cx.agent();
        self.started = cx.now();
        let mut spawned = cx.world_mut().spawn((Task::new(agent), self.name.clone()));
        (self.insert)(&mut spawned);
        let task = spawned.id();
        self.graceful = spawned.contains::<Graceful>();
        self.entity = Some(task);
        task
    }
}

impl Behave for Spawn {
    fn tick(&mut self, cx: &mut Context) -> Status {
        let task = self.started(cx);
        let elapsed = cx.now().saturating_sub(self.started);
        let interrupt = self.graceful && cx.interrupted();
        if let Some(mut task) = cx.world_mut().get_mut::<Task>(task) {
            // Time passing isn't a change a task system should react to.
            task.bypass_change_detection().set_elapsed(elapsed);
            if interrupt {
                task.interrupt();
            }
        }
        // One that's been despawned behind the tree's back failed.
        let outcome = match cx.world().get::<Task>(task) {
            Some(task) => task.outcome(),
            None => Some(Outcome::Failure),
        };
        let Some(outcome) = outcome else {
            if self.graceful {
                cx.hold();
            }
            return Status::Running;
        };
        self.entity = None;
        let _ = cx.world_mut().try_despawn(task);
        outcome.into()
    }

    fn cancel(&mut self, cx: &mut Context) {
        if let Some(task) = self.entity.take() {
            let _ = cx.world_mut().try_despawn(task);
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
        let mut tasks = app.world_mut().query::<&Task>();
        let task = tasks.single(app.world()).unwrap();
        assert_eq!(task.agent(), npc);
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
            .add_systems(FixedUpdate, wind_down.after(ArborSystems));
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
    fn a_graceful_task_still_goes_with_its_tree() {
        let mut app = app();
        let npc = agent(&mut app, behaviour_tree! { task("Work", (Work, Graceful)) });
        app.tick();
        app.entity_mut(npc).remove::<BehaviourTree>();
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 0);
    }
}
