use crate::behave::{Behave, Context, Outcome, Status};
use crate::guard::Check;
use crate::tree::TreeOutline;

/// Runs a system once: what it returns is the result.
pub(super) struct Act(Check<bool>);

impl Act {
    pub(super) fn new(check: Check<bool>) -> Self {
        Self(check)
    }
}

impl Behave for Act {
    fn tick(&mut self, cx: &mut Context) -> Status {
        let agent = cx.agent();
        Outcome::from(self.0.run(cx.world_mut(), agent).unwrap_or(false)).into()
    }

    fn cancel(&mut self, _cx: &mut Context) {}

    fn outline(&self, out: &mut TreeOutline) {
        out.line(format!("act {}", self.0.name()));
    }
}
