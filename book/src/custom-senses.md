# Senses of your own

The guard gets a dog. The dog can't see far in the dark, but it smells the
intruder through walls. Smell isn't built in, so you write it.

A sense of your own has three parts:

1. A component implementing [`Sense`]: its configuration.
2. [`SensePlugin`] registering it.
3. A system in `PerceptionSystems::Gather` that fills each sensor's
   `Candidates<S>`.

The built-in senses are made the same way.

## A minimal sense

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

/// On what leaves a scent: the intruder.
#[derive(Component)]
struct Scented;

fn sniff(
    space: Res<Space>,
    scents: Query<(Entity, &GlobalTransform), With<Scented>>,
    mut noses: Query<(&Smell, &GlobalTransform, &mut Candidates<Smell>)>,
) {
    for (smell, nose, mut candidates) in &mut noses {
        let nose = space.flatten(nose.translation());
        for (scent, at) in &scents {
            let at = space.flatten(at.translation());
            if at.distance(nose) <= smell.range {
                candidates.sense(scent, at);
            }
        }
    }
}

# fn add(app: &mut App) {
app.add_plugins(SensePlugin::<Smell>::default())
    .add_systems(FixedUpdate, sniff.in_set(PerceptionSystems::Gather));
# }
```

Spawn the dog with `Smell { range: 6.0 }` and a tree of its own. Give the
intruder `Scented`. The dog's conditions read `Perceived<Smell>`, just as the
guard's read `Perceived<Sight>`:

```rust
# use std::time::Duration;
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component, Reflect)] struct Smell { range: f32 }
# impl Sense for Smell { const MEMORY: Duration = Duration::from_secs(20); }
fn smells_intruder(smelled: &Perceived<Smell>) -> bool {
    smelled.iter().any(Percept::is_hostile)
}
```

## What the plugin gives

`SensePlugin::<Smell>` gives the sense everything the built-in ones have:

- `Perceived<Smell>` on each agent with `Smell`, or with a sensor that has it.
- `Candidates<Smell>` on each sensor, emptied after every tick.
- Memory, for `Smell::MEMORY` or the agent's own `remembering`.
- Sharing: a `Share` passes on what the dog smelled.
- The events `Noticed<Smell>`, `Lost<Smell>` and `Forgotten<Smell>`.
- Your occlusion and attitude systems work on it, as on any sense
  ([Occlusion and attitude](occlusion-attitude.md)).

Add it after `PerceptionPlugin`, that is after `ArborPlugins`.

## In 2D

`candidates.sense(entity, position)` takes a position in perception's
[`Space`]. Pass positions through `Space::flatten`, as `sniff` does. In 2D it
drops z, which 2D uses for layering, so z doesn't count toward distance.

## Sustained senses

Smell, as written, is a sense of moments, like hearing. Each sniff refreshes
the memory, and the scent fades when the sniffing stops. It's never *active*,
and never lost.

Each sniff also changes `Perceived<Smell>`, since the scent was sensed anew.
`sniff` runs every tick, so the dog's tree wakes every tick while it smells
the intruder. Sniff less often, or make the sense sustained.

A sustained sense keeps a thing active while it lasts, as sight does. Say so
with `SUSTAINED`, and call [`look_from`] each time a sensor looks:

```rust
# use std::time::Duration;
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component)] struct Scented;
#[derive(Component, Reflect)]
struct Smell {
    range: f32,
}

impl Sense for Smell {
    const SUSTAINED: bool = true;
    const MEMORY: Duration = Duration::from_secs(20);
}

fn sniff(
    space: Res<Space>,
    scents: Query<(Entity, &GlobalTransform), With<Scented>>,
    mut noses: Query<(&Smell, &GlobalTransform, &mut Candidates<Smell>)>,
) {
    for (smell, nose, mut candidates) in &mut noses {
        let nose = space.flatten(nose.translation());
        candidates.look_from(nose);
        for (scent, at) in &scents {
            let at = space.flatten(at.translation());
            if at.distance(nose) <= smell.range {
                candidates.sense(scent, at);
            }
        }
    }
}
```

What a sensor sensed at its last look is active. What none of an agent's
sensors sensed at their last look is lost, and triggers `Lost<Smell>`.

A sensor that doesn't call `look_from` this tick keeps what it had. So a
sense can look less often than every tick, as sight does with its interval.

`look_from` also says where the sensor looked from. That's the `eye()`
occluders cast rays from. Avian's `LineOfSightPlugin` only blocks sight by
default. For it to block another sense, add `LineOfSightPlugin::<Smell>` too.

## Pitfalls

- **Forgetting `Gather`.** Outside `PerceptionSystems::Gather`, a gather
  system isn't ordered with occlusion and attitude. What it senses may skip
  them, or be committed a tick late.
- **A moment sensed every tick.** For a sense that isn't sustained, every
  sensing changes `Perceived<S>` and wakes the trees reading it.
- **A sustained sense without `look_from`.** Nothing is ever active or lost.

[`Sense`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/sense/trait.Sense.html
[`SensePlugin`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/sense/struct.SensePlugin.html
[`Space`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/enum.Space.html
[`look_from`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/perceived/struct.Candidates.html#method.look_from
