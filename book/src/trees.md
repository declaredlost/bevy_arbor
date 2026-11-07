# Building trees

A tree is written as its shape with [`behaviour_tree!`], and put on an agent
as a [`BehaviourTree`]:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component, Clone)] struct Patrol;
# #[derive(Component, Clone)] struct Rest;
# fn spawn(mut commands: Commands) {
commands.spawn(BehaviourTree::new(behaviour_tree! {
    Forever => {
        Sequence => {
            task("Patrol", Patrol),
            Wait(2.0),
            task("Rest", Rest),
        }
    }
}));
# }
```

## Nodes

| Node | What it does |
|---|---|
| `Forever => { .. }` | Runs its child again each time it ends |
| `Select => { .. }` | The first branch whose guard passes, asked again as what it reads changes |
| `Fallback => { .. }` | The same, asking each guard once: what's started isn't interrupted |
| `Scored => { .. }` | Branches from the highest `score(system)` down, until one succeeds |
| `Random(roll) => { .. }` | Children in a random order, until one succeeds |
| `Sequence => { .. }` | Each child in turn, until one fails |
| `All => { .. }` | Children together; fails at the first failure, succeeds once all have |
| `Any => { .. }` | Children together; succeeds at the first success, fails once all have |
| `Race => { .. }` | Children together; the first to end settles it |
| `While(guard) => { .. }` | Its child over and over while the guard passes; succeeds when it stops |
| `Invert => { .. }` | Its child's result, reversed |
| `Succeed => { .. }` | Its child, then success whatever it did |
| `Commit => { .. }` | Its child, not to be interrupted once started |
| `Timeout(length) => { .. }` | Its child, cancelled and failed after `length` |
| `State(bundle) => { .. }` | Its child, with `bundle` on the agent while it runs |
| `Wait(length)` | Succeeds after `length` |
| `task(name, bundle)` | Spawns a task entity with `bundle`, until it reports |
| `act(system)` | Runs a system once; what it returns is its result |
| `@ expr`, `@[ expr ]` | A node built elsewhere, or an iterator of them |

A `length` is a number of seconds (`Wait(1.5)`), checked as it compiles, or
any `Duration` (`Wait(cooldown)`). Several children under a single-child node
run as a `Sequence`.

## Select, Fallback and Scored

Branches take a guard with `when(..) => ..`; one without always passes.

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component, Clone)] struct Flee;
# #[derive(Component, Clone)] struct Fight;
# #[derive(Component, Clone)] struct Idle;
# #[derive(Component)] struct Health(f32);
fn badly_hurt(health: &Health) -> bool {
    health.0 < 20.0
}

# fn tree() -> Behaviour {
behaviour_tree! {
    Select => {
        when(badly_hurt) => task("Flee", Flee),
        task("Idle", Idle),
    }
}
# }
```

`Select` asks its guards again whenever what they read changes: a guard
passing cancels what's running below it, and the branch running is cancelled
if its own guard stops passing. A branch that fails falls through to the next,
and stays out until its guard stops passing or the `Select` restarts, so it
can't restart itself in a loop. A branch that succeeds ends the `Select`.

`Fallback` asks each guard only as it gets to it, so what's started isn't
interrupted. `Scored` scores its branches once, as it starts, with systems
taking the agent and returning a number, and tries them from the highest
down:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component, Clone)] struct Eat;
# #[derive(Component, Clone)] struct Sleep;
fn hunger(In(_): In<Entity>) -> f32 {
    0.7
}

fn tiredness(In(_): In<Entity>) -> f32 {
    0.4
}

# fn tree() -> Behaviour {
behaviour_tree! {
    Scored => {
        score(hunger) => task("Eat", Eat),
        score(tiredness) => task("Sleep", Sleep),
    }
}
# }
```

## State

`State(bundle) => { .. }` puts `bundle` on the agent while its branch runs,
and takes it off as the branch ends or is cancelled. It's for systems that
work only while a branch runs, filtering on `With<Hunting>`:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component, Clone)] struct Approach;
# #[derive(Component, Clone)] struct Pounce;
# fn sees_prey(_: &Perceived<Sight>) -> bool { true }
#[derive(Component, Clone)]
struct Hunting;

# fn tree() -> Behaviour {
behaviour_tree! {
    Select => {
        when(sees_prey) => State(Hunting) => {
            task("Approach", Approach),
            task("Pounce", Pounce),
        },
    }
}
# }
```

A branch of one task needn't: its task entity already marks it, and a system
can filter on that.

## The tree on an agent

- [`TreeFinished`] is triggered on the agent as its tree ends, and
  `BehaviourTree::outcome` says how. One under `Forever` never does.
  `BehaviourTree::restart` starts a finished tree again from the top.
- [`TreePaused`] on the agent holds its tree where it is. Time goes on, so a
  `Wait` may be over when it's taken away.
- Taking the tree away, replacing it, or despawning its agent cancels what
  it's running.
- An agent with a [`TreeTrace`] keeps its tree's outline in it, with `>` on
  what's running, for an inspector.

[`behaviour_tree!`]: https://docs.rs/bevy_arbor/latest/bevy_arbor/macro.behaviour_tree.html
[`BehaviourTree`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/tree/struct.BehaviourTree.html
[`TreeFinished`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/tree/struct.TreeFinished.html
[`TreePaused`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/tree/struct.TreePaused.html
[`TreeTrace`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/tree/struct.TreeTrace.html
