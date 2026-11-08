# Installation and features

Add `bevy_arbor` to your `Cargo.toml`. The guard needs trees and perception:

```toml
[dependencies]
bevy_arbor = { version = "0.1", features = ["perception"] }
```

It needs Bevy 0.20 and Rust 1.99 or later, edition 2024.

Behavior trees (`bt`) are on by default. Everything else is a feature.

| Feature | Adds |
|---|---|
| `bt` | Behavior trees (default) |
| `perception` | Senses and memory |
| `eqs` | Environment queries; turns on `bt` and `perception` |
| `smart` | Smart objects; turns on `eqs` |
| `nav` | Navigation: `MoveTo` and path costs, over your pathfinding; turns on `bt` |
| `gizmos` | Debug drawing for perception, queries, smart objects and navigation |
| `avian2d`, `avian3d` | Line of sight and touch with avian |

[Feature flags](features.md) has the full list, including each crate's own
features.

## Adding the plugins

Add [`ArborPlugins`]. It adds the plugin of every part you turned on:

```rust,no_run
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
fn main() {
    App::new()
        .add_plugins((DefaultPlugins, ArborPlugins))
        .run();
}
```

Trees and perception run in `FixedUpdate`. Perception works in 3D by
default. The guard lives in a side-scroller, so switch perception to 2D. Use
`set`, as with Bevy's own plugin groups:

```rust,no_run
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            ArborPlugins.set(PerceptionPlugin::default().with_space(Space::Planar)),
        ))
        .run();
}
```

To run trees in another schedule, set [`TreePlugin::new`] the same way:
`ArborPlugins.set(TreePlugin::new(Update))`.

## Using one part alone

The parts don't depend on each other. Perception writes components on the
agent, and trees read them. To use one part alone, depend on its crate, such
as `bevy_arbor_bt` or `bevy_arbor_perception`, and add its plugin.

If you add the plugins one by one, also add [`ArborSystemsPlugin`]. It runs
the parts in the right order. [How it fits together](concepts.md) explains
that order.

## Pitfalls

- **Time.** Trees need Bevy's `TimePlugin` for waits, timeouts and
  cooldowns. `DefaultPlugins` and `MinimalPlugins` both include it.
  `TreePlugin` warns if it's missing.
- **Gizmos.** `ArborGizmosPlugin` isn't in `ArborPlugins`, because it needs a
  renderer. Add it yourself. [Debugging](debugging.md) shows how.
- **Both avian features.** With `avian2d` and `avian3d` both on, neither is in
  the prelude, since their names clash. Use `bevy_arbor::avian2d::...` or
  `bevy_arbor::avian3d::...` instead.

[`ArborPlugins`]: https://docs.rs/bevy_arbor/latest/bevy_arbor/struct.ArborPlugins.html
[`ArborSystemsPlugin`]: https://docs.rs/bevy_arbor/latest/bevy_arbor/struct.ArborSystemsPlugin.html
[`TreePlugin::new`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/struct.TreePlugin.html#method.new
