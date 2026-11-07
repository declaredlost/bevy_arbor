# Tasks

A task is the tree's work, done by your systems. `task(name, bundle)` spawns an
entity with a [`Task`], your bundle, a `Name` and a [`TaskOf`] its agent, and
waits for it to report.

```rust
# use std::time::Duration;
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
#[derive(Component, Clone)]
struct Dig {
    depth: f32,
}

#[derive(Component)]
struct Shovel(f32);

fn dig(mut tasks: Tasks<&Dig, &Shovel>) {
    tasks.for_each(|dig, mut task, shovel| {
        if task.elapsed() > Duration::from_secs_f32(dig.depth / shovel.0) {
            task.succeed();
        }
    });
}
```

[`Tasks`] pairs each task with what you want of its agent, skipping tasks
that have reported and those of paused trees. It's a shortcut for a query of
tasks and one of agents, paired by `TaskOf`, which you can write yourself.

## Reporting

Report with `succeed` or `fail`, as many times as you like: the first counts,
and wakes the tree, which reads it on its next tick. `task.elapsed()` is how
long it's been running, by the clock trees tick on, whichever schedule your
system runs in.

A task despawned some other way fails.

## Ending

As the tree takes a task away, [`TaskEnded`] is triggered on it, with how it
went, or `None` if it was cancelled before it reported; then it's despawned.
Clean up there, or in an `On<Remove<_>>` observer, if a task has anything to
undo.

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component)] struct Dig;
fn stop_digging(ended: On<TaskEnded>, digs: Query<(), With<Dig>>) {
    if digs.contains(ended.entity) {
        info!("{} stopped digging: {:?}", ended.agent, ended.outcome);
    }
}
```

An agent's [`AgentTasks`] lists the tasks it's running. Despawning the agent
despawns them.

[`Task`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/task/struct.Task.html
[`TaskOf`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/task/struct.TaskOf.html
[`Tasks`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/task/struct.Tasks.html
[`TaskEnded`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/task/struct.TaskEnded.html
[`AgentTasks`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/task/struct.AgentTasks.html
