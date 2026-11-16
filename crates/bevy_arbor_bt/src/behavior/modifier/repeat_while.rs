use crate::behavior::{Behave, Behavior, Context, Outcome, Status, WindDown};
use crate::decorator::Decorator;
use crate::inspect::{Inspect, Inspection};

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
            return match self.child.wind_down(cx) {
                WindDown::Holding => Status::Running,
                WindDown::Ended(outcome) => outcome.into(),
                WindDown::Free => Status::Success,
            };
        }
        // Asked to stop, it doesn't start its child again.
        if cx.is_interrupted() && !self.child.is_running() {
            return Status::Success;
        }
        // Told it's starting once it's let in: never if it never passed.
        if !self.started {
            self.started = true;
            self.decorator.start(cx);
        }
        match self.child.tick(cx) {
            Status::Failure => Status::Failure,
            Status::Success if cx.is_interrupted() => Status::Success,
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
}

impl Inspect for RepeatWhile {
    fn inspect(&self, inspection: &mut Inspection<'_>) {
        inspection
            .name(format_args!("RepeatWhile {}", self.decorator.label()))
            .child(&self.child);
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use crate::prelude::*;
    use crate::testing::*;

    #[test]
    fn it_repeats_while_its_condition_passes_then_cancels_and_succeeds() {
        let mut app = app();
        let npc = with_flags(
            &mut app,
            behavior_tree! {
                RepeatWhile(high) => { task((Work, Report::Succeed)) }
            },
            true,
            false,
        );
        app.ticks(5);
        assert_eq!(outcome(&app, npc), None);
        assert_eq!(app.count::<With<Work>>(), 1);

        set(&mut app, npc, false, false);
        app.tick();
        assert_eq!(outcome(&app, npc), Some(true));
        assert_eq!(app.count::<With<Work>>(), 0);
    }

    #[test]
    fn its_decorator_is_told_it_started_only_once_it_passed() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behavior_tree! { RepeatWhile(high, Counted) => { task(Work) } },
        );
        app.tick();
        assert_eq!(inside(&app), 0, "never let in");
        assert_eq!(outcome(&app, npc), Some(true));
    }

    #[test]
    fn it_fails_if_its_child_does() {
        let mut app = app();
        let npc = with_flags(
            &mut app,
            behavior_tree! { RepeatWhile(high) => { act(refuse) } },
            true,
            false,
        );
        app.tick();
        assert_eq!(outcome(&app, npc), Some(false));
    }

    #[test]
    fn a_child_failing_as_it_winds_down_fails_it() {
        let mut app = app();
        let npc = with_flags(
            &mut app,
            behavior_tree! { RepeatWhile(high) => { task((Work, Graceful)) } },
            true,
            false,
        );
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
