use crate::behavior::system::Check;
use crate::behavior::{Behave, Context, Outcome, Status};

/// Runs a system once: what it returns is the result.
pub(in crate::behavior) struct Act(Check<bool>);

impl Act {
    pub(in crate::behavior) fn new(check: Check<bool>) -> Self {
        Self(check)
    }
}

impl Behave for Act {
    fn tick(&mut self, cx: &mut Context) -> Status {
        Outcome::from(self.0.run(cx).unwrap_or(false)).into()
    }

    fn end(&mut self, _cx: &mut Context, _outcome: Option<Outcome>) {}

    fn label(&self) -> String {
        format!("act {}", self.0.name())
    }
}
