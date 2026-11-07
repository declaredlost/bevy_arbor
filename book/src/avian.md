# Physics with avian

Perception is physics-free; [`bevy_arbor_avian2d`] and `bevy_arbor_avian3d`
add what [avian](https://github.com/avianphysics/avian) can answer: line of
sight, and touch from contacts. Enable the `avian2d` or `avian3d` feature of
`bevy_arbor`, and `ArborPlugins` adds them.

```toml
[dependencies]
bevy_arbor = { version = "0.1", features = ["avian2d"] }
avian2d = "0.8"
```

`bevy_arbor` doesn't choose avian's features: you depend on avian as you
would anyway.

## Line of sight

`LineOfSightPlugin` casts a ray from where each sensor looked to each thing it
might be seeing, in `PerceptionSystems::Occlude`. What the ray hits first on
the way, if it isn't the thing itself, blocks it. The agent's and the
sensor's own colliders never do.

What blocks sight is what's on [`Occluders`]' layers, avian's collision
layers: every layer, unless you say otherwise.

```rust,no_run
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
use avian2d::prelude::*;
use bevy_arbor::avian2d::Occluders;

#[derive(PhysicsLayer, Default)]
enum Layer {
    #[default]
    Default,
    Terrain,
    Glass,
    Monsters,
}

fn main() {
    App::new()
        .add_plugins((
            MinimalPlugins,
            PhysicsPlugins::default(),
            ArborPlugins.set(PerceptionPlugin::default().with_space(Space::Planar)),
        ))
        // Terrain and glass block sight; other monsters don't.
        .insert_resource(Occluders(LayerMask::from([Layer::Terrain, Layer::Glass])))
        .run();
}
```

A sensor's own [`SensorOccluders`] says otherwise for it, as avian's
`GravityScale` does over `Gravity`. Eyes that see through glass leave its layer
out:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# use avian2d::prelude::*;
# #[derive(PhysicsLayer, Default)] enum Layer { #[default] Default, Terrain, Glass }
use bevy_arbor::avian2d::SensorOccluders;

# fn spawn(mut commands: Commands) {
commands.spawn((
    Sight::new(12.0),
    SensorOccluders(LayerMask::from([Layer::Terrain])),
    Transform::default(),
));
# }
```

## Sight points

Without more, a ray goes to a thing's middle, so a player crouched behind a
low wall is hidden, and so is one standing behind it. [`SightPoints`] lists
the points on a thing a sensor can see it by, in its own space: it's seen if
a ray to any gets through.

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
use bevy_arbor::avian2d::SightPoints;

# fn spawn(mut commands: Commands) {
commands.spawn((
    Perceivable,
    SightPoints(vec![
        Vec3::new(0.0, 1.7, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::ZERO,
    ]),
    Transform::default(),
));
# }
```

Head, chest and feet: a head over the wall is enough.

## Touch from contacts

`ContactTouchPlugin` turns avian's collision events into `Touched`, for
agents with `Touch`: a bump or a hit is felt with no code of yours. An agent
with `Touch` gets avian's `CollisionEventsEnabled`, so its own collider
reports; a collider on a child of it needs one too.

Only colliders on its layers touch, every layer unless set: a monster feels a
sword's hitbox, not the grass.

```rust,no_run
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# use avian2d::prelude::*;
# #[derive(PhysicsLayer, Default)] enum Layer { #[default] Default, Hitbox }
use bevy_arbor::avian2d::ContactTouchPlugin;

# fn main() {
App::new().add_plugins((
    MinimalPlugins,
    PhysicsPlugins::default(),
    ArborPlugins.set(ContactTouchPlugin { layers: LayerMask::from([Layer::Hitbox]) }),
));
# }
```

## Timing

Perception runs in `FixedUpdate` and avian in `FixedPostUpdate`, so a ray sees
colliders where the last physics step left them: up to a step behind, which
an AI won't notice.

[`bevy_arbor_avian2d`]: https://docs.rs/bevy_arbor_avian2d
[`Occluders`]: https://docs.rs/bevy_arbor_avian2d/latest/bevy_arbor_avian2d/struct.Occluders.html
[`SensorOccluders`]: https://docs.rs/bevy_arbor_avian2d/latest/bevy_arbor_avian2d/struct.SensorOccluders.html
[`SightPoints`]: https://docs.rs/bevy_arbor_avian2d/latest/bevy_arbor_avian2d/struct.SightPoints.html
