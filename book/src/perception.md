# Perception

Perception tells an agent what's around it. What it senses is written to it,
one component per sense: [`Perceived<Sight>`], `Perceived<Hearing>`, and so
on. Each changes only when something is noticed, lost or forgotten by that
sense, so a tree reading `Perceived<Sight>` sleeps in between, and a noise
doesn't wake it.

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
fn sees_enemy(seen: &Perceived<Sight>) -> bool {
    seen.active().any(Percept::is_hostile)
}

fn heard_something(heard: &Perceived<Hearing>) -> bool {
    !heard.is_empty()
}
```

## 2D and 3D

Perception works in 3D by default: ahead is -Z, Bevy's forward. For 2D,
[`Space::Planar`] works in the XY plane, ignoring z, which 2D uses for
layering, and ahead is +X:

```rust,no_run
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# fn main() {
App::new().add_plugins((
    MinimalPlugins,
    ArborPlugins.set(PerceptionPlugin::default().with_space(Space::Planar)),
));
# }
```

It's one code path for both: only what's ahead and what distance is differ.

## Senses

| Sense | Sustained | Sensed from |
|---|---|---|
| [Sight](sight.md) | Yes: in view, then lost | What's `Perceivable` in range and in its cone |
| [Hearing](hearing-touch-sharing.md) | No | `Noise` messages within range |
| [Touch](hearing-touch-sharing.md) | No | `Touched` messages |
| [Shared](hearing-touch-sharing.md) | No | `Share` messages: allies passing on what they know |

A sense is a component, its own configuration, on the agent or on a
[sensor](sensors.md). A sustained sense keeps a thing *active* while it lasts;
a sense of moments keeps when and where it last sensed it.

## Stages

Perception runs in four stages, [`PerceptionSystems`], and a system of your
own joins one:

| Stage | Built in | Yours |
|---|---|---|
| `Gather` | Sight through a spatial grid; noises, touches and what's shared | Candidates for [senses of your own](custom-senses.md) |
| `Occlude` | Nothing | Block what something's in the way of: a raycast |
| `Classify` | Nothing | Give candidates their attitude |
| `Commit` | Writes each `Perceived<S>`, with the `Team` fallback; forgets what's faded | |

It doesn't depend on a physics engine: line of sight is a system in
`Occlude`, yours ([Occlusion and attitude](occlusion-attitude.md)) or avian's
([Physics with avian](avian.md)).

[`Perceived<Sight>`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/perceived/struct.Perceived.html
[`Space::Planar`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/space/enum.Space.html
[`PerceptionSystems`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/enum.PerceptionSystems.html
