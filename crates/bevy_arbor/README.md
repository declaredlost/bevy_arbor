# bevy_arbor

AI for Bevy, one crate per part. This crate brings them together: each part
behind a feature, as a module of its own, in one prelude, and with one plugin
group.

```rust
use bevy::prelude::*;
use bevy_arbor::prelude::*;

App::new().add_plugins(ArborPlugins.set(TreePlugin::new(Update)));
```

`ArborPlugins` also puts every part's systems in `ArborSystems`, in order:
perception, then trees, where they share a schedule. Put what keeps facts on
agents before it, and task systems after it. Adding the parts' plugins one by
one, add `ArborSystemsPlugin` too.

Applications depend on `bevy_arbor`. Depend on a part's crate directly to use
it alone: the parts don't depend on each other, as each reads and writes
components on the agent.

| Feature | Part | Module | Plugin |
|---|---|---|---|
| `bt` (default) | [`bevy_arbor_bt`](https://github.com/declaredlost/bevy_arbor/tree/main/crates/bevy_arbor_bt): reactive behaviour trees | `bevy_arbor::bt`, with `bt::behaviour` and `bt::task` | `TreePlugin` |
| `perception` | [`bevy_arbor_perception`](https://github.com/declaredlost/bevy_arbor/tree/main/crates/bevy_arbor_perception): sight, hearing, touch, sharing and senses of your own, written as `Perceived<S>` on the agent | `bevy_arbor::perception`, with `perception::sense`, `perception::perceived` and `perception::space` | `PerceptionPlugin` |
| `gizmos` | Drawing what agents perceive: `bevy_arbor::perception::gizmos`, not in `ArborPlugins` | | `PerceptionGizmosPlugin` |
