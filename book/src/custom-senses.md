# Senses of your own

A sense of your own is a component implementing [`Sense`], registered with
[`SensePlugin`], and a system in `PerceptionSystems::Gather` filling its
sensors' `Candidates<S>`. The built-in senses are made the same way.

```rust
# use std::time::Duration;
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
/// On an agent: it smells what's `Scented` within range.
#[derive(Component, Reflect)]
struct Smell {
    range: f32,
}

impl Sense for Smell {
    const MEMORY: Duration = Duration::from_secs(20);
}

#[derive(Component)]
struct Scented;

fn sniff(
    scents: Query<(Entity, &GlobalTransform), With<Scented>>,
    mut noses: Query<(&Smell, &GlobalTransform, &mut Candidates<Smell>)>,
) {
    for (smell, nose, mut candidates) in &mut noses {
        for (scent, at) in &scents {
            if at.translation().distance(nose.translation()) <= smell.range {
                candidates.sense(scent, at.translation());
            }
        }
    }
}

# fn add(app: &mut App) {
app.add_plugins(SensePlugin::<Smell>::default())
    .add_systems(FixedUpdate, sniff.in_set(PerceptionSystems::Gather));
# }
```

Add `SensePlugin` after `PerceptionPlugin`. It keeps `Perceived<Smell>` on the
agent, `Candidates<Smell>` on each sensor, the systems writing and forgetting
them, sharing them, and `Noticed<Smell>`, `Lost<Smell>` and `Forgotten<Smell>`.
Occlusion and attitude systems work on it as on any sense.

## Sustained senses

A sustained sense keeps a thing active while it lasts, as sight does. Say so
with `SUSTAINED`, and say when a sensor looked, with `look_from`: what it
sensed at its last look is active, and what none of an agent's sensors sensed
at theirs is lost.

```rust
# use std::time::Duration;
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component)] struct Blip;
#[derive(Component, Reflect)]
struct Radar {
    range: f32,
}

impl Sense for Radar {
    const SUSTAINED: bool = true;
    const MEMORY: Duration = Duration::from_secs(5);
}

fn sweep(
    blips: Query<(Entity, &GlobalTransform), With<Blip>>,
    mut dishes: Query<(&Radar, &GlobalTransform, &mut Candidates<Radar>)>,
) {
    for (radar, dish, mut candidates) in &mut dishes {
        candidates.look_from(dish.translation());
        for (blip, at) in &blips {
            if at.translation().distance(dish.translation()) <= radar.range {
                candidates.sense(blip, at.translation());
            }
        }
    }
}
```

In 2D, pass positions through `Space::flatten`, as the built-in senses do, so
z, a 2D layer, doesn't count.

[`Sense`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/sense/trait.Sense.html
[`SensePlugin`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/sense/struct.SensePlugin.html
