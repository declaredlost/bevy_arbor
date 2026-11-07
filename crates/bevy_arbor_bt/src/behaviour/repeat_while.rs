use super::Behaviour;
use crate::behaviour::{Behave, Context, Guard, Status};
use crate::tree::TreeOutline;

/// Its child over and over, while its guard passes.
pub(super) struct RepeatWhile {
    guard: Guard,
    child: Behaviour,
}

impl RepeatWhile {
    pub(super) fn new(guard: Guard, child: Behaviour) -> Self {
        Self { guard, child }
    }
}

impl Behave for RepeatWhile {
    fn polls(&self) -> bool {
        false
    }

    fn tick(&mut self, cx: &mut Context) -> Status {
        if !self.guard.passes(cx) {
            if self.child.holds() {
                let status = self.child.tick_interrupted(cx);
                if status == Status::Running && self.child.holds() {
                    return Status::Running;
                }
            }
            self.child.cancel(cx);
            return Status::Success;
        }
        // Asked to stop, it doesn't start its child again.
        if cx.interrupted() && !self.child.running {
            return Status::Success;
        }
        match self.child.tick(cx) {
            Status::Failure => Status::Failure,
            Status::Success if cx.interrupted() => Status::Success,
            Status::Success => {
                // It starts its child again next tick.
                cx.poll();
                Status::Running
            }
            Status::Running => Status::Running,
        }
    }

    fn cancel(&mut self, cx: &mut Context) {
        self.child.cancel(cx);
    }

    fn outline(&self, out: &mut TreeOutline) {
        out.line(format!("While {}", self.guard.name()));
        out.child(&self.child);
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use crate::prelude::*;
    use crate::testing::*;

    #[test]
    fn it_repeats_while_the_guard_passes_then_cancels_and_succeeds() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behaviour_tree! {
                While(high) => { task("Work", (Work, Report::Succeed)) }
            },
        );
        set(&mut app, npc, true, false);
        app.ticks(5);
        assert_eq!(outcome(&app, npc), None);
        assert_eq!(app.count::<With<Work>>(), 1);

        set(&mut app, npc, false, false);
        app.tick();
        assert_eq!(outcome(&app, npc), Some(true));
        assert_eq!(app.count::<With<Work>>(), 0);
    }

    #[test]
    fn it_fails_if_its_child_does() {
        let mut app = app();
        let npc = agent(&mut app, behaviour_tree! { While(high) => { act(refuse) } });
        set(&mut app, npc, true, false);
        app.tick();
        assert_eq!(outcome(&app, npc), Some(false));
    }
}
