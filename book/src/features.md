# Feature flags

## bevy_arbor

| Feature | Default | What it enables |
|---|---|---|
| `bt` | Yes | Behaviour trees: `bevy_arbor_bt`, as `bevy_arbor::bt`, with `TreePlugin` in `ArborPlugins` |
| `perception` | No | Perception: `bevy_arbor_perception`, as `bevy_arbor::perception`, with `PerceptionPlugin` in `ArborPlugins` |
| `gizmos` | No | Drawing what agents perceive, as `bevy_arbor::perception::gizmos`; turns on `perception`. Not in `ArborPlugins`: it needs a renderer |
| `avian2d` | No | Perception with avian2d: `bevy_arbor_avian2d`, as `bevy_arbor::avian2d`, with `AvianPerceptionPlugins` in `ArborPlugins`; turns on `perception` |
| `avian3d` | No | The same with avian3d: `bevy_arbor_avian3d`, as `bevy_arbor::avian3d` |

The prelude has the avian crate that's on. With both on, their names would
clash, so neither is in it: use `bevy_arbor::avian2d` and `bevy_arbor::avian3d`.

## bevy_arbor_perception

| Feature | Default | What it enables |
|---|---|---|
| `gizmos` | No | `PerceptionGizmosPlugin` and `PerceptionGizmos`, through Bevy's `bevy_gizmos` |

## bevy_arbor_avian2d and bevy_arbor_avian3d

| Feature | Default | What it enables |
|---|---|---|
| `parry-f32` | Yes | avian's single-precision collider backend |

For double precision, turn off default features and enable `parry-f64` on your
own avian dependency; the two backends can't both be on.

`bevy_arbor_bt` and `bevy_arbor_macros` have no features.
