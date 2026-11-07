use std::time::Duration;

use super::Behaviour;
use crate::behaviour::{Behave, Context, Status};
use crate::tree::TreeOutline;

/// Its child, cancelled and failed if it takes more than its length.
pub(super) struct Timeout {
    length: Duration,
    started: Option<Duration>,
    child: Behaviour,
}

impl Timeout {
    pub(super) fn new(length: Duration, child: Behaviour) -> Self {
        Self {
            length,
            started: None,
            child,
        }
    }
}

impl Behave for Timeout {
    fn polls(&self) -> bool {
        false
    }

    fn tick(&mut self, cx: &mut Context) -> Status {
        let now = cx.now();
        let since = *self.started.get_or_insert(now);
        if now.saturating_sub(since) >= self.length {
            self.cancel(cx);
            return Status::Failure;
        }
        let status = self.child.tick(cx);
        if status == Status::Running {
            cx.wake_at(since.saturating_add(self.length));
        } else {
            self.started = None;
        }
        status
    }

    fn cancel(&mut self, cx: &mut Context) {
        self.started = None;
        self.child.cancel(cx);
    }

    fn outline(&self, out: &mut TreeOutline) {
        out.line(format!("Timeout({}s)", self.length.as_secs_f32()));
        out.child(&self.child);
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
        let npc = agent(
            &mut app,
            behaviour_tree! { Timeout(0.5) => { task("Work", Work) } },
        );
        app.secs(0.25);
        assert_eq!(app.count::<With<Work>>(), 1);
        app.secs(0.5);
        assert_eq!(outcome(&app, npc), Some(false));
        assert_eq!(app.count::<With<Work>>(), 0);
    }
}
