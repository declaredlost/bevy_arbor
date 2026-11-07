# Installation

Add `bevy_arbor` to your `Cargo.toml`, with the parts you use:

```toml
[dependencies]
bevy_arbor = { version = "0.1", features = ["perception"] }
```

Behaviour trees (`bt`) are on by default; perception and its gizmos are
features. [Feature flags](features.md) lists them all.

Then add `ArborPlugins`, which adds the plugin of every part you enabled:

```rust,no_run
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
fn main() {
    App::new()
        .add_plugins((MinimalPlugins, ArborPlugins))
        .run();
}
```

Each part keeps its defaults: trees and perception run in `FixedUpdate`, and
perception works in 3D. Change one with `set`, as with Bevy's own plugin
groups:

```rust,no_run
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# fn main() {
App::new().add_plugins((
    MinimalPlugins,
    ArborPlugins.set(PerceptionPlugin::default().with_space(Space::Planar)),
));
# }
```

## Using a part alone

The parts don't depend on each other: perception writes components on the
agent and trees read them. To use one alone, depend on its crate,
`bevy_arbor_bt` or `bevy_arbor_perception`, and add its plugin, `TreePlugin`
or `PerceptionPlugin`. Adding both that way, add `ArborSystemsPlugin` too, so
perception runs before trees ([How it fits together](concepts.md)).
