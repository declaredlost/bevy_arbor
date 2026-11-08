# bevy_arbor_bt

[![Build Status][action-badge]][action-url]
[![Crate Docs][docs-badge]][docs-url]
[![Crate Version][crates-badge]][crates-url]

**Part of [bevy_arbor](https://github.com/declaredlost/bevy_arbor), AI for Bevy.**

[action-badge]: https://img.shields.io/github/actions/workflow/status/declaredlost/bevy_arbor/build.yml?branch=main&label=build&logo=github&style=flat-square
[action-url]: https://github.com/declaredlost/bevy_arbor/actions/workflows/build.yml
[crates-badge]: https://img.shields.io/crates/v/bevy_arbor_bt.svg?logo=rust&style=flat-square
[crates-url]: https://crates.io/crates/bevy_arbor_bt
[docs-badge]: https://img.shields.io/docsrs/bevy_arbor_bt?logo=Docs.rs&style=flat-square
[docs-url]: https://docs.rs/bevy_arbor_bt

Reactive behavior trees for Bevy: trees that sleep until something they wait
on changes, with tasks as entities.

## Overview

- A [`BehaviorTree`] is a component on an agent, built with the
  [`behavior_tree!`] macro or [`Behavior`]'s builders: composites (`Select`,
  `Sequence`, `Fallback`, parallels, scored and random picks), modifiers
  (`Forever`, `Repeat`, `Commit`, `Timeout`, sockets) and leaves.
- A task is an entity the tree spawns to do its work, with a component of
  yours on it. Your systems tick it through [`Tasks`] and report how it went;
  the tree despawns it once it's done or no longer wanted.
- Decorators decide which branches may run: conditions that are plain
  functions of components, system conditions, cooldowns, or your own
  [`Decorate`].
- Between ticks a tree sleeps. It wakes when a component its decorators read
  changes, a timer it set runs out, or its task reports.

## Installation

Most games depend on `bevy_arbor`, where behavior trees are the default
feature and this crate is `bevy_arbor::bt`:

```toml
[dependencies]
bevy_arbor = "0.1"
```

Or depend on this crate directly to use trees alone:

```toml
[dependencies]
bevy_arbor_bt = "0.1"
```

## Usage

```rust,no_run
use bevy::prelude::*;
use bevy_arbor_bt::prelude::*;

#[derive(Component)]
struct Target(Option<Entity>);

#[derive(Component, Clone)]
struct Chase;

#[derive(Component, Clone)]
struct Wander;

/// A condition: the tree wakes when `Target` changes.
fn has_target(target: &Target) -> bool {
    target.0.is_some()
}

/// The chase task, ticked by your system after the trees.
fn chase(mut tasks: Tasks<&Chase, &Target>) {
    tasks.for_each(|_, mut task, target| {
        if target.0.is_none() {
            task.fail();
        }
    });
}

fn spawn(mut commands: Commands) {
    commands.spawn((
        Target(None),
        BehaviorTree::new(behavior_tree! {
            Forever => {
                Select => {
                    when(has_target) => task(Chase),
                    task(Wander),
                }
            }
        }),
    ));
}

fn main() {
    App::new()
        .add_plugins((MinimalPlugins, TreePlugin::default()))
        .add_systems(Startup, spawn)
        .add_systems(FixedUpdate, chase.after(TreeSystems))
        .run();
}
```

`Wander` needs no idea of `Chase`: once `Target` changes so that `has_target`
passes, the tree wakes and cancels it.

## Learn more

- The book's [Building trees](https://declaredlost.github.io/bevy_arbor/trees.html),
  [Decorators](https://declaredlost.github.io/bevy_arbor/decorators.html),
  [Tasks](https://declaredlost.github.io/bevy_arbor/tasks.html),
  [Interruption](https://declaredlost.github.io/bevy_arbor/interruption.html),
  [Sleeping trees](https://declaredlost.github.io/bevy_arbor/sleeping.html),
  [Large trees and sockets](https://declaredlost.github.io/bevy_arbor/large-trees.html)
  and [Custom nodes](https://declaredlost.github.io/bevy_arbor/custom-nodes.html)
  pages, and [Feature flags](https://declaredlost.github.io/bevy_arbor/features.html)
- The [API documentation](https://docs.rs/bevy_arbor_bt)

## Contributing

See [CONTRIBUTING.md](https://github.com/declaredlost/bevy_arbor/blob/main/CONTRIBUTING.md)
for development setup and guidelines.

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/declaredlost/bevy_arbor/blob/main/LICENSE-APACHE)
or [MIT license](https://github.com/declaredlost/bevy_arbor/blob/main/LICENSE-MIT) at your option.

[`BehaviorTree`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/tree/struct.BehaviorTree.html
[`behavior_tree!`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/macro.behavior_tree.html
[`Behavior`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/behavior/struct.Behavior.html
[`Tasks`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/task/struct.Tasks.html
[`Decorate`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/decorator/trait.Decorate.html
