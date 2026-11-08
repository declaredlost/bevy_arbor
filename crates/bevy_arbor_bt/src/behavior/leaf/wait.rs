use std::time::Duration;

use crate::behavior::{Behave, Context, Outcome, Status};

/// Succeeds after its length.
pub(in crate::behavior) struct Wait {
    length: Duration,
    /// When it's over.
    until: Duration,
}

impl Wait {
    pub(in crate::behavior) fn new(length: Duration) -> Self {
        Self {
            length,
            until: Duration::ZERO,
        }
    }
}

impl Behave for Wait {
    fn start(&mut self, cx: &mut Context) {
        self.until = cx.now().saturating_add(self.length);
    }

    fn tick(&mut self, cx: &mut Context) -> Status {
        if cx.now() < self.until {
            cx.wake_at(self.until);
            return Status::Running;
        }
        Status::Success
    }

    fn end(&mut self, _cx: &mut Context, _outcome: Option<Outcome>) {}

    fn label(&self) -> String {
        format!("Wait({}s)", self.length.as_secs_f32())
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use crate::testing::*;

    #[test]
    fn it_succeeds_after_its_time() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behavior_tree! { Sequence => { Wait(0.5), act(count) } },
        );
        app.secs(0.25);
        assert_eq!(acts(&app), 0);
        app.secs(0.5);
        assert_eq!(acts(&app), 1);
        assert_eq!(outcome(&app, npc), Some(true));
    }
}
