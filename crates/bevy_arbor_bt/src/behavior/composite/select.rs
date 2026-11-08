use crate::behavior::{Behave, Branch, Child, Context, Outcome, Status};

/// The first branch whose decorator passes, asking again each tick.
pub(in crate::behavior) struct Select {
    branches: Vec<Branch>,
    active: Option<usize>,
}

impl Select {
    pub(in crate::behavior) fn new(branches: Vec<Branch>) -> Self {
        Self {
            branches,
            active: None,
        }
    }

    /// Ticks branch `index`. `Some` if that settles this: anything but a
    /// failure, which falls through to what's next.
    fn run(&mut self, index: usize, cx: &mut Context) -> Option<Status> {
        self.active = Some(index);
        match self.branches[index].tick(cx, false) {
            Status::Running => {
                self.recheck_after(index, cx);
                Some(Status::Running)
            }
            Status::Success => Some(Status::Success),
            Status::Failure => {
                self.branches[index].failed = true;
                self.active = None;
                None
            }
        }
    }

    /// Asks the decorators of the failed branches after `index` running.
    ///
    /// They aren't asked otherwise while it runs: one whose decorator has
    /// stopped is let back in, as one before it would be.
    fn recheck_after(&mut self, index: usize, cx: &mut Context) {
        for branch in &mut self.branches[index + 1..] {
            if branch.failed && !branch.passes(cx) {
                branch.failed = false;
            }
        }
    }

    /// Ticks the branch running, asked to stop: nothing else starts.
    fn wind_down(&mut self, cx: &mut Context) -> Status {
        match self.active {
            Some(active) => self.branches[active].tick(cx, false),
            None => Status::Failure,
        }
    }
}

impl Behave for Select {
    fn start(&mut self, _cx: &mut Context) {
        self.active = None;
        for branch in &mut self.branches {
            branch.failed = false;
        }
    }

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
            // Its decorator stopped, or one above it passed.
            if let Some(active) = self.active
                && (active == index) != wanted
            {
                let running = &mut self.branches[active];
                // One holding goes on until it can stop: however it ends
                // then, it's what wanted it stopped that's next.
                if running.holds() {
                    let status = running.tick(cx, true);
                    if status == Status::Running && running.holds() {
                        return Status::Running;
                    }
                }
                running.cancel(cx);
                self.active = None;
            }
            if wanted && let Some(status) = self.run(index, cx) {
                return status;
            }
        }
        Status::Failure
    }

    fn end(&mut self, cx: &mut Context, _outcome: Option<Outcome>) {
        for branch in &mut self.branches {
            branch.cancel(cx);
        }
    }

    fn children(&self) -> Vec<Child<'_>> {
        self.branches.iter().map(Branch::child).collect()
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use bevy_harness::TestApp;

    use crate::prelude::*;
    use crate::testing::*;

    fn work_or_other() -> Behavior {
        behavior_tree! {
            Select => {
                when(high) => task(Work),
                task(Other),
            }
        }
    }

    #[test]
    fn it_runs_the_first_whose_decorator_passes() {
        let mut app = app();
        let npc = agent(&mut app, work_or_other());
        set(&mut app, npc, true, false);
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 1);
        assert_eq!(app.count::<With<Other>>(), 0);
    }

    #[test]
    fn a_decorator_passing_cancels_what_runs_below_it() {
        let mut app = app();
        let npc = agent(&mut app, work_or_other());
        app.ticks(2);
        assert_eq!(app.count::<With<Other>>(), 1);
        set(&mut app, npc, true, false);
        app.tick();
        assert_eq!(app.count::<With<Other>>(), 0, "canceled");
        assert_eq!(app.count::<With<Work>>(), 1, "started the same tick");
    }

    #[test]
    fn a_decorator_stopping_cancels_its_branch() {
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
            behavior_tree! {
                Select => {
                    when(high) => task(Work),
                    when(low) => task(Other),
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
    fn a_failed_branch_falls_through_and_stays_out_until_its_decorator_stops() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behavior_tree! {
                Forever => {
                    Select => {
                        when(high) => task((Work, Report::Fail)),
                        task(Other),
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
            behavior_tree! { Select => { when(high) => act(count) } },
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
            behavior_tree! {
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
            Branch::when(high, Behavior::act(count)),
            Branch::always(Behavior::act(refuse)),
        ];
        let npc = agent(&mut app, behavior_tree! { Select => { @[ branches ] } });
        app.tick();
        assert_eq!(outcome(&app, npc), Some(false));
    }

    /// Fails the one `Work` task.
    fn fail_work(app: &mut TestApp) {
        let mut tasks = app.world_mut().query_filtered::<&mut Task, With<Work>>();
        tasks.single_mut(app.world_mut()).unwrap().fail();
    }

    #[test]
    fn a_failed_branch_after_the_running_one_is_let_back_in_once_its_decorator_stops() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behavior_tree! {
                Select => {
                    when(high) => task(Work),
                    when(low) => task((Other, Report::Fail)),
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
        assert_eq!(
            app.count::<With<Other>>(),
            1,
            "its decorator stopped: back in"
        );
    }
}
