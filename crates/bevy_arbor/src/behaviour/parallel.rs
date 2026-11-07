use super::Behaviour;
use crate::behaviour::{Behave, Context, Status};
use crate::tree::TreeOutline;

/// What settles a [`Parallel`] early.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Mode {
    /// A failure.
    All,
    /// A success.
    Any,
    /// Any ending.
    Race,
}

impl Mode {
    fn settles(self, status: Status) -> bool {
        match self {
            Self::All => status == Status::Failure,
            Self::Any => status == Status::Success,
            Self::Race => status != Status::Running,
        }
    }

    /// The result when they all end with nothing settling it.
    fn exhausted(self) -> Status {
        match self {
            Self::All | Self::Race => Status::Success,
            Self::Any => Status::Failure,
        }
    }
}

/// Its children together.
pub(super) struct Parallel {
    mode: Mode,
    children: Vec<Behaviour>,
    /// Which children have ended.
    ended: Vec<bool>,
}

impl Parallel {
    pub(super) fn new(mode: Mode, children: Vec<Behaviour>) -> Self {
        let ended = vec![false; children.len()];
        Self {
            mode,
            children,
            ended,
        }
    }
}

impl Behave for Parallel {
    fn tick(&mut self, cx: &mut Context) -> Status {
        let mut running = false;
        for (child, ended) in self.children.iter_mut().zip(&mut self.ended) {
            if *ended {
                continue;
            }
            let status = child.tick(cx);
            if self.mode.settles(status) {
                self.cancel(cx);
                return status;
            }
            match status {
                Status::Running => running = true,
                _ => *ended = true,
            }
        }
        if running {
            return Status::Running;
        }
        self.ended.fill(false);
        self.mode.exhausted()
    }

    fn cancel(&mut self, cx: &mut Context) {
        self.ended.fill(false);
        for child in &mut self.children {
            child.cancel(cx);
        }
    }

    fn outline(&self, out: &mut TreeOutline) {
        out.line(match self.mode {
            Mode::All => "All",
            Mode::Any => "Any",
            Mode::Race => "Race",
        });
        for child in &self.children {
            out.child(child);
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use crate::prelude::*;
    use crate::testing::*;

    #[test]
    fn all_fails_at_the_first_failure_and_cancels_the_rest() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behaviour_tree! {
                All => {
                    task("Work", Work),
                    task("Other", (Other, Report::Fail)),
                }
            },
        );
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 1);
        app.tick();
        assert_eq!(outcome(&app, npc), Some(false));
        assert_eq!(app.count::<With<Work>>(), 0, "cancelled");
    }

    #[test]
    fn all_succeeds_once_every_child_has() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behaviour_tree! { All => { act(count), act(count) } },
        );
        app.tick();
        assert_eq!(outcome(&app, npc), Some(true));
        assert_eq!(acts(&app), 2);
    }

    #[test]
    fn any_succeeds_at_the_first_success_and_cancels_the_rest() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behaviour_tree! {
                Any => {
                    task("Work", Work),
                    task("Other", (Other, Report::Succeed)),
                }
            },
        );
        app.ticks(2);
        assert_eq!(outcome(&app, npc), Some(true));
        assert_eq!(app.count::<With<Work>>(), 0, "cancelled, not left running");
    }

    #[test]
    fn any_fails_once_every_child_has() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behaviour_tree! { Any => { act(refuse), act(refuse) } },
        );
        app.tick();
        assert_eq!(outcome(&app, npc), Some(false));
        assert_eq!(acts(&app), 2);
    }

    #[test]
    fn race_takes_the_first_ending_either_way() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behaviour_tree! {
                Race => {
                    task("Work", Work),
                    act(refuse),
                }
            },
        );
        app.tick();
        assert_eq!(outcome(&app, npc), Some(false));
        assert_eq!(app.count::<With<Work>>(), 0, "cancelled");
    }

    #[test]
    fn it_runs_again_from_the_start() {
        let mut app = app();
        agent(
            &mut app,
            behaviour_tree! { Forever => { All => { act(count), Wait(0.0) } } },
        );
        app.tick();
        assert!(acts(&app) >= 2, "ran twice: {}", acts(&app));
    }

    #[test]
    fn settling_cancels_even_a_committed_child() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behaviour_tree! {
                All => {
                    Commit => { task("Work", Work) },
                    act(refuse),
                }
            },
        );
        app.tick();
        assert_eq!(outcome(&app, npc), Some(false));
        assert_eq!(app.count::<With<Work>>(), 0, "forced");
    }
}
