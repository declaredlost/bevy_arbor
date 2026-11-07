use super::Behaviour;
use crate::behaviour::{Behave, Context, Guard, IntoGuard, Status};
use crate::tree::TreeOutline;

/// A branch of a `Select` or `Fallback`: a node, and the guard it runs under.
pub struct Branch {
    guard: Option<Guard>,
    pub(super) node: Behaviour,
    /// That it failed, and stays out of a `Select` until its guard stops
    /// passing or the `Select` restarts.
    failed: bool,
}

impl Branch {
    /// Runs `node` while `guard` passes.
    pub fn when<M>(guard: impl IntoGuard<M>, node: Behaviour) -> Self {
        Self {
            guard: Some(guard.into_guard()),
            node,
            failed: false,
        }
    }

    /// Runs `node` unless a branch before it can.
    #[must_use]
    pub fn always(node: Behaviour) -> Self {
        Self {
            guard: None,
            node,
            failed: false,
        }
    }

    /// Whether its guard passes; one without a guard always does.
    pub(super) fn passes(&mut self, cx: &mut Context) -> bool {
        match &mut self.guard {
            Some(guard) => guard.passes(cx),
            None => true,
        }
    }

    /// Writes it under its node: its guard, if it has one, then its node.
    pub(super) fn outline(&self, out: &mut TreeOutline) {
        if let Some(guard) = &self.guard {
            let failed = if self.failed { " (failed)" } else { "" };
            out.note(format!("when {}{failed}", guard.name()));
        }
        out.child(&self.node);
    }
}

/// The first branch whose guard passes, asking again each tick.
pub(super) struct Select {
    branches: Vec<Branch>,
    active: Option<usize>,
}

impl Select {
    pub(super) fn new(branches: Vec<Branch>) -> Self {
        Self {
            branches,
            active: None,
        }
    }

    /// Ready to start again.
    fn reset(&mut self) {
        self.active = None;
        for branch in &mut self.branches {
            branch.failed = false;
        }
    }

    /// Ticks branch `index`. `Some` if that settles this: anything but a
    /// failure, which falls through to what's next.
    fn run(&mut self, index: usize, cx: &mut Context) -> Option<Status> {
        self.active = Some(index);
        match self.branches[index].node.tick(cx) {
            Status::Running => {
                self.recheck_after(index, cx);
                Some(Status::Running)
            }
            Status::Success => {
                self.reset();
                Some(Status::Success)
            }
            Status::Failure => {
                self.branches[index].failed = true;
                self.active = None;
                None
            }
        }
    }

    /// Asks the guards of the failed branches after `index` running.
    ///
    /// They aren't asked otherwise while it runs: one whose guard has stopped
    /// is let back in, as one before it would be.
    fn recheck_after(&mut self, index: usize, cx: &mut Context) {
        for branch in &mut self.branches[index + 1..] {
            if branch.failed && !branch.passes(cx) {
                branch.failed = false;
            }
        }
    }

    /// Ticks the branch running, asked to stop: nothing else starts.
    fn wind_down(&mut self, cx: &mut Context) -> Status {
        let Some(active) = self.active else {
            self.reset();
            return Status::Failure;
        };
        let status = self.branches[active].node.tick(cx);
        if status != Status::Running {
            self.reset();
        }
        status
    }
}

impl Behave for Select {
    fn tick(&mut self, cx: &mut Context) -> Status {
        if cx.interrupted() {
            return self.wind_down(cx);
        }
        for index in 0..self.branches.len() {
            let branch = &mut self.branches[index];
            let passes = branch.passes(cx);
            if !passes {
                branch.failed = false;
            }
            let wanted = passes && !branch.failed;
            // Its guard stopped, or one above it passed.
            if let Some(active) = self.active
                && (active == index) != wanted
            {
                let node = &mut self.branches[active].node;
                // One holding goes on until it can stop: however it ends
                // then, it's what wanted it stopped that's next.
                if node.holds() {
                    let status = node.tick_interrupted(cx);
                    if status == Status::Running && node.holds() {
                        return Status::Running;
                    }
                }
                node.cancel(cx);
                self.active = None;
            }
            if wanted && let Some(status) = self.run(index, cx) {
                return status;
            }
        }
        self.reset();
        Status::Failure
    }

    fn cancel(&mut self, cx: &mut Context) {
        self.reset();
        for branch in &mut self.branches {
            branch.node.cancel(cx);
        }
    }

