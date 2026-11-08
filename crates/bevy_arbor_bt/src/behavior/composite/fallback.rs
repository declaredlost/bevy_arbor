use crate::behavior::{Behave, Branch, Child, Context, Outcome, Status};

/// The first branch whose decorator passes, asking each decorator only as it
/// gets there: what's started isn't interrupted.
pub(in crate::behavior) struct Fallback {
    branches: Vec<Branch>,
    active: Option<usize>,
}

impl Fallback {
    pub(in crate::behavior) fn new(branches: Vec<Branch>) -> Self {
        Self {
            branches,
            active: None,
        }
    }
}

impl Behave for Fallback {
    fn start(&mut self, _cx: &mut Context) {
        self.active = None;
    }

    fn tick(&mut self, cx: &mut Context) -> Status {
        let mut next = 0;
        if let Some(active) = self.active {
            match self.branches[active].tick(cx, false) {
                Status::Failure => next = active + 1,
                status => return status,
            }
        }
        // Asked to stop, it doesn't start another.
        if cx.interrupted() {
            return Status::Failure;
        }
        for index in next..self.branches.len() {
            let branch = &mut self.branches[index];
            if !branch.passes(cx) {
                continue;
            }
            self.active = Some(index);
            match branch.tick(cx, false) {
                Status::Failure => {}
                status => return status,
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

    use crate::prelude::*;
    use crate::testing::*;

    #[test]
    fn a_fallback_is_not_interrupted() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behavior_tree! {
                Fallback => {
                    when(high) => task(Work),
                    task(Other),
                }
            },
        );
        app.ticks(2);
        assert_eq!(app.count::<With<Other>>(), 1);
        set(&mut app, npc, true, false);
        app.ticks(3);
        assert_eq!(app.count::<With<Other>>(), 1, "goes on");
        assert_eq!(app.count::<With<Work>>(), 0);
    }

    #[test]
    fn a_fallback_tries_the_next_after_a_failure() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behavior_tree! {
                Fallback => {
                    act(refuse),
                    when(high) => act(count),
                    act(count),
                }
            },
        );
        app.tick();
        assert_eq!(acts(&app), 2, "one refused, one decorated out, one ran");
        assert_eq!(outcome(&app, npc), Some(true));
    }
}
