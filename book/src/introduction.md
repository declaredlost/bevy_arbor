# Introduction

bevy_arbor is AI for [Bevy](https://bevyengine.org): behaviour trees that
react to what changes, and perception that tells them what changed. Each part
is its own crate, and `bevy_arbor` brings them together.

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
#[derive(Component, Clone)]
struct Chase;

#[derive(Component, Clone)]
struct Wander;

fn sees_enemy(seen: &Perceived<Sight>) -> bool {
    seen.active().any(Percept::is_hostile)
}

fn spawn(mut commands: Commands) {
    commands.spawn((
        Sight::new(12.0),
        Team(1),
        Transform::default(),
        BehaviourTree::new(behaviour_tree! {
            Forever => {
                Select => {
                    when(sees_enemy) => task("Chase", Chase),
                    task("Wander", Wander),
                }
            }
        }),
    ));
}
```

The monster wanders until it sees an enemy, then chases it, and wanders again
once it's lost it. Nothing in the tree is asked every frame: the tree sleeps
until what `sees_enemy` reads, the monster's `Perceived<Sight>`, changes.

## What bevy_arbor does for you

- **Reactive behaviour trees**: a `Select` switches branches the moment a
  guard changes its answer, cancelling what was running.
- **Trees that sleep**: a tree is ticked only when something it waits on
  comes: a component its guards read changes, its task reports, a timer is up.
  An idle tree costs about 20 nanoseconds a tick.
- **Tasks as entities**: each task the tree reaches is an entity, ticked by
  your systems, and despawned when it's done or no longer wanted.
- **Perception**: sight, hearing, touch and allies sharing what they know, in
  2D or 3D, written to the agent one component per sense.
- **Sensors and senses of your own**: eyes on a head, a rear motion sensor,
  a sense of smell or a radar, all through the same API as the built-in ones.
- **Physics-free**: line of sight, attitudes and senses plug in as ordinary
  systems, so it works with any physics engine, or none.

## The crates

Most projects depend only on `bevy_arbor`, which re-exports the rest behind
feature flags:

| Crate | Contains |
|---|---|
| `bevy_arbor` | The facade: every part, one prelude, one plugin group |
| `bevy_arbor_bt` | Behaviour trees, tasks and guards |
| `bevy_arbor_perception` | Perception: senses, sensors, memory |
| `bevy_arbor_macros` | The `behaviour_tree!` macro |

The API reference is on [docs.rs](https://docs.rs/bevy_arbor). This book is
the guide: how the pieces fit together, chapter by chapter.
