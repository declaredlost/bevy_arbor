use std::time::Duration;

use crate::behavior::{Behave, Behavior, Child, Context, Outcome, Status};

/// Its child, cancelled and failed if it takes more than its length.
pub(in crate::behavior) struct Timeout {
    length: Duration,
    /// When it's up.
    until: Duration,
    child: Behavior,
}

impl Timeout {
    pub(in crate::behavior) fn new(length: Duration, child: Behavior) -> Self {
        Self {
            length,
            until: Duration::ZERO,
            child,
        }
    }
}

impl Behave for Timeout {
    fn start(&mut self, cx: &mut Context) {
        self.until = cx.now().saturating_add(self.length);
    }

    fn tick(&mut self, cx: &mut Context) -> Status {
        // Up: its child is cancelled as it ends.
        if cx.now() >= self.until {
            return Status::Failure;
        }
        let status = self.child.tick(cx);
        if status == Status::Running {
            cx.wake_at(self.until);
        }
        status
    }

    fn end(&mut self, cx: &mut Context, _outcome: Option<Outcome>) {
        self.child.cancel(cx);
    }

    fn label(&self) -> String {
        format!("Timeout({}s)", self.length.as_secs_f32())
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
    fn it_cancels_and_fails() {
        let mut app = app();
        let npc = agent(&mut app, behavior_tree! { Timeout(0.5) => { task(Work) } });
        app.secs(0.25);
        assert_eq!(app.count::<With<Work>>(), 1);
        app.secs(0.5);
        assert_eq!(outcome(&app, npc), Some(false));
        assert_eq!(app.count::<With<Work>>(), 0);
    }
}
