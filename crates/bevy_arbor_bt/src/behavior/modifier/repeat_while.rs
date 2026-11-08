use crate::behavior::{Behave, Behavior, Child, Context, Outcome, Status};
use crate::decorator::Decorator;

/// Its child over and over, while its decorator passes.
pub(in crate::behavior) struct RepeatWhile {
    decorator: Decorator,
    child: Behavior,
}

impl RepeatWhile {
    pub(in crate::behavior) fn new(decorator: Decorator, child: Behavior) -> Self {
        Self { decorator, child }
    }
}

impl Behave for RepeatWhile {
    fn start(&mut self, cx: &mut Context) {
        self.decorator.start(cx);
    }

    fn tick(&mut self, cx: &mut Context) -> Status {
        if !self.decorator.passes(cx) {
            if self.child.holds() {
                let status = self.child.tick_interrupted(cx);
                if status == Status::Running && self.child.holds() {
                    return Status::Running;
                }
                // Ended winding down: as it ended.
                if status != Status::Running {
                    return status;
                }
            }
            return Status::Success;
        }
        // Asked to stop, it doesn't start its child again.
        if cx.interrupted() && !self.child.is_running() {
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

    fn end(&mut self, cx: &mut Context, outcome: Option<Outcome>) {
        self.child.cancel(cx);
        self.decorator.end(cx, outcome);
    }

    fn label(&self) -> String {
        format!("RepeatWhile {}", self.decorator.label())
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
    fn it_repeats_while_the_decorator_passes_then_cancels_and_succeeds() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behavior_tree! {
                RepeatWhile(high) => { task((Work, Report::Succeed)) }
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
        let npc = agent(
            &mut app,
            behavior_tree! { RepeatWhile(high) => { act(refuse) } },
        );
        set(&mut app, npc, true, false);
        app.tick();
        assert_eq!(outcome(&app, npc), Some(false));
    }

    #[test]
    fn a_child_failing_as_it_winds_down_fails_it() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behavior_tree! { RepeatWhile(high) => { task((Work, Graceful)) } },
        );
        set(&mut app, npc, true, false);
        app.tick();
        set(&mut app, npc, false, false);
        app.tick();
        assert!(work_interrupted(&mut app), "told to wind down");
        let mut works = app.world_mut().query_filtered::<&mut Task, With<Work>>();
        works.single_mut(app.world_mut()).unwrap().fail();
        app.tick();
        assert_eq!(outcome(&app, npc), Some(false), "as its child ended");
    }
}
