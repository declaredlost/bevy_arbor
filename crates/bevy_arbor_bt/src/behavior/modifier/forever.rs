use crate::behavior::{Behave, Behavior, Child, Context, Outcome, Status};

/// Its child again each time it ends.
pub(in crate::behavior) struct Forever {
    child: Behavior,
    /// The run it last ticked in, and how many times its child ended in it.
    ended: (u64, u8),
}

impl Forever {
    pub(in crate::behavior) fn new(child: Behavior) -> Self {
        Self {
            child,
            ended: (0, 0),
        }
    }
}

impl Behave for Forever {
    fn tick(&mut self, cx: &mut Context) -> Status {
        if self.ended.0 != cx.run() {
            self.ended = (cx.run(), 0);
        }
        // Twice a run at most, however many passes it takes to hear from its
        // acts: what ends is followed at once, but a tree ending every time
        // can't hang.
        while self.ended.1 < 2 {
            // Asked to stop, it doesn't start its child again.
            if cx.interrupted() && !self.child.is_running() {
                break;
            }
            if self.child.tick(cx) == Status::Running {
                break;
            }
            self.ended.1 += 1;
        }
        // Its child ended twice: it starts it again next tick.
        if !self.child.is_running() {
            cx.poll();
        }
        Status::Running
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
    use bevy::prelude::*;

    use crate::prelude::*;
    use crate::testing::*;

    #[test]
    fn it_follows_an_ending_at_once_and_cannot_hang() {
        let mut app = app();
        agent(&mut app, behavior_tree! { Forever => { act(count) } });
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
            behavior_tree! { Forever => { task((Work, Report::Succeed)) } },
        );
        for _ in 0..5 {
            app.tick();
            assert_eq!(app.count::<With<Work>>(), 1);
        }
    }

    #[test]
    fn asked_to_stop_it_does_not_start_its_child_again() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behavior_tree! {
                Select => {
                    when(high) => task(Other),
                    Forever => { Commit => { task((Work, Report::Succeed)) } },
                }
            },
        );
        app.tick();
        set(&mut app, npc, true, false);
        app.ticks(2);
        assert_eq!(app.count::<With<Other>>(), 1, "the commit ended, so on");
        assert_eq!(app.count::<With<Work>>(), 0);
    }
}
