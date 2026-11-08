# Interruption

A `Select` cancels the running branch as soon as a better one passes. That
keeps the guard alert, but some work shouldn't be cut off. A shot already
fired should land. A reload half done should finish.

## Commit

`Commit => { .. }` can't be interrupted once it starts. The guard always
finishes a shot:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component, Clone)] struct Patrol { from: f32, to: f32 }
# #[derive(Component, Clone)] struct Chase;
# #[derive(Component, Clone)] struct Shoot;
# fn sees_intruder(seen: &Perceived<Sight>) -> bool { seen.active().any(Percept::is_hostile) }
# fn tree() -> Behavior {
behavior_tree! {
    Forever => {
        Select => {
            when(sees_intruder) => Sequence => {
                task(Chase),
                Commit => { task(Shoot) },
                Wait(0.8),
            },
            task(Patrol { from: -10.0, to: 10.0 }),
        }
    }
}
# }
```

If the intruder ducks out of sight mid-shot, the shot finishes first. Then
the `Select` switches to patrolling. The `Chase` and the `Wait` around it can
still be interrupted.

A task inside `Commit` isn't even told that something wants it to stop.

## Graceful tasks

A task with [`Graceful`] in its bundle is asked to stop instead of being
canceled. `task.interrupted()` turns true. The task winds down, reports, and
only then does the tree move on.

The guard reloads gracefully. Past halfway, it finishes the reload even when
asked to stop:

```rust
# use std::time::Duration;
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component)] struct Ammo(u32);
# fn out_of_ammo(ammo: &Ammo) -> bool { ammo.0 == 0 }
#[derive(Component, Clone)]
struct Reload;

const RELOAD: Duration = Duration::from_secs(2);

fn reload(mut tasks: Tasks<&Reload, &mut Ammo>) {
    tasks.for_each(|_, mut task, mut ammo| {
        if task.elapsed() >= RELOAD {
            ammo.0 = 6;
            task.succeed();
        } else if task.interrupted() && task.elapsed() < RELOAD / 2 {
            // Not far in: give up the reload.
            task.fail();
        }
    });
}

# fn tree() -> Behavior {
behavior_tree! {
    Select => {
        when(out_of_ammo) => task((Reload, Graceful)),
    }
}
# }
```

- If whatever asked it to stop changes its mind, `interrupted()` turns false
  again.
- While a task winds down, nothing new starts around it. A `Sequence` doesn't
  move to its next child, and `Forever` doesn't restart.

## Select or Fallback

[Building trees](trees.md#select-and-fallback) explains how each asks its
decorators. Which to reach for:

| You want | Use |
|---|---|
| The guard to drop the patrol the moment it sees someone | `Select` |
| A list of things to try in order, each run to the end | `Fallback` |
| One part of a `Select` branch to finish | `Commit` around that part |
| A task to finish cleanly when interrupted | `Graceful` |

## Holding

`Commit` and `Graceful` work by holding: a node that holds asks not to be
stopped this tick. Your own nodes can hold too. See
[Custom nodes](custom-nodes.md#holding-and-interruption).

Holding only stops `Select` and `RepeatWhile`. Force still gets through:

- a `Timeout` running out,
- an `All`, `Any` or `Race` settling and canceling the rest,
- anything above being canceled,
- the tree being removed, replaced or despawned.

## Pitfalls

- A graceful task that never reports holds its branch forever. Put a
  `Timeout` around one that might not.
- `Commit` around a whole branch makes the guard deaf to everything above it
  until the branch ends. Commit to the smallest part that needs it.

[`Graceful`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/task/struct.Graceful.html
