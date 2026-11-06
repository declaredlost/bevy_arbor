use super::Behaviour;
use crate::behave::{Behave, Context, Status};
use crate::tree::TreeOutline;

/// Its children in turn, until one fails.
pub(super) struct Sequence {
    children: Vec<Behaviour>,
    at: usize,
}

impl Sequence {
    pub(super) fn new(children: Vec<Behaviour>) -> Self {
        Self { children, at: 0 }
    }
}

impl Behave for Sequence {
    fn tick(&mut self, cx: &mut Context) -> Status {
        while let Some(child) = self.children.get_mut(self.at) {
            match child.tick(cx) {
                Status::Running => return Status::Running,
                Status::Failure => {
                    self.at = 0;
                    return Status::Failure;
                }
                Status::Success => self.at += 1,
            }
        }
        self.at = 0;
        Status::Success
    }

    fn cancel(&mut self, cx: &mut Context) {
        self.at = 0;
        for child in &mut self.children {
            child.cancel(cx);
        }
    }

    fn outline(&self, out: &mut TreeOutline) {
        out.line("Sequence");
        for child in &self.children {
            out.child(child);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use crate::testing::*;

    #[test]
    fn it_goes_on_only_after_success() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behaviour_tree! {
                Sequence => {
                    act(count),
                    act(refuse),
                    act(count),
                }
            },
        );
        app.tick();
        assert_eq!(acts(&app), 2, "the third never ran");
        assert_eq!(outcome(&app, npc), Some(false));
    }
}
