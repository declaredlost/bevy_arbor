# Sight

[`Sight`] sees what's [`Perceivable`] in range and in its cone.

```rust
# use std::f32::consts::FRAC_PI_4;
# use std::time::Duration;
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# fn spawn(mut commands: Commands) {
commands.spawn((
    Sight::new(12.0)
        .with_lose_range(15.0)
        .with_half_angle(FRAC_PI_4)
        .with_eye(Vec3::new(0.0, 1.6, 0.0))
        .with_interval(Duration::from_millis(100)),
    Transform::default(),
));
commands.spawn((Perceivable, Transform::from_xyz(0.0, 0.0, -10.0)));
# }
```

| Field | What it does | Default |
|---|---|---|
| `range` | How far it notices things | Given |
| `lose_range` | How far it keeps seeing what it already sees | `range` |
| `half_angle` | How far off ahead it sees, either side, in radians; π is all round | π/2: the front half |
| `eye` | Where it looks from, on the agent | The origin |
| `interval` | How long between looks; zero looks every tick | 0.2 s |

`lose_range` keeps a thing at the edge from flickering in and out of view.
Each agent looks at a moment of its own within its interval, so looks spread
across ticks.

## Facing

Sight looks the way its transform does: +X in 2D, so a sprite flipped by a
negative x scale looks the other way; -Z in 3D. In a side-scroller, where a
flipped sprite doesn't turn its transform, keep a [`Facing`] up to date
instead:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
/// The game's own: which way a character walks, and its sprite is flipped.
#[derive(Component)]
struct Walking {
    left: bool,
}

fn face(mut agents: Query<(&Walking, &mut Facing), Changed<Walking>>) {
    for (walking, mut facing) in &mut agents {
        let way = if walking.left { Dir3::NEG_X } else { Dir3::X };
        facing.set_if_neq(Facing(way));
    }
}
```

## What's seen

What's in view is *active* in `Perceived<Sight>`; its `trace` is when and where
it came into view. Once lost, it isn't active, and its trace is when and where
it was lost, until it's forgotten ([Memory and events](memory-events.md)). For
where something in view is now, read its transform: `Perceived` keeps no live
positions, so it doesn't change as things move.

Sight finds what's near through a spatial grid, rebuilt each tick at about
10 nanoseconds a thing, so its cost grows with the number of agents, not their
square.

[`Sight`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/sense/struct.Sight.html
[`Perceivable`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/sense/struct.Perceivable.html
[`Facing`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/sense/struct.Facing.html