    fn outline(&self, out: &mut TreeOutline) {
        out.line("Select");
        for branch in &self.branches {
            branch.outline(out);
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use bevy_harness::TestApp;

    use crate::prelude::*;
    use crate::testing::*;

    fn work_or_other() -> Behaviour {
        behaviour_tree! {
            Select => {
                when(high) => task("Work", Work),
                task("Other", Other),
            }
        }
    }

    #[test]
    fn it_runs_the_first_whose_guard_passes() {
        let mut app = app();
        let npc = agent(&mut app, work_or_other());
        set(&mut app, npc, true, false);
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 1);
        assert_eq!(app.count::<With<Other>>(), 0);
    }

    #[test]
    fn a_guard_passing_cancels_what_runs_below_it() {
        let mut app = app();
        let npc = agent(&mut app, work_or_other());
        app.ticks(2);
        assert_eq!(app.count::<With<Other>>(), 1);
        set(&mut app, npc, true, false);
        app.tick();
        assert_eq!(app.count::<With<Other>>(), 0, "cancelled");
        assert_eq!(app.count::<With<Work>>(), 1, "started the same tick");
    }

    #[test]
    fn a_guard_stopping_cancels_its_branch() {
        let mut app = app();
        let npc = agent(&mut app, work_or_other());
        set(&mut app, npc, true, false);
        app.ticks(2);
        assert_eq!(app.count::<With<Work>>(), 1);
        set(&mut app, npc, false, false);
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 0);
        assert_eq!(app.count::<With<Other>>(), 1);
    }

    #[test]
    fn a_task_keeps_going_while_its_branch_stands() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behaviour_tree! {
                Select => {
                    when(high) => task("Work", Work),
                    when(low) => task("Other", Other),
                }
            },
        );
        set(&mut app, npc, false, true);
        app.tick();
        let first = app
            .world_mut()
            .query_filtered::<Entity, With<Other>>()
            .single(app.world())
            .unwrap();
        app.ticks(5);
        let last = app
            .world_mut()
            .query_filtered::<Entity, With<Other>>()
            .single(app.world())
            .unwrap();
        assert_eq!(first, last, "not restarted");
    }

    #[test]
    fn a_failed_branch_falls_through_and_stays_out_until_its_guard_stops() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behaviour_tree! {
                Forever => {
                    Select => {
                        when(high) => task("Work", (Work, Report::Fail)),
                        task("Other", Other),
                    }
                }
            },
        );
        set(&mut app, npc, true, false);
        app.ticks(2);
        assert_eq!(app.count::<With<Work>>(), 0, "failed");
        assert_eq!(app.count::<With<Other>>(), 1, "fell through");
        app.ticks(3);
        assert_eq!(app.count::<With<Work>>(), 0, "stays out");

        set(&mut app, npc, false, false);
        app.tick();
        set(&mut app, npc, true, false);
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 1, "back in");
        assert_eq!(app.count::<With<Other>>(), 0);
    }

    #[test]
    fn with_nothing_to_run_it_fails() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behaviour_tree! { Select => { when(high) => act(count) } },
        );
        app.tick();
        assert_eq!(outcome(&app, npc), Some(false));
        assert_eq!(acts(&app), 0);
    }

    #[test]
    fn a_branch_that_succeeds_ends_it() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behaviour_tree! {
                Select => {
                    act(count),
                    act(count),
                }
            },
        );
        app.tick();
        assert_eq!(outcome(&app, npc), Some(true));
        assert_eq!(acts(&app), 1);
    }

    #[test]
    fn it_takes_a_list_of_branches() {
        let mut app = app();
        let branches = vec![
            Branch::when(high, Behaviour::act(count)),
            Branch::always(Behaviour::act(refuse)),
        ];
        let npc = agent(&mut app, behaviour_tree! { Select => { @[ branches ] } });
        app.tick();
        assert_eq!(outcome(&app, npc), Some(false));
    }

    /// Fails the one `Work` task.
    fn fail_work(app: &mut TestApp) {
        let mut tasks = app.world_mut().query_filtered::<&mut Task, With<Work>>();
        tasks.single_mut(app.world_mut()).unwrap().fail();
    }

    #[test]
    fn a_failed_branch_after_the_running_one_is_let_back_in_once_its_guard_stops() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behaviour_tree! {
                Select => {
                    when(high) => task("Work", Work),
                    when(low) => task("Other", (Other, Report::Fail)),
                    Wait(100.0),
                }
            },
        );
        set(&mut app, npc, false, true);
        app.ticks(2);
        assert_eq!(app.count::<With<Other>>(), 0, "failed, so out");
        set(&mut app, npc, true, true);
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 1);
        set(&mut app, npc, true, false);
        app.tick();
        set(&mut app, npc, true, true);
        app.tick();
        fail_work(&mut app);
        app.tick();
        assert_eq!(app.count::<With<Other>>(), 1, "its guard stopped: back in");
    }
}
