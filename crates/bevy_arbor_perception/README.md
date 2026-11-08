# bevy_arbor_perception

[![Build Status][action-badge]][action-url]
[![Crate Docs][docs-badge]][docs-url]
[![Crate Version][crates-badge]][crates-url]

**Part of [bevy_arbor](https://github.com/declaredlost/bevy_arbor), AI for Bevy.**

[action-badge]: https://img.shields.io/github/actions/workflow/status/declaredlost/bevy_arbor/build.yml?branch=main&label=build&logo=github&style=flat-square
[action-url]: https://github.com/declaredlost/bevy_arbor/actions/workflows/build.yml
[crates-badge]: https://img.shields.io/crates/v/bevy_arbor_perception.svg?logo=rust&style=flat-square
[crates-url]: https://crates.io/crates/bevy_arbor_perception
[docs-badge]: https://img.shields.io/docsrs/bevy_arbor_perception?logo=Docs.rs&style=flat-square
[docs-url]: https://docs.rs/bevy_arbor_perception

Perception for Bevy AI, in 2D or 3D: what an agent senses, written to it as a
component per sense.

## Overview

- A sense is a component, its own configuration: [`Sight`] (range, cone, a
  lose range, a look interval), [`Hearing`] of [`Noise`] messages, [`Touch`]
  from [`Touched`] messages, and [`Shared`], what allies pass on with
  [`Share`]. Senses of your own join through [`SensePlugin`].
- What an agent senses is written to it as [`Perceived<S>`], one per sense,
  with what's active and what's remembered for a while after.
- `Perceived<S>` changes only when something is noticed, lost or forgotten, so
  a behavior tree reading it sleeps in between. [`Noticed`], [`Lost`] and
  [`Forgotten`] are triggered on the agent as it does.
- Sensors are entities [`SensorOf`] an agent, each sensing from its own
  transform: a camera, a turret's eye.
- [`Space`] says whether it's 2D (the XY plane) or 3D. It doesn't depend on a
  physics engine; systems of your own block what's out of sight and set how
  an agent stands toward what it senses.

## Installation

Most games depend on `bevy_arbor` with its `perception` feature, where this
crate is `bevy_arbor::perception`:

```toml
[dependencies]
bevy_arbor = { version = "0.1", features = ["perception"] }
```

Or depend on this crate directly to use perception alone:

```toml
[dependencies]
bevy_arbor_perception = "0.1"
```

## Usage

```rust,no_run
use bevy::prelude::*;
use bevy_arbor_perception::prelude::*;

/// What a behavior tree, or any system, reads.
fn sees_enemy(seen: &Perceived<Sight>) -> bool {
    seen.active().any(Percept::is_hostile)
}

fn spawn(mut commands: Commands) {
    // A guard that sees 12 units ahead, and keeps seeing out to 15.
    commands.spawn((
        Sight::new(12.0).with_lose_range(15.0),
        Team(1),
        Transform::default(),
    ));
    // Something it can see, on another team.
    commands.spawn((Perceivable, Team(2), Transform::from_xyz(8.0, 0.0, 0.0)));
}

fn main() {
    App::new()
        .add_plugins((
            MinimalPlugins,
            PerceptionPlugin::default().with_space(Space::Planar),
        ))
        .add_systems(Startup, spawn)
        .run();
}
```

## Learn more

- The book's [Senses](https://declaredlost.github.io/bevy_arbor/senses.html),
  [Sensors, memory and events](https://declaredlost.github.io/bevy_arbor/sensors-memory.html),
  [Occlusion and attitude](https://declaredlost.github.io/bevy_arbor/occlusion-attitude.html)
  and [Senses of your own](https://declaredlost.github.io/bevy_arbor/custom-senses.html)
  pages, and [Feature flags](https://declaredlost.github.io/bevy_arbor/features.html)
- The [API documentation](https://docs.rs/bevy_arbor_perception)

## Contributing

See [CONTRIBUTING.md](https://github.com/declaredlost/bevy_arbor/blob/main/CONTRIBUTING.md)
for development setup and guidelines.

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/declaredlost/bevy_arbor/blob/main/LICENSE-APACHE)
or [MIT license](https://github.com/declaredlost/bevy_arbor/blob/main/LICENSE-MIT) at your option.

[`Sight`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/sense/struct.Sight.html
[`Hearing`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/sense/struct.Hearing.html
[`Noise`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/sense/struct.Noise.html
[`Touch`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/sense/struct.Touch.html
[`Touched`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/sense/struct.Touched.html
[`Shared`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/sense/struct.Shared.html
[`Share`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/sense/struct.Share.html
[`SensePlugin`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/sense/struct.SensePlugin.html
[`Perceived<S>`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/perceived/struct.Perceived.html
[`Noticed`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/perceived/struct.Noticed.html
[`Lost`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/perceived/struct.Lost.html
[`Forgotten`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/perceived/struct.Forgotten.html
[`SensorOf`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/sense/struct.SensorOf.html
[`Space`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/enum.Space.html
