# bevy_arbor

[![Build Status][action-badge]][action-url]
[![Crate Docs][docs-badge]][docs-url]
[![Crate Version][crates-badge]][crates-url]
[![Book][book-badge]][book-url]

[action-badge]: https://img.shields.io/github/actions/workflow/status/declaredlost/bevy_arbor/build.yml?branch=main&label=build&logo=github&style=flat-square
[action-url]: https://github.com/declaredlost/bevy_arbor/actions/workflows/build.yml
[crates-badge]: https://img.shields.io/crates/v/bevy_arbor.svg?logo=rust&style=flat-square
[crates-url]: https://crates.io/crates/bevy_arbor
[docs-badge]: https://img.shields.io/docsrs/bevy_arbor?logo=Docs.rs&style=flat-square
[docs-url]: https://docs.rs/bevy_arbor
[book-badge]: https://img.shields.io/badge/book-guide-blue?logo=mdbook&style=flat-square
[book-url]: https://declaredlost.github.io/bevy_arbor/

AI for [Bevy](https://bevy.org): reactive behavior trees, perception,
environment queries, smart objects and navigation.

## Overview

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

## Installation

```toml
[dependencies]
bevy_arbor = { version = "0.1", features = ["perception"] }
```

Applications depend on `bevy_arbor`. Each part is a crate of its own,
re-exported here behind a feature: `bevy_arbor::bt`, `perception`, `eqs`,
`smart`, `nav`, `gizmos`, `avian2d` and `avian3d`. The prelude brings in what using
the enabled parts takes, and `ArborPlugins` adds their plugins.

`bt` is enabled by default. The rest are listed on the book's [Feature
flags](https://declaredlost.github.io/bevy_arbor/features.html) page.

## Usage

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

Trees and perception run in `FixedUpdate`, in `ArborSystems`: perception
first, then trees, then the queries trees asked for. Systems that keep facts
on agents go before it, and task systems after it.

## Learn more

[The bevy_arbor book](https://declaredlost.github.io/bevy_arbor/) explains every part:

- **Getting started**: [installation](https://declaredlost.github.io/bevy_arbor/installation.html), [your first
  agent](https://declaredlost.github.io/bevy_arbor/first-agent.html), and [how it fits together](https://declaredlost.github.io/bevy_arbor/concepts.html)
- **Behavior trees**: [building trees](https://declaredlost.github.io/bevy_arbor/trees.html),
  [decorators](https://declaredlost.github.io/bevy_arbor/decorators.html), [tasks](https://declaredlost.github.io/bevy_arbor/tasks.html),
  [interruption](https://declaredlost.github.io/bevy_arbor/interruption.html), [sleeping trees](https://declaredlost.github.io/bevy_arbor/sleeping.html),
  [large trees and sockets](https://declaredlost.github.io/bevy_arbor/large-trees.html), and [custom
  nodes](https://declaredlost.github.io/bevy_arbor/custom-nodes.html)
- **Perception**: [senses](https://declaredlost.github.io/bevy_arbor/senses.html), [sensors, memory and
  events](https://declaredlost.github.io/bevy_arbor/sensors-memory.html), [occlusion and
  attitude](https://declaredlost.github.io/bevy_arbor/occlusion-attitude.html), and [senses of your
  own](https://declaredlost.github.io/bevy_arbor/custom-senses.html)
- **The world**: [queries](https://declaredlost.github.io/bevy_arbor/queries.html), [running
  queries](https://declaredlost.github.io/bevy_arbor/running-queries.html), [smart objects](https://declaredlost.github.io/bevy_arbor/smart-objects.html),
  and [navigation](https://declaredlost.github.io/bevy_arbor/navigation.html)
- **Putting it together**: [the complete guard](https://declaredlost.github.io/bevy_arbor/complete-guard.html),
  [debugging](https://declaredlost.github.io/bevy_arbor/debugging.html), [testing](https://declaredlost.github.io/bevy_arbor/testing.html), and
  [performance](https://declaredlost.github.io/bevy_arbor/performance.html)

The API reference is on [docs.rs](https://docs.rs/bevy_arbor).

## Contributing

See [CONTRIBUTING.md](https://github.com/declaredlost/bevy_arbor/blob/main/CONTRIBUTING.md)
for development setup and guidelines.

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/declaredlost/bevy_arbor/blob/main/LICENSE-APACHE)
or [MIT license](https://github.com/declaredlost/bevy_arbor/blob/main/LICENSE-MIT) at your option.
