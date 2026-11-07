use super::Behaviour;
use crate::behaviour::{Behave, Context, Status};
use crate::tree::TreeOutline;

/// Its child, its result reversed.
pub(super) struct Invert {
    child: Behaviour,
}

impl Invert {
    pub(super) fn new(child: Behaviour) -> Self {
        Self { child }
    }
}

impl Behave for Invert {
    fn polls(&self) -> bool {
        false
    }

    fn tick(&mut self, cx: &mut Context) -> Status {
        match self.child.tick(cx) {
            Status::Running => Status::Running,
            Status::Success => Status::Failure,
            Status::Failure => Status::Success,
        }
    }

    fn cancel(&mut self, cx: &mut Context) {
        self.child.cancel(cx);
    }

    fn outline(&self, out: &mut TreeOutline) {
        out.line("Invert");
        out.child(&self.child);
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use crate::testing::*;

    #[test]
    fn it_changes_the_result() {
        let mut app = app();
        let npc = agent(&mut app, behaviour_tree! { Invert => { act(refuse) } });
        app.tick();
        assert_eq!(outcome(&app, npc), Some(true));
    }
}
