# Occlusion and attitude

Perception is physics-free: what blocks sight, and who's hostile, are systems
of yours, joining its stages. They work on each sensor's [`Candidates<S>`]:
what it might be sensing this tick.

## Occlusion

A system in `PerceptionSystems::Occlude` blocks what something's in the way
of. With a physics engine, that's a raycast from where the sensor looked:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component)] struct Wall;
# fn ray_hits_wall(_: Vec3, _: Vec3, _: &Query<&GlobalTransform, With<Wall>>) -> bool { false }
fn walls(mut sensors: Query<&mut Candidates<Sight>>, walls: Query<&GlobalTransform, With<Wall>>) {
    for mut candidates in &mut sensors {
        let Some(eye) = candidates.eye() else {
            continue;
        };
        for candidate in candidates.iter_mut() {
            if ray_hits_wall(eye, candidate.position(), &walls) {
                candidate.block();
            }
        }
    }
}

# fn add(app: &mut App) {
app.add_systems(FixedUpdate, walls.in_set(PerceptionSystems::Occlude));
# }
```

Several occluders combine: one blocking is enough. With avian, this one is
ready-made: [Physics with avian](avian.md).

## Attitude

Each percept has an attitude: hostile, neutral or friendly. A system in
`PerceptionSystems::Classify` gives it, and what it says stands:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
#[derive(Component)]
struct Cursed;

fn curses(mut sensors: Query<&mut Candidates<Sight>>, cursed: Query<(), With<Cursed>>) {
    for mut candidates in &mut sensors {
        for candidate in candidates.iter_mut() {
            if cursed.contains(candidate.entity()) {
                candidate.set_attitude(Attitude::Hostile);
            }
        }
    }
}
```

What nothing classified gets the [`Team`] fallback: the same team is friendly,
another is hostile, and either side without one is neutral.

[`Candidates<S>`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/perceived/struct.Candidates.html
[`Team`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/perceived/struct.Team.html
