# bevy_arbor_nav

[![Build Status][action-badge]][action-url]
[![Crate Docs][docs-badge]][docs-url]
[![Crate Version][crates-badge]][crates-url]

**Part of [bevy_arbor](https://github.com/declaredlost/bevy_arbor), AI for Bevy.**

[action-badge]: https://img.shields.io/github/actions/workflow/status/declaredlost/bevy_arbor/build.yml?branch=main&label=build&logo=github&style=flat-square
[action-url]: https://github.com/declaredlost/bevy_arbor/actions/workflows/build.yml
[crates-badge]: https://img.shields.io/crates/v/bevy_arbor_nav.svg?logo=rust&style=flat-square
[crates-url]: https://crates.io/crates/bevy_arbor_nav
[docs-badge]: https://img.shields.io/docsrs/bevy_arbor_nav?logo=Docs.rs&style=flat-square
[docs-url]: https://docs.rs/bevy_arbor_nav

Navigation for Bevy AI: where agents are asked to go, and how getting there
goes, between behavior trees, environment queries and your game's own
movement. It finds no paths and moves nothing: any pathfinding works under it.

## Overview

- The contract is three components on an agent. [`Destination`] is where it's
  asked to go: a point, or an entity to follow. [`Travel`] is how it's going:
  `Moving`, `Arrived` or `Unreachable`, written by your movement. [`Route`] is
  the way it's taking, if your movement says.
- [`MoveTo`] is a behavior tree task. It sets the destination as it starts,
  succeeds once your movement says `Arrived`, fails on `Unreachable`, and
  takes the destination it set off however it ends. With `eqs`,
  `MoveTo::found::<Cover>()` goes where a query found.
- [`PathDistance`] is an environment query test: the walk from the agent to
  each item, as your pathfinder says. Set it once with `set_path_costs`, a
  read-only system answering a whole batch of items at once.
- Your movement does the rest, over any pathfinding: a navmesh, a grid, or a
  platformer's jump graph.

## Installation

Most games depend on `bevy_arbor` with its `nav` feature, where this crate is
`bevy_arbor::nav`:

```toml
[dependencies]
bevy_arbor = { version = "0.1", features = ["nav"] }
```

Or depend on it directly, with `bevy_arbor_bt`:

```toml
[dependencies]
bevy_arbor_nav = "0.1"
```

| Feature | Default | Enables |
|---|---|---|
| `eqs` | Yes | `MoveTo::found` and `PathDistance`, with environment queries |

## Usage

```rust,no_run
use bevy::prelude::*;
use bevy_arbor_bt::prelude::*;
use bevy_arbor_nav::prelude::*;

/// Walks between two posts, waiting at each.
fn sentry() -> Behavior {
    behavior_tree! {
        Forever => {
            Sequence => {
                task(MoveTo::point(Vec3::new(-5.0, 0.0, 0.0))),
                Wait(2.0),
                task(MoveTo::point(Vec3::new(5.0, 0.0, 0.0))),
                Wait(2.0),
            }
        }
    }
}

/// Your movement: straight toward the destination, saying how it goes.
fn walk(
    mut agents: Query<(Entity, &Destination, &mut Transform, Option<&Travel>)>,
    places: Query<&GlobalTransform>,
    time: Res<Time>,
    mut commands: Commands,
) {
    for (agent, destination, mut at, travel) in &mut agents {
        let Some(goal) = destination.goal.position(&places) else {
            continue;
        };
        let toward = goal - at.translation;
        let now = if toward.length() <= destination.within {
            Travel::Arrived
        } else {
            at.translation += toward.clamp_length_max(2.0 * time.delta_secs());
            Travel::Moving
        };
        if travel != Some(&now) {
            commands.entity(agent).insert(now);
        }
    }
}

fn main() {
    App::new()
        .add_plugins((MinimalPlugins, TreePlugin::default(), NavPlugin::default()))
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn((Transform::default(), BehaviorTree::new(sentry())));
        })
        .add_systems(FixedUpdate, walk.after(NavSystems))
        .run();
}
```

## Learn more

- The book's [Navigation](https://declaredlost.github.io/bevy_arbor/navigation.html)
  and [Running queries](https://declaredlost.github.io/bevy_arbor/running-queries.html)
  pages, and [Feature flags](https://declaredlost.github.io/bevy_arbor/features.html)
- The [API documentation](https://docs.rs/bevy_arbor_nav)

## Contributing

See [CONTRIBUTING.md](https://github.com/declaredlost/bevy_arbor/blob/main/CONTRIBUTING.md)
for development setup and guidelines.

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/declaredlost/bevy_arbor/blob/main/LICENSE-APACHE)
or [MIT license](https://github.com/declaredlost/bevy_arbor/blob/main/LICENSE-MIT) at your option.

[`Destination`]: https://docs.rs/bevy_arbor_nav/latest/bevy_arbor_nav/travel/struct.Destination.html
[`Travel`]: https://docs.rs/bevy_arbor_nav/latest/bevy_arbor_nav/travel/enum.Travel.html
[`Route`]: https://docs.rs/bevy_arbor_nav/latest/bevy_arbor_nav/travel/struct.Route.html
[`MoveTo`]: https://docs.rs/bevy_arbor_nav/latest/bevy_arbor_nav/move_to/struct.MoveTo.html
[`PathDistance`]: https://docs.rs/bevy_arbor_nav/latest/bevy_arbor_nav/path/struct.PathDistance.html
