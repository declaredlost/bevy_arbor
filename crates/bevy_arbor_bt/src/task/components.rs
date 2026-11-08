use std::time::Duration;

use bevy::prelude::*;

use crate::behavior::Outcome;

/// On each task entity: how long it has run, and how it went.
///
/// Whose it is is its [`TaskOf`].
#[derive(Component, Reflect, Debug, Clone, Default)]
#[reflect(Component, Debug, Default)]
pub struct Task {
    /// When it started, by the clock its tree ticks on.
    started: Duration,
    elapsed: Duration,
    outcome: Option<Outcome>,
    interrupted: bool,
}

impl Task {
    /// A task starting at `started`, by the clock its tree ticks on.
    pub(crate) fn starting(started: Duration) -> Self {
        Self {
            started,
            ..Self::default()
        }
    }

    /// How long it's been running, by the clock its tree ticks on, as of the
    /// last time trees ticked: the same whichever schedule reads it.
    #[must_use]
    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// Brings [`elapsed`](Self::elapsed) up to `clock`.
    pub(crate) fn advance(&mut self, clock: Duration) {
        self.elapsed = clock.saturating_sub(self.started);
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
    ///
    /// Asked again each tick, so it goes back to `false` if what wanted it
    /// stopped no longer does, before it has reported.
    #[must_use]
    pub fn interrupted(&self) -> bool {
        self.interrupted
    }

    pub(crate) fn set_interrupted(&mut self, interrupted: bool) {
        self.interrupted = interrupted;
    }
}

/// On a task: the agent whose tree spawned it, as a Bevy relationship.
///
/// Despawning the agent despawns its tasks; its [`AgentTasks`] are kept by
/// this, not to be set by hand.
#[derive(Component, Reflect, Debug, Clone, Copy, PartialEq, Eq)]
#[reflect(Component, Debug, PartialEq)]
#[relationship(relationship_target = AgentTasks)]
pub struct TaskOf(Entity);

impl TaskOf {
    pub(crate) fn new(agent: Entity) -> Self {
        Self(agent)
    }

    /// The agent whose tree the task is from.
    #[must_use]
    pub fn agent(&self) -> Entity {
        self.0
    }
}

/// On an agent: the tasks its tree is running, kept by [`TaskOf`].
///
/// They go with it.
#[derive(Component, Reflect, Debug, Default)]
#[reflect(Component, Debug)]
#[relationship_target(relationship = TaskOf, linked_spawn)]
pub struct AgentTasks(Vec<Entity>);

impl AgentTasks {
    /// The tasks, in the order they started.
    pub fn iter(&self) -> impl Iterator<Item = Entity> + '_ {
        self.0.iter().copied()
    }
}

/// Marks a task to be told to stop and waited for, instead of despawned.
///
/// In a task's bundle: when the tree wants it to stop, it isn't despawned but
/// told ([`Task::interrupted`]), and the tree waits for it to report. Only
/// force still despawns it: a `Timeout`, an `All`, `Any` or `Race` settling,
/// anything above being canceled, or the tree going. Under a `Commit` it isn't
/// told at all.
///
/// The tree waits as long as it takes: one that never reports holds its
/// `Select` or `RepeatWhile` where it is for good. Put a `Timeout` around it if
/// it might not.
#[derive(Component, Reflect, Debug, Clone, Copy, Default)]
#[reflect(Component, Debug, Default)]
pub struct Graceful;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use crate::testing::*;

    #[test]
    fn the_first_report_counts() {
        let mut task = Task::default();
        task.fail();
        task.succeed();
        assert_eq!(task.outcome(), Some(Outcome::Failure));
    }

    #[test]
    fn elapsed_is_how_long_it_has_run_by_the_trees_clock() {
        let mut app = app();
        agent(&mut app, behavior_tree! { task(Work) });
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
