# Trees

A behavior tree says what the guard does, and in what order of priority. You
write its shape with [`behavior_tree!`] and put it on the guard as a
[`BehaviorTree`].

## A guard that fights

The guard chases and shoots an intruder it sees, searches for one it
remembers, and patrols otherwise:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component, Clone)] struct Patrol { from: f32, to: f32 }
# #[derive(Component, Clone)] struct Chase;
# #[derive(Component, Clone)] struct Search;
# #[derive(Component, Clone)] struct Shoot;
# fn sees_intruder(seen: &Perceived<Sight>) -> bool { seen.active().any(Percept::is_hostile) }
# fn remembers_intruder(seen: &Perceived<Sight>) -> bool { seen.iter().any(|p| p.is_hostile() && !p.is_active()) }
fn guard() -> Behavior {
    behavior_tree! {
        Forever => {
            Select => {
                when(sees_intruder) => Sequence => {
                    task(Chase),
                    task(Shoot),
                    Wait(0.8),
                },
                when(remembers_intruder) => task(Search),
                task(Patrol { from: -10.0, to: 10.0 }),
            }
        }
    }
}
```

Read it from the top:

- `Forever` runs its child again each time it ends, so the guard never stops.
- `Select` runs the first branch whose decorator passes. A branch without
  `when` always passes, so patrolling is the fallback.
- `Sequence` runs its children in turn: chase, shoot, then wait 0.8 seconds.
- `task(..)` is work your systems do. See [Tasks](tasks.md).

Put the tree on the guard as you spawn it:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component)] struct Guard;
# fn guard() -> Behavior { behavior_tree! { Wait(1.0) } }
fn spawn_guard(mut commands: Commands) {
    commands.spawn((Guard, Transform::default(), BehaviorTree::new(guard())));
}
```

## Nodes

Composites have several children:

| Node | What it does |
|---|---|
| `Select => { .. }` | Runs the first branch whose decorator passes, and switches when that changes |
| `Fallback => { .. }` | Like `Select`, but never interrupts what it started |
| `Scored => { .. }` | Tries branches from the highest `score(..)` down, until one succeeds |
| `Random => { .. }` | Tries children in a random order, until one succeeds. `Random(seed)` for a seed of its own |
| `Sequence => { .. }` | Runs children in turn, until one fails |
| `All => { .. }` | Runs children together. Fails at the first failure, succeeds once all succeed |
| `Any => { .. }` | Runs children together. Succeeds at the first success, fails once all fail |
| `Race => { .. }` | Runs children together. The first to end decides |

Modifiers have one child:

| Node | What it does |
|---|---|
| `Forever => { .. }` | Runs its child again each time it ends |
| `Repeat(times) => { .. }` | Runs its child `times` times. Fails at its first failure |
| `RepeatWhile(decorator) => { .. }` | Runs its child over and over while the decorator passes |
| `Invert => { .. }` | Reverses its child's result |
| `Succeed => { .. }` | Succeeds whatever its child did |
| `Commit => { .. }` | Can't be interrupted once started. See [Interruption](interruption.md) |
| `Timeout(length) => { .. }` | Cancels and fails its child after `length` |
| `State(bundle) => { .. }` | Puts `bundle` on the agent while its child runs. See [Tasks](tasks.md#state) |
| `Socket(key)` | Runs the subtree plugged into the agent. See [Large trees](large-trees.md#sockets) |

Leaves do the work:

| Node | What it does |
|---|---|
| `task(bundle)` | Spawns a task entity and waits for it to report. See [Tasks](tasks.md) |
| `act(system)` | Runs a system once, with the tree's commands. Its `bool` is the result |
| `Wait(length)` | Succeeds after `length` |
| `@ expr` | Puts in a subtree built elsewhere. See [Large trees](large-trees.md) |

A `length` is seconds, like `Wait(0.8)`, or any `Duration`. A number is
checked as the tree compiles, so a negative wait is a compile error. If you
give a one-child node several children, they run as a `Sequence`.

## Select and Fallback

`Select` keeps asking its decorators. When the guard spots an intruder, the
`Select` cancels the patrol and starts the chase. When the intruder is gone,
it cancels the chase. This makes the guard react at once.

A branch that fails falls through to the next one. It stays out until its
decorator stops passing, so it can't restart itself in a loop. A branch that
succeeds ends the `Select`.

`Fallback` asks each decorator only when it gets to it. What it started runs
to the end. Use it where reacting isn't wanted. [Interruption](interruption.md)
compares the two.

## Running things together

`All`, `Any` and `Race` run their children at the same time. The guard shouts
for help while it chases:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component, Clone)] struct Chase;
# #[derive(Component, Clone)] struct Shout;
# fn tree() -> Behavior {
behavior_tree! {
    All => {
        task(Chase),
        task(Shout),
    }
}
# }
```

When a parallel node settles, it cancels the children still running.

## Scored and Random

`Scored` asks a score for each branch as it starts, and tries them from the
highest down. A typed score is a function of the agent's components, as a
typed condition is, returning an `f32`. `Random` shuffles its children:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component, Clone)] struct Patrol { from: f32, to: f32 }
# #[derive(Component, Clone)] struct Search;
#[derive(Component)]
struct Mood {
    suspicion: f32,
    boredom: f32,
}

fn suspicion(mood: &Mood) -> f32 {
    mood.suspicion
}

fn boredom(mood: &Mood) -> f32 {
    mood.boredom
}

# fn tree() -> Behavior {
behavior_tree! {
    Scored => {
        score(suspicion) => task(Search),
        score(boredom) => Random => {
            task(Patrol { from: -10.0, to: 0.0 }),
            task(Patrol { from: 0.0, to: 10.0 }),
        },
    }
}
# }
```

