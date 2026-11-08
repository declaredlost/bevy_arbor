use crate::behavior::{Behave, Behavior, Child, Context, Outcome, Status};

/// Its child, its result reversed.
pub(in crate::behavior) struct Invert {
    child: Behavior,
}

impl Invert {
    pub(in crate::behavior) fn new(child: Behavior) -> Self {
        Self { child }
    }
}

impl Behave for Invert {
    fn tick(&mut self, cx: &mut Context) -> Status {
        match self.child.tick(cx) {
            Status::Running => Status::Running,
            Status::Success => Status::Failure,
            Status::Failure => Status::Success,
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
        let npc = agent(&mut app, behavior_tree! { Invert => { act(refuse) } });
        app.tick();
        assert_eq!(outcome(&app, npc), Some(true));
    }
}
