# Feature flags

## bevy_arbor

| Feature | Default | What it enables |
|---|---|---|
| `bt` | Yes | Behaviour trees: `bevy_arbor_bt`, as `bevy_arbor::bt`, with `TreePlugin` in `ArborPlugins` |
| `perception` | No | Perception: `bevy_arbor_perception`, as `bevy_arbor::perception`, with `PerceptionPlugin` in `ArborPlugins` |
| `gizmos` | No | Drawing what agents perceive, as `bevy_arbor::perception::gizmos`; turns on `perception`. Not in `ArborPlugins`: it needs a renderer |

## bevy_arbor_perception

| Feature | Default | What it enables |
|---|---|---|
| `gizmos` | No | `PerceptionGizmosPlugin` and `PerceptionGizmos`, through Bevy's `bevy_gizmos` |

`bevy_arbor_bt` and `bevy_arbor_macros` have no features.
