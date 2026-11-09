# bevy_arbor_avian

[![Build Status][action-badge]][action-url]
[![Crate Docs][docs-badge]][docs-url]
[![Crate Version][crates-badge]][crates-url]

**Part of [bevy_arbor](https://github.com/declaredlost/bevy_arbor), AI for Bevy.**

[action-badge]: https://img.shields.io/github/actions/workflow/status/declaredlost/bevy_arbor/build.yml?branch=main&label=build&logo=github&style=flat-square
[action-url]: https://github.com/declaredlost/bevy_arbor/actions/workflows/build.yml
[crates-badge]: https://img.shields.io/crates/v/bevy_arbor_avian2d.svg?logo=rust&style=flat-square
[crates-url]: https://crates.io/crates/bevy_arbor_avian2d
[docs-badge]: https://img.shields.io/docsrs/bevy_arbor_avian2d?logo=Docs.rs&style=flat-square
[docs-url]: https://docs.rs/bevy_arbor_avian2d

Perception with [avian](https://github.com/avianphysics/avian): line of sight
through colliders, and touch from contacts. Built as `bevy_arbor_avian2d` and
`bevy_arbor_avian3d` from one source, as avian's own crates are.

## Overview

- `ColliderSightPlugin` lets sight find what's near through avian's
  colliders, instead of perception's own grid: nothing to rebuild. What's
  seen then needs a collider; `PerceivableLayers` keeps looks off walls.
- `LineOfSightPlugin` blocks what's out of sight: a ray from where each sensor
  looked to each thing it would notice. What's on `Occluders`' layers blocks
  it, or a sensor's own `SensorOccluders`. Something with `SightPoints` is
  seen if a ray to any of them gets through: a head over a low wall.
- The agent's own colliders, the sensor's, and trigger volumes (avian's
  `Sensor`) never block sight.
- `ContactTouchPlugin` turns avian's collisions into perception's `Touched`,
  for agents with `Touch`, from colliders on the layers you choose.
- With the `eqs` feature, three environment query tests: `LineOfSight`,
  whether an anchor can see each item; `Clearance`, whether there's room to
  stand at it; and `OccluderDistance`, how far it is from what blocks sight.
  Together they find cover: room to stand, out of sight, up against a wall.
- `AvianPerceptionPlugins` adds all three plugins.
- Perception has to work in the space of the crate's dimension:
  `Space::Planar` with avian2d, `Space::Volume` with avian3d. Otherwise the
  plugins warn at startup.

## Installation

Most games depend on `bevy_arbor` with its `avian2d` or `avian3d` feature,
where `ArborPlugins` already adds `AvianPerceptionPlugins`:

```toml
[dependencies]
bevy_arbor = { version = "0.1", features = ["perception", "avian2d"] }
```

Or depend on a crate directly, with `bevy_arbor_perception`:

```toml
[dependencies]
bevy_arbor_avian2d = "0.1"
```

| Feature | Default | Enables |
|---|---|---|
| `parry-f32` | Yes | avian's single-precision collider backend. For double precision, turn it off and enable `parry-f64` on your own avian dependency. |
| `eqs` | No | `LineOfSight`, `Clearance` and `OccluderDistance`, environment query tests |

## Usage

```rust,ignore
use avian2d::prelude::*;
use bevy::prelude::*;
use bevy_arbor_avian2d::prelude::*;
use bevy_arbor_perception::prelude::*;

App::new()
    .add_plugins((
        DefaultPlugins,
        PhysicsPlugins::default(),
        PerceptionPlugin::default().with_space(Space::Planar),
        AvianPerceptionPlugins,
    ))
    // Only walls and doors block sight.
    .insert_resource(Occluders(LayerMask::from([Layer::Walls, Layer::Doors])))
    .run();
```

## Learn more

- The book's [Occlusion and attitude](https://declaredlost.github.io/bevy_arbor/occlusion-attitude.html)
  page, and [Feature flags](https://declaredlost.github.io/bevy_arbor/features.html)
- The [API documentation](https://docs.rs/bevy_arbor_avian2d)

## Contributing

See [CONTRIBUTING.md](https://github.com/declaredlost/bevy_arbor/blob/main/CONTRIBUTING.md)
for development setup and guidelines.

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/declaredlost/bevy_arbor/blob/main/LICENSE-APACHE)
or [MIT license](https://github.com/declaredlost/bevy_arbor/blob/main/LICENSE-MIT) at your option.
