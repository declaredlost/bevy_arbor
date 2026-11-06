use std::time::Duration;

use crate::behaviour::{Behave, Context, Status};
use crate::tree::TreeOutline;

/// Succeeds after its length.
pub(super) struct Wait {
    length: Duration,
    started: Option<Duration>,
}

impl Wait {
    pub(super) fn new(length: Duration) -> Self {
        Self {
            length,
            started: None,
        }
    }
}

impl Behave for Wait {
    fn tick(&mut self, cx: &mut Context) -> Status {
        let now = cx.now();
        let since = *self.started.get_or_insert(now);
        if now.saturating_sub(since) < self.length {
            return Status::Running;
        }
        self.started = None;
        Status::Success
    }

    fn cancel(&mut self, _cx: &mut Context) {
        self.started = None;
    }

    fn outline(&self, out: &mut TreeOutline) {
        out.line(format!("Wait({}s)", self.length.as_secs_f32()));
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
            behaviour_tree! { Sequence => { Wait(0.5), act(count) } },
        );
        app.secs(0.25);
        assert_eq!(acts(&app), 0);
        app.secs(0.5);
        assert_eq!(acts(&app), 1);
        assert_eq!(outcome(&app, npc), Some(true));
    }
}
