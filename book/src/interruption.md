# Interruption

A `Select` or `While` cancels what's running when its guards say so. Three
ways keep work from being cut off.

## Fallback

Nothing in a `Fallback` is interrupted by its own guards: each is asked once,
as it gets there.

## Commit

`Commit => { .. }` isn't interrupted once started. It isn't even told it's
wanted to stop, so a graceful task in it doesn't wind down. In a `Sequence`,
what comes after it still can be.

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component, Clone)] struct Swing;
# #[derive(Component, Clone)] struct Wander;
# fn sees_enemy(_: &Perceived<Sight>) -> bool { true }
# fn tree() -> Behaviour {
behaviour_tree! {
    Select => {
        when(sees_enemy) => Commit => { task("Swing", Swing) },
        task("Wander", Wander),
    }
}
# }
```

A swing, once started, finishes even if the enemy leaves.

## Graceful tasks

[`Graceful`] in a task's bundle: the task is told (`Task::interrupted`),
winds down and reports, and only then does the tree move on.

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
#[derive(Component, Clone)]
struct Climb;

fn climb(mut tasks: Tasks<&Climb, ()>) {
    tasks.for_each(|_, mut task, ()| {
        if task.interrupted() {
            // Climb back down first.
            task.succeed();
        }
    });
}

# fn tree() -> Behaviour {
behaviour_tree! { task("Climb", (Climb, Graceful)) }
# }
```

A task told to stop is told it isn't any more if what asked stops asking
first. One that never reports holds the tree there for good, so put a
`Timeout` around one that might not.

Meanwhile nothing new starts in what's winding down: a `Sequence` doesn't go
on to its next child, nor `Forever` start its own again. However it ends,
what asked it to stop is next.

## Force

Only force gets past these: a `Timeout`, an `All`, `Any` or `Race` settling
(cancelling the rest), anything above being cancelled, or the tree going.

[`Graceful`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/task/struct.Graceful.html
