use crate::behavior::{Behave, Behavior, Child, Context, Outcome, Status};

/// Its child, then success whatever it did.
pub(in crate::behavior) struct Succeed {
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

    fn children(&self) -> Vec<Child<'_>> {
        vec![Child::new(&self.child)]
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
