# Feature flags

## bevy_arbor

| Feature | Default | Enables |
|---|---|---|
| `bt` | Yes | Behavior trees: `bevy_arbor_bt` as `bevy_arbor::bt`. Adds `TreePlugin` to `ArborPlugins`. |
| `perception` | No | Perception: `bevy_arbor_perception` as `bevy_arbor::perception`. Adds `PerceptionPlugin` to `ArborPlugins`. |
| `eqs` | No | Environment queries: `bevy_arbor_eqs` as `bevy_arbor::eqs`. Adds `EqsPlugin` to `ArborPlugins`. Turns on `bt` and `perception`, and the avian crates' tests (`LineOfSight`, `Clearance`, `OccluderDistance`, `Footing`) where they're on. |
| `smart` | No | Smart objects: `bevy_arbor_smart` as `bevy_arbor::smart`. Adds `SmartPlugin` to `ArborPlugins`. Turns on `eqs`. |
| `nav` | No | Navigation: `bevy_arbor_nav` as `bevy_arbor::nav`. Adds `NavPlugin` to `ArborPlugins`. Turns on `bt`. With `eqs` on too, `MoveTo::found` and `PathDistance`. |
| `gizmos` | No | Debug drawing: `bevy_arbor_gizmos` as `bevy_arbor::gizmos`, with what queries, smart objects and navigation draw when those parts are on. Turns on `perception`. `ArborGizmosPlugin` isn't in `ArborPlugins`, since it needs a renderer. |
| `avian2d` | No | Perception with avian2d: `bevy_arbor_avian2d` as `bevy_arbor::avian2d`. Adds `AvianPerceptionPlugins` to `ArborPlugins`. Turns on `perception`. |
| `avian3d` | No | The same with avian3d, as `bevy_arbor::avian3d`. |

The prelude includes the avian crate that's on. With both on, their names
clash, so neither is in the prelude. Use `bevy_arbor::avian2d` and
`bevy_arbor::avian3d` directly.

## bevy_arbor_gizmos

It always draws perception, as `PerceptionGizmos`.

| Feature | Default | Enables |
|---|---|---|
| `eqs` | No | Drawing how queries weighed their items, as `EqsGizmos` |
| `smart` | No | Drawing smart objects' slots, as `SmartGizmos` |
| `nav` | No | Drawing where agents are going, as `NavGizmos` |

## bevy_arbor_nav

| Feature | Default | Enables |
|---|---|---|
| `eqs` | Yes | `MoveTo::found` and `PathDistance`, with environment queries |

## bevy_arbor_avian2d and bevy_arbor_avian3d

| Feature | Default | Enables |
|---|---|---|
| `parry-f32` | Yes | avian's single-precision collider backend |
| `eqs` | No | `LineOfSight`, `Clearance`, `OccluderDistance` and `Footing`, environment query tests |

For double precision, turn off default features and enable `parry-f64` on
your own avian dependency. The two backends can't both be on.

`bevy_arbor_bt` and `bevy_arbor_macros` have no features.
