# bevy_arbor_gizmos

[![Build Status][action-badge]][action-url]
[![Crate Docs][docs-badge]][docs-url]
[![Crate Version][crates-badge]][crates-url]

**Part of [bevy_arbor](https://github.com/declaredlost/bevy_arbor), AI for Bevy.**

[action-badge]: https://img.shields.io/github/actions/workflow/status/declaredlost/bevy_arbor/build.yml?branch=main&label=build&logo=github&style=flat-square
[action-url]: https://github.com/declaredlost/bevy_arbor/actions/workflows/build.yml
[crates-badge]: https://img.shields.io/crates/v/bevy_arbor_gizmos.svg?logo=rust&style=flat-square
[crates-url]: https://crates.io/crates/bevy_arbor_gizmos
[docs-badge]: https://img.shields.io/docsrs/bevy_arbor_gizmos?logo=Docs.rs&style=flat-square
[docs-url]: https://docs.rs/bevy_arbor_gizmos

Debug drawing for Bevy AI, with Bevy's gizmos: what agents perceive, how
queries weighed their items, smart objects' slots, and where agents are going.

## Overview

- [`ArborGizmosPlugin`] draws every part that's on. It needs a renderer:
  Bevy's `GizmoPlugin`, in `DefaultPlugins`.
- Perception: sight cones out to `range`, fainter to `lose_range`; hearing
  ranges; a line to everything in view, colored by attitude; a cross where
  what's remembered was last sensed.
- Queries, with `eqs`: each agent's last query from its `SurveyTrace`, kept
  items from red to green by score, dropped ones gray, the pick ringed.
- Smart objects, with `smart`: every slot, free, claimed with a line to its
  agent, or disabled.
- Navigation, with `nav`: a ring round each agent's destination, and the
  route it's taking.
- Each part is a gizmo group of its own, [`PerceptionGizmos`], [`EqsGizmos`],
  [`SmartGizmos`] and [`NavGizmos`]: a color for each kind of shape, or `None` to leave it
  out. To draw only some agents, turn `draw_all` off and give them
  [`ShowPerception`].

## Installation

Most games depend on `bevy_arbor` with its `gizmos` feature, where this crate
is `bevy_arbor::gizmos` and draws whatever other parts are on:

```toml
[dependencies]
bevy_arbor = { version = "0.1", features = ["perception", "gizmos"] }
```

Or depend on it directly:

```toml
[dependencies]
bevy_arbor_gizmos = { version = "0.1", features = ["eqs", "smart", "nav"] }
```

| Feature | Default | Enables |
|---|---|---|
| `eqs` | No | Drawing how queries weighed their items |
| `smart` | No | Drawing smart objects' slots |
| `nav` | No | Drawing where agents are going |

## Usage

```rust,no_run
use bevy::prelude::*;
use bevy_arbor_gizmos::prelude::*;
use bevy_arbor_perception::prelude::*;

/// Hearing ranges off.
fn quieter(mut config: ResMut<GizmoConfigStore>) {
    let (_, gizmos) = config.config_mut::<PerceptionGizmos>();
    gizmos.hearing = None;
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, PerceptionPlugin::default(), ArborGizmosPlugin))
        .add_systems(Startup, quieter)
        .run();
}
```

## Learn more

- The book's [Debugging](https://declaredlost.github.io/bevy_arbor/debugging.html)
  page, and [Feature flags](https://declaredlost.github.io/bevy_arbor/features.html)
- The [API documentation](https://docs.rs/bevy_arbor_gizmos)

## Contributing

See [CONTRIBUTING.md](https://github.com/declaredlost/bevy_arbor/blob/main/CONTRIBUTING.md)
for development setup and guidelines.

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/declaredlost/bevy_arbor/blob/main/LICENSE-APACHE)
or [MIT license](https://github.com/declaredlost/bevy_arbor/blob/main/LICENSE-MIT) at your option.

[`ArborGizmosPlugin`]: https://docs.rs/bevy_arbor_gizmos/latest/bevy_arbor_gizmos/struct.ArborGizmosPlugin.html
[`PerceptionGizmos`]: https://docs.rs/bevy_arbor_gizmos/latest/bevy_arbor_gizmos/perception/struct.PerceptionGizmos.html
[`EqsGizmos`]: https://docs.rs/bevy_arbor_gizmos/latest/bevy_arbor_gizmos/eqs/struct.EqsGizmos.html
[`SmartGizmos`]: https://docs.rs/bevy_arbor_gizmos/latest/bevy_arbor_gizmos/smart/struct.SmartGizmos.html
[`NavGizmos`]: https://docs.rs/bevy_arbor_gizmos/latest/bevy_arbor_gizmos/nav/struct.NavGizmos.html
[`ShowPerception`]: https://docs.rs/bevy_arbor_gizmos/latest/bevy_arbor_gizmos/perception/struct.ShowPerception.html
