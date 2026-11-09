<div align="center">

# bevy_arbor

**AI for Bevy: reactive behavior trees, perception, environment queries, smart objects and navigation.**

Behavior trees that sleep until something they wait on changes, with tasks as
entities your own systems tick.

[![Build](https://img.shields.io/github/actions/workflow/status/declaredlost/bevy_arbor/build.yml?branch=main&label=build&style=flat-square)](https://github.com/declaredlost/bevy_arbor/actions/workflows/build.yml)
[![Security](https://img.shields.io/github/actions/workflow/status/declaredlost/bevy_arbor/security.yml?branch=main&label=security&style=flat-square)](https://github.com/declaredlost/bevy_arbor/actions/workflows/security.yml)
[![Crates.io](https://img.shields.io/crates/v/bevy_arbor.svg?style=flat-square)](https://crates.io/crates/bevy_arbor)
[![Docs](https://img.shields.io/docsrs/bevy_arbor?style=flat-square)](https://docs.rs/bevy_arbor)

[**The bevy_arbor book**](https://declaredlost.github.io/bevy_arbor/) · [**docs.rs/bevy_arbor**](https://docs.rs/bevy_arbor) · [**crates.io/crates/bevy_arbor**](https://crates.io/crates/bevy_arbor)

</div>

An agent carries its behavior tree as a component. Each task the tree reaches
is an entity of its own, with a component of yours on it, ticked by your
systems; the tree despawns it when it's done or no longer wanted. Between
ticks a tree sleeps, until a component its decorators read changes, a timer
runs out, or its task reports. Perception writes what an agent senses to it,
changing only when something is noticed, lost or forgotten, so trees reading
it sleep in between.

> [!WARNING]
> **Early development.** The API may change at any time.

## Features

- **Reactive behavior trees**: composites, modifiers and decorators, built
  with the `behavior_tree!` macro, that sleep until something they wait on
  changes.
- **Tasks as entities**: each task a tree starts is an entity with a component
  of yours, ticked by your own systems and despawned when it's no longer
  wanted.
- **Perception**: sight, hearing, touch and sharing between allies, in 2D or
  3D, with memory, sensors and senses of your own.
- **Environment queries**: generate places and things round an agent, filter
  and score them, and pick the best: cover, a target, a place to flank.
- **Smart objects**: slots on things in the world that agents find, claim and
  use, running behavior the object brings.
- **Navigation, your way**: a `MoveTo` task and path costs for queries, over
  whatever pathfinding and movement your game has.
- **Physics with avian**: line of sight through colliders, and touch from
  contacts.
- **Debug drawing**: sight cones, what's perceived, how queries scored,
  slots, and where agents are going, with Bevy's gizmos.

## Requirements

- **Rust + Cargo**: 1.99+, Edition 2024
- **Bevy**: 0.20

## Quick start

Add bevy_arbor with perception:

```toml
[dependencies]
bevy_arbor = { version = "0.1", features = ["perception"] }
```

A guard that patrols, and chases what it sees:

```rust,no_run
use bevy::prelude::*;
use bevy_arbor::prelude::*;

/// A task is a component of yours: the tree spawns an entity with it, and
/// your systems tick it.
#[derive(Component, Clone)]
struct Patrol;

#[derive(Component, Clone)]
struct Chase;

/// A condition: the tree wakes only when what the guard sees changes.
fn sees_intruder(seen: &Perceived<Sight>) -> bool {
    seen.active().any(Percept::is_hostile)
}

fn spawn(mut commands: Commands) {
    commands.spawn((
        Sight::new(12.0).with_lose_range(15.0),
        Team(1),
        Transform::default(),
        BehaviorTree::new(behavior_tree! {
            Forever => {
                Select => {
                    when(sees_intruder) => task(Chase),
                    task(Patrol),
                }
            }
        }),
    ));
}

/// Moves each chasing guard toward what it sees; `patrol` is alike.
fn chase(
    mut tasks: Tasks<&Chase, (&Perceived<Sight>, &mut Transform)>,
    targets: Query<&GlobalTransform>,
) {
    tasks.for_each(|_, _, (seen, mut at)| {
        let target = seen.active().find(|percept| percept.is_hostile());
        if let Some(target) = target.and_then(|percept| targets.get(percept.entity()).ok()) {
            let toward = target.translation() - at.translation;
            at.translation += toward.clamp_length_max(0.1);
        }
    });
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            ArborPlugins.set(PerceptionPlugin::default().with_space(Space::Planar)),
        ))
        .add_systems(Startup, spawn)
        .add_systems(FixedUpdate, chase.after(ArborSystems))
        .run();
}
```

[The bevy_arbor book](https://declaredlost.github.io/bevy_arbor/) is the guide: building trees, tasks and
interruption, perception, environment queries, smart objects and navigation,
one guard
built up chapter by chapter. See its [Installation](https://declaredlost.github.io/bevy_arbor/installation.html) and
[Feature flags](https://declaredlost.github.io/bevy_arbor/features.html) pages for configuration.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and guidelines, and
[CHANGELOG.md](CHANGELOG.md) for release notes.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
