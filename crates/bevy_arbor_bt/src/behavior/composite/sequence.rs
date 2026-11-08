use crate::behavior::{Behave, Behavior, Child, Context, Outcome, Status};

/// Its children in turn, until one fails.
pub(in crate::behavior) struct Sequence {
    children: Vec<Behavior>,
    at: usize,
}

impl Sequence {
    pub(in crate::behavior) fn new(children: Vec<Behavior>) -> Self {
        Self { children, at: 0 }
    }
}

impl Behave for Sequence {
    fn start(&mut self, _cx: &mut Context) {
        self.at = 0;
    }

    fn tick(&mut self, cx: &mut Context) -> Status {
        while let Some(child) = self.children.get_mut(self.at) {
            match child.tick(cx) {
                Status::Running => return Status::Running,
                Status::Failure => return Status::Failure,
                // Asked to stop, it doesn't start the next.
                Status::Success if cx.interrupted() => break,
                Status::Success => self.at += 1,
            }
        }
        Status::Success
    }

    fn end(&mut self, cx: &mut Context, _outcome: Option<Outcome>) {
        for child in &mut self.children {
            child.cancel(cx);
        }
    }

    fn children(&self) -> Vec<Child<'_>> {
        self.children.iter().map(Child::new).collect()
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
            behavior_tree! {
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
