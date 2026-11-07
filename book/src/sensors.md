# Sensors

A sense can sit on a sensor entity, [`SensorOf`] an agent, instead of on the
agent: eyes on a head, a rear motion sensor, two ears.

```rust
# use std::f32::consts::PI;
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# fn spawn(mut commands: Commands) {
let agent = commands.spawn(Transform::default()).id();
commands.spawn((
    Sight::new(12.0),
    SensorOf(agent),
    ChildOf(agent),
    Transform::from_xyz(0.0, 1.6, 0.0),
));
commands.spawn((
    Sight::new(4.0).with_half_angle(PI / 3.0),
    SensorOf(agent),
    ChildOf(agent),
    Transform::from_rotation(Quat::from_rotation_y(PI)),
));
# }
```

A sensor senses from its own transform, so eyes on a head bone turn with it.
What it senses goes to its agent's `Perceived<S>`; the sensor keeps nothing
itself. Despawning the agent despawns its sensors.

An agent can have several sensors of one sense. For a sustained sense, what
any of them senses is active, and lost only once none does: the rear sensor
looking away doesn't lose what the front eyes still see.

`SensorOf` is only about perception; parent the sensor with `ChildOf` too, as
above, for its transform to follow the agent's.

[`SensorOf`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/sense/struct.SensorOf.html
