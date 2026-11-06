use std::time::Duration;

use bevy::prelude::*;

use crate::behave::Outcome;

/// On each task entity: whose it is, how long it's run, and how it went.
#[derive(Component, Debug, Clone)]
pub struct Task {
    agent: Entity,
    elapsed: Duration,
    outcome: Option<Outcome>,
    interrupted: bool,
}

impl Task {
    pub(crate) fn new(agent: Entity) -> Self {
        Self {
            agent,
            elapsed: Duration::ZERO,
            outcome: None,
            interrupted: false,
        }
    }

    /// The entity whose tree this task is from.
    #[must_use]
    pub fn agent(&self) -> Entity {
        self.agent
    }

    /// How long it's been running, by the clock its tree ticks on, as of
    /// the tree's last tick: the same whichever schedule reads it.
    #[must_use]
    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    pub(crate) fn set_elapsed(&mut self, elapsed: Duration) {
        self.elapsed = elapsed;
    }

    /// Reports `outcome`, unless it's already reported.
    pub fn report(&mut self, outcome: Outcome) {
        self.outcome.get_or_insert(outcome);
    }

    /// Reports success, unless it's already reported.
    pub fn succeed(&mut self) {
        self.report(Outcome::Success);
    }

    /// Reports failure, unless it's already reported.
    pub fn fail(&mut self) {
        self.report(Outcome::Failure);
    }

    /// How it went, once it's reported.
    #[must_use]
    pub fn outcome(&self) -> Option<Outcome> {
        self.outcome
    }

    /// Whether the tree wants it to stop, for a [`Graceful`] one: it should
    /// wind down, and report when it has.
    #[must_use]
    pub fn interrupted(&self) -> bool {
        self.interrupted
    }

    pub(crate) fn interrupt(&mut self) {
        self.interrupted = true;
    }
}

/// In a task's bundle: when the tree wants it to stop, it isn't despawned
/// but told ([`Task::interrupted`]), and the tree waits for it to report.
/// Only something forcing it, like a `Timeout` or the tree going, still
/// despawns it.
///
/// The tree waits as long as it takes: one that never reports holds its
/// `Select` or `While` where it is for good. Put a `Timeout` around it if
/// it might not.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Graceful;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use crate::testing::*;

    #[test]
    fn the_first_report_counts() {
        let mut task = Task::new(Entity::PLACEHOLDER);
        task.fail();
        task.succeed();
        assert_eq!(task.outcome(), Some(Outcome::Failure));
    }

    #[test]
    fn elapsed_is_how_long_it_has_run_by_the_trees_clock() {
        let mut app = app();
        agent(&mut app, behaviour_tree! { task("Work", Work) });
        app.tick();
        app.secs(1.0);
        let mut tasks = app.world_mut().query::<&Task>();
        let elapsed = tasks.single(app.world()).unwrap().elapsed();
        assert!(
            (0.9..=1.1).contains(&elapsed.as_secs_f32()),
            "elapsed {elapsed:?}"
        );
    }
}
