# bevy_arbor_avian

[bevy_arbor](https://github.com/declaredlost/bevy_arbor) perception with
[avian](https://github.com/avianphysics/avian): what physics can answer that
perception, physics-free, leaves to you. Built as `bevy_arbor_avian2d` and
`bevy_arbor_avian3d`, one source, as avian's own crates are.

| Plugin | What it does |
|---|---|
| `LineOfSightPlugin` | Blocks what's out of sight: a ray from where each sensor looked to each candidate, in `PerceptionSystems::Occlude` |
| `ContactTouchPlugin` | Avian's `CollisionStart` as `Touched`, for agents with `Touch` |
| `AvianPerceptionPlugins` | Both |

Through `bevy_arbor`, with its `avian2d` or `avian3d` feature, `ArborPlugins`
already has `AvianPerceptionPlugins`: add nothing more.

```rust,ignore
app.add_plugins((
    PhysicsPlugins::default(),
    ArborPlugins.set(PerceptionPlugin::default().with_space(Space::Planar)),
))
.insert_resource(Occluders(LayerMask::from([Layer::Terrain, Layer::Doors])));
```

On its own, with `bevy_arbor_perception`, add it after `PerceptionPlugin`:

```rust,ignore
app.add_plugins((
    PhysicsPlugins::default(),
    PerceptionPlugin::default().with_space(Space::Planar),
    AvianPerceptionPlugins,
));
```

What blocks sight is what's on `Occluders`' layers, avian's `CollisionLayers`,
or a sensor's own `SensorOccluders`; the agent's and the sensor's own colliders
never do, nor trigger volumes (avian's `Sensor`). Something with `SightPoints` is seen if a ray to any of them gets
through: a head over a low wall.

It doesn't choose avian's features: enable `avian2d` or `avian3d` with the
backend you use. `parry-f32` is on by default; for double precision, turn off
default features and enable `parry-f64` on your own avian dependency.
