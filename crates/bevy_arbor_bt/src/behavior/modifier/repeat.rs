use crate::behavior::{Behave, Behavior, Child, Context, Outcome, Status};

/// Its child a number of times, until it fails.
pub(in crate::behavior) struct Repeat {
    times: u32,
    /// How many times its child has succeeded this run.
    done: u32,
    child: Behavior,
}

impl Repeat {
    pub(in crate::behavior) fn new(times: u32, child: Behavior) -> Self {
        Self {
            times,
            done: 0,
            child,
        }
    }
}

impl Behave for Repeat {
    fn start(&mut self, _cx: &mut Context) {
        self.done = 0;
    }

    fn tick(&mut self, cx: &mut Context) -> Status {
        // What ends is followed at once, as in a `Sequence` of copies: it's
        // bounded by `times`, so it can't hang.
        while self.done < self.times {
            match self.child.tick(cx) {
                Status::Running => return Status::Running,
                Status::Failure => return Status::Failure,
                Status::Success => self.done += 1,
            }
            // Asked to stop, it doesn't start its child again.
            if cx.interrupted() {
                break;
            }
        }
        Status::Success
    }

    fn end(&mut self, cx: &mut Context, _outcome: Option<Outcome>) {
        self.child.cancel(cx);
    }

    fn label(&self) -> String {
        format!("Repeat({}, {} done)", self.times, self.done)
    }

    fn children(&self) -> Vec<Child<'_>> {
        vec![Child::new(&self.child)]
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use crate::testing::*;

    #[test]
    fn it_runs_its_child_that_many_times_then_succeeds() {
        let mut app = app();
        let npc = agent(&mut app, behavior_tree! { Repeat(3) => { act(count) } });
        app.tick();
        assert_eq!(acts(&app), 3, "one tick, as a sequence of three would");
        assert_eq!(outcome(&app, npc), Some(true));
    }

    #[test]
    fn it_waits_for_a_running_child_between_times() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behavior_tree! { Repeat(2) => { task((Work, Report::Succeed)) } },
        );
        app.tick();
        assert_eq!(works_started(&app), 1);
        app.tick();
        assert_eq!(works_started(&app), 2, "the second once the first reported");
        assert_eq!(outcome(&app, npc), None);
        app.tick();
        assert_eq!(outcome(&app, npc), Some(true));
    }

    #[test]
    fn it_fails_at_the_first_failure() {
        let mut app = app();
        let npc = agent(&mut app, behavior_tree! { Repeat(3) => { act(refuse) } });
        app.tick();
        assert_eq!(acts(&app), 1);
        assert_eq!(outcome(&app, npc), Some(false));
    }

    #[test]
    fn it_counts_again_each_run() {
        let mut app = app();
        agent(
            &mut app,
            behavior_tree! { Forever => { Repeat(2) => { act(count) } } },
        );
        app.tick();
        assert_eq!(acts(&app), 4, "two runs of two, as Forever follows once");
    }

    #[test]
    fn none_times_succeeds_at_once() {
        let mut app = app();
        let npc = agent(&mut app, behavior_tree! { Repeat(0) => { act(count) } });
        app.tick();
        assert_eq!(acts(&app), 0);
        assert_eq!(outcome(&app, npc), Some(true));
    }
}
