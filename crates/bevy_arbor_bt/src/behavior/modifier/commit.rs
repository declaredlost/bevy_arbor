use crate::behavior::{Behave, Behavior, Child, Context, Outcome, Status};

/// Its child, refusing to be interrupted once started.
///
/// Not even told, so a [`Graceful`](crate::task::Graceful) task in it doesn't
/// wind down.
pub(in crate::behavior) struct Commit {
    child: Behavior,
}

impl Commit {
    pub(in crate::behavior) fn new(child: Behavior) -> Self {
        Self { child }
    }
}

impl Behave for Commit {
    fn tick(&mut self, cx: &mut Context) -> Status {
        let status = cx.sheltering(|cx| self.child.tick(cx));
        if status == Status::Running {
            cx.hold();
        }
        status
    }

    fn end(&mut self, cx: &mut Context, _outcome: Option<Outcome>) {
        self.child.cancel(cx);
    }

    fn children(&self) -> Vec<Child<'_>> {
        vec![Child::new(&self.child)]
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use crate::prelude::*;
    use crate::testing::*;

    #[test]
    fn a_decorator_waits_for_a_committed_part_only() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behavior_tree! {
                Select => {
                    when(high) => task(Other),
                    Sequence => {
                        Commit => { Wait(0.5) },
                        task(Work),
                    },
                }
            },
        );
        app.tick();
        set(&mut app, npc, true, false);
        app.tick();
        assert_eq!(app.count::<With<Other>>(), 0, "committed");
        app.secs(0.6);
        assert_eq!(app.count::<With<Other>>(), 1, "over, so interrupted");
        assert_eq!(app.count::<With<Work>>(), 0);
    }

    #[test]
    fn what_comes_after_it_is_not_started_once_asked_to_stop() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behavior_tree! {
                Select => {
                    when(high) => task(Other),
                    Sequence => {
                        Commit => { Wait(0.5) },
                        task(Work),
                    },
                }
            },
        );
        app.tick();
        set(&mut app, npc, true, false);
        app.secs(0.6);
        assert_eq!(works_started(&app), 0, "never spawned, not just canceled");
        assert_eq!(app.count::<With<Other>>(), 1);
    }

    #[test]
    fn a_graceful_task_in_it_is_not_told_to_stop() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behavior_tree! {
                Select => {
                    when(high) => task(Other),
                    Commit => { task((Work, Graceful)) },
                }
            },
        );
        app.tick();
        set(&mut app, npc, true, false);
        app.ticks(3);
        assert!(!work_interrupted(&mut app), "not told");
        assert_eq!(app.count::<With<Other>>(), 0);
    }

    #[test]
    fn a_timeout_still_stops_it() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behavior_tree! {
                Timeout(0.25) => { Commit => { task(Work) } }
            },
        );
        app.secs(0.5);
        assert_eq!(outcome(&app, npc), Some(false));
        assert_eq!(app.count::<With<Work>>(), 0);
    }
}
