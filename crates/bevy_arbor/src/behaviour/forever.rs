use super::Behaviour;
use crate::behaviour::{Behave, Context, Status};
use crate::tree::TreeOutline;

/// Its child again each time it ends.
pub(super) struct Forever {
    child: Behaviour,
}

impl Forever {
    pub(super) fn new(child: Behaviour) -> Self {
        Self { child }
    }
}

impl Behave for Forever {
    fn tick(&mut self, cx: &mut Context) -> Status {
        // Twice at most: what ends is followed at once, but a tree ending
        // every time can't hang.
        for _ in 0..2 {
            if self.child.tick(cx) == Status::Running {
                break;
            }
        }
        Status::Running
    }

    fn cancel(&mut self, cx: &mut Context) {
        self.child.cancel(cx);
    }

    fn outline(&self, out: &mut TreeOutline) {
        out.line("Forever");
        out.child(&self.child);
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use crate::prelude::*;
    use crate::testing::*;

    #[test]
    fn it_follows_an_ending_at_once_and_cannot_hang() {
        let mut app = app();
        agent(&mut app, behaviour_tree! { Forever => { act(count) } });
        app.tick();
        assert_eq!(acts(&app), 2);
        app.tick();
        assert_eq!(acts(&app), 4);
    }

    #[test]
    fn it_starts_the_next_task_the_tick_one_ends() {
        let mut app = app();
        agent(
            &mut app,
            behaviour_tree! { Forever => { task("Work", (Work, Report::Succeed)) } },
        );
        for _ in 0..5 {
            app.tick();
            assert_eq!(app.count::<With<Work>>(), 1);
        }
    }
}
