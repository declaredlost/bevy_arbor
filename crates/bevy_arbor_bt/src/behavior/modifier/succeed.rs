use crate::behavior::{Behave, Behavior, Context, Outcome, Status};
use crate::inspect::Inspect;

/// Its child, then success whatever it did.
#[derive(Inspect)]
pub(in crate::behavior) struct Succeed {
    #[inspect(child)]
    child: Behavior,
}

impl Succeed {
    pub(in crate::behavior) fn new(child: Behavior) -> Self {
        Self { child }
    }
}

impl Behave for Succeed {
    fn tick(&mut self, cx: &mut Context) -> Status {
        match self.child.tick(cx) {
            Status::Running => Status::Running,
            _ => Status::Success,
        }
    }

    fn end(&mut self, cx: &mut Context, _outcome: Option<Outcome>) {
        self.child.cancel(cx);
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use crate::testing::*;

    #[test]
    fn it_changes_the_result() {
        let mut app = app();
        let npc = agent(&mut app, behavior_tree! { Succeed => { act(refuse) } });
        app.tick();
        assert_eq!(outcome(&app, npc), Some(true));
    }
}
