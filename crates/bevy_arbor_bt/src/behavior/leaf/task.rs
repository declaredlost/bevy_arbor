use std::any::type_name;
use std::borrow::Cow;

use bevy::prelude::*;
use bevy::utils::prelude::ShortName;

use crate::behavior::{Behave, Context, Outcome, Status};
use crate::task::{Graceful, Task, TaskOf};

type Insert = Box<dyn Fn(&mut EntityWorldMut) + Send + Sync>;

/// Spawns a task entity, and waits for it to report.
pub(in crate::behavior) struct Spawn {
    name: Name,
    insert: Insert,
    /// The task running.
    entity: Option<Entity>,
    /// Whether what's running has [`Graceful`] in its bundle.
    graceful: bool,
}

impl Spawn {
    pub(in crate::behavior) fn new<B: Bundle + Clone>(name: Cow<'static, str>, bundle: B) -> Self {
        Self {
            name: Name::new(name),
            insert: Box::new(move |entity| {
                entity.insert(bundle.clone());
            }),
            entity: None,
            graceful: false,
        }
    }
}

impl Behave for Spawn {
    fn start(&mut self, cx: &mut Context) {
        let (agent, now) = (cx.agent(), cx.now());
        let mut spawned =
            cx.world_mut()
                .spawn((Task::starting(now), TaskOf::new(agent), self.name.clone()));
        (self.insert)(&mut spawned);
        self.graceful = spawned.contains::<Graceful>();
        self.entity = Some(spawned.id());
    }

    fn tick(&mut self, cx: &mut Context) -> Status {
        let Some(task) = self.entity else {
            return Status::Failure;
        };
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
        outcome.into()
    }

    /// Takes the task away, if it's still there: its `Task` is still on it
    /// for `On<Remove<..>>` observers, with what it reported, if it has.
    fn end(&mut self, cx: &mut Context, _outcome: Option<Outcome>) {
        if let Some(task) = self.entity.take() {
            let _ = cx.world_mut().try_despawn(task);
        }
    }

    fn label(&self) -> String {
        self.name.to_string()
    }
}

/// What a bundle `B` is called: its type's short name, or for a tuple, its
/// first element's.
pub(in crate::behavior) fn bundle_name<B: ?Sized>() -> String {
    ShortName(first_element(type_name::<B>())).to_string()
}

/// The first element of a tuple type's `name`, nested ones too; anything else
/// as it is.
fn first_element(mut name: &str) -> &str {
    while let Some(inner) = name.strip_prefix('(') {
        let mut depth = 0usize;
        let mut previous = ' ';
        let end = inner
            .char_indices()
            .find(|&(_, c)| {
                let arrow = previous == '-';
                previous = c;
                match c {
                    // The `>` of a `->` closes nothing.
                    '>' if arrow => {}
                    '<' | '(' | '[' => depth += 1,
                    '>' | ')' | ']' if depth > 0 => depth -= 1,
                    ',' | ')' if depth == 0 => return true,
                    _ => {}
                }
                false
            })
            .map_or(inner.len(), |(at, _)| at);
        name = inner[..end].trim();
    }
    name
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use bevy_harness::TestApp;

    use crate::prelude::*;
    use crate::testing::*;

    #[test]
    fn a_task_is_spawned_for_the_agent() {
        let mut app = app();
        let npc = agent(&mut app, behavior_tree! { task(Work) });
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
        let npc = agent(&mut app, behavior_tree! { task((Work, Report::Succeed)) });
        app.ticks(2);
        assert_eq!(outcome(&app, npc), Some(true));
        assert_eq!(app.count::<With<Work>>(), 0);
    }

    #[test]
    fn a_task_despawned_from_outside_counts_as_failed() {
        let mut app = app();
        let npc = agent(&mut app, behavior_tree! { task(Work) });
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
            behavior_tree! {
                Select => {
                    when(high) => task(Other),
                    task((Work, Graceful)),
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

    /// How each task went, as it was taken away, by name.
    #[derive(Resource, Default)]
    struct Ended(Vec<(String, Option<Outcome>)>);

    fn ended(app: &mut TestApp) {
        app.app_mut().init_resource::<Ended>();
        app.app_mut().add_observer(
            |ended: On<Remove<Task>>, tasks: Query<(&Task, &Name)>, mut seen: ResMut<Ended>| {
                let (task, name) = tasks.get(ended.entity).unwrap();
                seen.0.push((name.to_string(), task.outcome()));
            },
        );
    }

    fn seen(app: &TestApp) -> Vec<(&str, Option<Outcome>)> {
        let ended = &app.world().resource::<Ended>().0;
        ended
            .iter()
            .map(|(name, outcome)| (name.as_str(), *outcome))
            .collect()
    }

    #[test]
    fn a_task_going_still_has_how_it_went() {
        let mut app = app();
        ended(&mut app);
        let npc = agent(
            &mut app,
            behavior_tree! {
                Sequence => {
                    task((Work, Report::Succeed)),
                    Select => {
                        when(high) => Wait(100.0),
                        task(Other),
                    },
                }
            },
        );
        app.ticks(2);
        set(&mut app, npc, true, false);
        app.tick();
        assert_eq!(
            seen(&app),
            [("Work", Some(Outcome::Success)), ("Other", None)],
            "reported, then canceled"
        );
    }

    #[test]
    fn a_task_going_with_its_agent_is_seen_going_too() {
        let mut app = app();
        ended(&mut app);
        let npc = agent(&mut app, behavior_tree! { task(Work) });
        app.tick();
        app.entity_mut(npc).despawn();
        assert_eq!(seen(&app), [("Work", None)]);
    }

    #[test]
    fn a_graceful_task_still_goes_with_its_tree() {
        let mut app = app();
        let npc = agent(&mut app, behavior_tree! { task((Work, Graceful)) });
        app.tick();
        app.entity_mut(npc).remove::<BehaviorTree>();
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 0);
    }

    #[test]
    fn a_graceful_task_kept_after_all_is_told_so() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behavior_tree! {
                Select => {
                    when(high) => task(Other),
                    task((Work, Graceful)),
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
    fn a_task_canceled_after_it_reported_goes_with_its_report() {
        let mut app = app();
        ended(&mut app);
        let npc = agent(
            &mut app,
            behavior_tree! {
                Select => {
                    when(high) => Wait(100.0),
                    task((Work, Report::Succeed)),
                }
            },
        );
        app.tick();
        set(&mut app, npc, true, false);
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 0, "canceled");
        assert_eq!(seen(&app), [("Work", Some(Outcome::Success))]);
    }

    #[derive(Component)]
    struct Chased;

    #[derive(Component)]
    struct Speed<T>(T);

    #[test]
    fn a_bundle_is_called_by_its_first_component() {
        assert_eq!(super::bundle_name::<Chased>(), "Chased");
        assert_eq!(super::bundle_name::<(Chased, Name)>(), "Chased");
        assert_eq!(super::bundle_name::<((Chased, Name), Name)>(), "Chased");
        assert_eq!(
            super::bundle_name::<(Speed<(u8, f32)>, Chased)>(),
            "Speed<(u8, f32)>"
        );
        assert_eq!(
            super::bundle_name::<(Result<fn() -> u32, u8>, Chased)>(),
            "Result<fn() -> u32, u8>",
            "the `>` of `->` closes nothing"
        );
        assert_eq!(super::bundle_name::<()>(), "");
    }
}
