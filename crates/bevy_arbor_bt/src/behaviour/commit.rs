use super::Behaviour;
use crate::behaviour::{Behave, Context, Status};
use crate::tree::TreeOutline;

/// Its child, refusing to be interrupted once started.
///
/// Not even told, so a [`Graceful`](crate::task::Graceful) task in it doesn't
/// wind down.
pub(super) struct Commit {
    child: Behaviour,
}

impl Commit {
    pub(super) fn new(child: Behaviour) -> Self {
        Self { child }
    }
}

impl Behave for Commit {
    fn polls(&self) -> bool {
        false
    }

    fn tick(&mut self, cx: &mut Context) -> Status {
        let status = cx.sheltering(|cx| self.child.tick(cx));
        if status == Status::Running {
            cx.hold();
        }
        status
    }

    fn cancel(&mut self, cx: &mut Context) {
        self.child.cancel(cx);
    }

    fn outline(&self, out: &mut TreeOutline) {
        out.line("Commit");
        out.child(&self.child);
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use crate::prelude::*;
    use crate::testing::*;

    #[test]
    fn a_guard_waits_for_a_committed_part_only() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behaviour_tree! {
                Select => {
                    when(high) => task("Other", Other),
                    Sequence => {
                        Commit => { Wait(0.5) },
                        task("Work", Work),
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
            behaviour_tree! {
                Select => {
                    when(high) => task("Other", Other),
                    Sequence => {
                        Commit => { Wait(0.5) },
                        task("Work", Work),
                    },
                }
            },
        );
        app.tick();
        set(&mut app, npc, true, false);
        app.secs(0.6);
        assert_eq!(works_started(&app), 0, "never spawned, not just cancelled");
        assert_eq!(app.count::<With<Other>>(), 1);
    }

    #[test]
    fn a_graceful_task_in_it_is_not_told_to_stop() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behaviour_tree! {
                Select => {
                    when(high) => task("Other", Other),
                    Commit => { task("Work", (Work, Graceful)) },
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
            behaviour_tree! {
                Timeout(0.25) => { Commit => { task("Work", Work) } }
            },
        );
        app.secs(0.5);
        assert_eq!(outcome(&app, npc), Some(false));
        assert_eq!(app.count::<With<Work>>(), 0);
    }
}
