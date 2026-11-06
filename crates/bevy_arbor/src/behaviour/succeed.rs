use super::Behaviour;
use crate::behave::{Behave, Context, Status};
use crate::tree::TreeOutline;

/// Its child, then success whatever it did.
pub(super) struct Succeed {
    child: Behaviour,
}

impl Succeed {
    pub(super) fn new(child: Behaviour) -> Self {
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

    fn cancel(&mut self, cx: &mut Context) {
        self.child.cancel(cx);
    }

    fn outline(&self, out: &mut TreeOutline) {
        out.line("Succeed");
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
        let npc = agent(&mut app, behaviour_tree! { Succeed => { act(refuse) } });
        app.tick();
        assert_eq!(outcome(&app, npc), Some(true));
    }
}