- A branch whose score can't be read, or is NaN, goes last.
- A score of your own, reading beyond the agent, implements [`Score`].
- `Random` shuffles with a generator of its own, seeded from the agent, so
  the same tree on the same agent shuffles the same, run after run: a replay
  replays. `Random(seed)` seeds it with a number of yours.

## Acts

`act(system)` runs a system once on the agent, and its `bool` is the result.
Trees tick side by side, reading the world, so an act doesn't run as the tree
ticks: it runs with the tree's commands, as a cached Bevy system
(`World::run_system_cached_with`), once the tick is done. The tree is then
ticked again, in the same run, to go on from what it said: a `Sequence` of
acts still runs in one frame, and what each changes, the next sees.

- It can take `Commands`, `ResMut`, or a `&mut` query: it runs alone.
- It can return `Result<bool>`, so you can use `?`. An error fails, and goes
  to Bevy's error handler.
- It's one instance, shared by every tree and agent, so its `Local`s are
  shared too, and it can't capture anything. `act(system, param)` passes a
  parameter in `In` with the agent.
- What a task does over time is a task, run by your own system over every
  task at once: see [Tasks](tasks.md). An act is for a quick change.

## Without the macro

The macro calls [`Behavior`]'s builders. You can call them yourself, for
trees built from data:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component, Clone)] struct Patrol { from: f32, to: f32 }
# #[derive(Component, Clone)] struct Chase;
# fn sees_intruder(seen: &Perceived<Sight>) -> bool { seen.active().any(Percept::is_hostile) }
use bevy_arbor::bt::behavior::Branch;

fn guard() -> Behavior {
    Behavior::forever(Behavior::select(vec![
        Branch::when(sees_intruder, Behavior::task(Chase)),
        Branch::always(Behavior::task(Patrol { from: -10.0, to: 10.0 })),
    ]))
}
```

## The tree on the guard

- [`TreeFinished`] is triggered on the guard when its tree ends.
  `BehaviorTree::outcome` says how it ended. A tree under `Forever` never ends.
- `BehaviorTree::restart` starts a finished tree again.
- [`TreePaused`] on the guard holds its tree where it is: no decorators
  asked, nothing started, and its task skipped by `Tasks`. Time still passes,
  so a `Wait` may be over when you remove it.
- Removing the tree, replacing it, or despawning the guard cancels what the
  tree was running.

## Pitfalls

- A `Select` ends when a branch succeeds. Without `Forever` around it, the
  guard stops after its first chase.
- Order matters in a `Select`. A branch without `when` catches everything,
  so put it last.

[`behavior_tree!`]: https://docs.rs/bevy_arbor/latest/bevy_arbor/macro.behavior_tree.html
[`BehaviorTree`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/tree/struct.BehaviorTree.html
[`Behavior`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/behavior/struct.Behavior.html
[`TreeFinished`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/tree/struct.TreeFinished.html
[`TreePaused`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/tree/struct.TreePaused.html
[`Score`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/behavior/trait.Score.html
