use crate::behavior::{Behave, Behavior, Child, Context, Outcome, Status};
use crate::decorator::Decorator;

/// Its child over and over, while its decorator passes.
pub(in crate::behavior) struct RepeatWhile {
    decorator: Decorator,
    child: Behavior,
    /// That its decorator was told it started, as it first passed.
    started: bool,
}

impl RepeatWhile {
    pub(in crate::behavior) fn new(decorator: Decorator, child: Behavior) -> Self {
        Self {
            decorator,
            child,
            started: false,
        }
    }
}

impl Behave for RepeatWhile {
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
        // Told it's starting once it's let in: never if it never passed.
        if !self.started {
            self.started = true;
            self.decorator.start(cx);
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
        if std::mem::take(&mut self.started) {
            self.decorator.end(cx, outcome);
        }
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

    /// How many times `Counted` was told its branch started, less ended.
    #[derive(Resource, Default)]
    struct Inside(i32);

    struct Counted;

    impl crate::decorator::Decorate for Counted {
        fn start(&mut self, cx: &mut crate::behavior::Context) {
            cx.world_mut().resource_mut::<Inside>().0 += 1;
        }

        fn passes(&mut self, cx: &mut crate::behavior::Context) -> bool {
            cx.wait_on_nothing();
            true
        }

        fn end(&mut self, cx: &mut crate::behavior::Context, _outcome: Option<Outcome>) {
            cx.world_mut().resource_mut::<Inside>().0 -= 1;
        }
    }

    #[test]
    fn its_decorator_is_told_it_started_only_once_it_passed() {
        let mut app = app();
        app.app_mut().init_resource::<Inside>();
        let npc = agent(
            &mut app,
            behavior_tree! { RepeatWhile(high, Counted) => { task(Work) } },
        );
        app.tick();
        assert_eq!(app.world().resource::<Inside>().0, 0, "never let in");
        assert_eq!(outcome(&app, npc), Some(true));
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
