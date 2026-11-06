use super::Branch;
use crate::behaviour::{Behave, Context, Status};
use crate::tree::TreeOutline;

/// The first branch whose guard passes, asking each guard only as it gets
/// there: what's started isn't interrupted.
pub(super) struct Fallback {
    branches: Vec<Branch>,
    active: Option<usize>,
}

impl Fallback {
    pub(super) fn new(branches: Vec<Branch>) -> Self {
        Self {
            branches,
            active: None,
        }
    }
}

impl Behave for Fallback {
    fn tick(&mut self, cx: &mut Context) -> Status {
        let mut next = 0;
        if let Some(active) = self.active {
            match self.branches[active].node.tick(cx) {
                Status::Running => return Status::Running,
                Status::Success => {
                    self.active = None;
                    return Status::Success;
                }
                Status::Failure => next = active + 1,
            }
        }
        for index in next..self.branches.len() {
            let branch = &mut self.branches[index];
            if !branch.passes(cx) {
                continue;
            }
            match branch.node.tick(cx) {
                Status::Running => {
                    self.active = Some(index);
                    return Status::Running;
                }
                Status::Success => {
                    self.active = None;
                    return Status::Success;
                }
                Status::Failure => {}
            }
        }
        self.active = None;
        Status::Failure
    }

    fn cancel(&mut self, cx: &mut Context) {
        self.active = None;
        for branch in &mut self.branches {
            branch.node.cancel(cx);
        }
    }

    fn outline(&self, out: &mut TreeOutline) {
        out.line("Fallback");
        for branch in &self.branches {
            branch.outline(out);
        }
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
            behaviour_tree! {
                Fallback => {
                    when(high) => task("Work", Work),
                    task("Other", Other),
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
            behaviour_tree! {
                Fallback => {
                    act(refuse),
                    when(high) => act(count),
                    act(count),
                }
            },
        );
        app.tick();
        assert_eq!(acts(&app), 2, "one refused, one guarded out, one ran");
        assert_eq!(outcome(&app, npc), Some(true));
    }
}
