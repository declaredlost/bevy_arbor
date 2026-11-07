# bevy_arbor_perception

Perception for Bevy AI. An agent senses what's around it, and what it senses is
written to it, one component per sense: [`Perceived<Sight>`],
`Perceived<Hearing>`, and so on, for behaviour tree guards to read, or anything
else. Each changes only when something is noticed, lost or forgotten by that
sense, so a tree reading `Perceived<Sight>` sleeps in between, and a noise
doesn't wake it.

```rust,ignore
app.add_plugins(PerceptionPlugin::default().with_space(Space::Planar));

commands.spawn((Sight::new(12.0).with_lose_range(15.0), Team(1), Transform::default()));
commands.spawn((Perceivable, Team(2), Transform::from_xyz(8.0, 0.0, 0.0)));

fn sees_enemy(seen: &Perceived<Sight>) -> bool {
    seen.active().any(|percept| percept.is_hostile())
}
```

It works in 2D ([`Space::Planar`], the XY plane, ahead is +X) or 3D
([`Space::Volume`], ahead is -Z), one code path for both, and doesn't depend
on a physics engine.

## Senses

A sense is a component, its own configuration. On an agent, the agent senses.

| Sense | Sustained | Sensed from |
|---|---|---|
| [`Sight`] | Yes: in view, then lost | What's [`Perceivable`] in range and in its cone |
| [`Hearing`] | No | [`Noise`] messages within its range times their loudness |
| [`Touch`] | No | [`Touched`] messages: a hit, a bump |
| [`Shared`] | No | [`Share`] messages: allies passing on what they perceive, by every sense |

A sustained sense keeps a thing active while it lasts: in view, for sight. A
sense of moments keeps when and where it last sensed it.

Sight looks every `interval`, each sensor at a moment of its own, so looks
spread across ticks. What it notices within `range` it keeps seeing out to
`lose_range`. Facing is the sensor's transform, or [`Facing`] where that
doesn't turn: a sprite flipped in a side-scroller.

What's shared is what the teller has active, where it is now, and what it
remembers, from when and where it last sensed it. Each listener gives it its
own attitude, and keeps what it knew if that's newer.

## Sensors

A sense can also sit on a sensor entity, [`SensorOf`] an agent: eyes on a
head, a rear motion sensor, two ears. A sensor senses from its own transform,
so eyes on a head bone turn with it, and what it senses goes to its agent's
`Perceived<S>`. An agent can have several sensors of one sense: what any of
them senses is active, and lost only once none does.

## Senses of your own

A sense of your own is a component implementing [`Sense`], registered with
[`SensePlugin`], and a system in `PerceptionSystems::Gather` filling its
sensors' [`Candidates<S>`]: `candidates.sense(entity, position)`, and, for a
sustained sense, `candidates.look_from(eye)` each time it looks. The built-in
senses are registered the same way.

```rust,ignore
#[derive(Component, Reflect)]
struct Smell {
    range: f32,
}

impl Sense for Smell {
    const MEMORY: Duration = Duration::from_secs(20);
}

app.add_plugins(SensePlugin::<Smell>::default())
    .add_systems(FixedUpdate, sniff.in_set(PerceptionSystems::Gather));
```

## Stages

Perception runs in four stages ([`PerceptionSystems`]), and a system of your
own joins one to block what's out of sight with a raycast, or to give
attitudes from your own rules.

| Stage | Built in | Yours |
|---|---|---|
| `Gather` | Sight through a spatial grid; noises, touches and what's shared | Candidates for senses of your own, or from elsewhere |
| `Occlude` | Nothing | Block candidates something's in the way of |
| `Classify` | Nothing | Set candidates' attitude |
| `Commit` | Writes each `Perceived<S>`, with the [`Team`] fallback for attitude; forgets what's faded | |

## Memory and events

Each `Perceived<S>` remembers for its sense's [`Sense::MEMORY`] (ten seconds
for sight and what's shared, five for hearing and touch), or as long as it was
made with: `Perceived::<Hearing>::remembering(..)`. What's no longer active
fades that long after it was last sensed, and is forgotten. An agent with
nothing due to fade costs nothing that tick.

Triggered on the agent, for observers, by sense: [`Noticed<S>`] as something
new comes to its attention or is active again, [`Lost<S>`] as something stops
being active, and [`Forgotten<S>`] as it forgets something.

## Gizmos

With the `gizmos` feature, `PerceptionGizmosPlugin` draws what agents
perceive: each sensor's sight cone out to `range` and, fainter, `lose_range`;
hearing ranges; a line to everything active, coloured by attitude; and a cross
where what's remembered was last sensed. As Bevy's own gizmos do,
`PerceptionGizmos` gives each kind a colour, or `None` to leave it out, and
draws every agent or, without `draw_all`, only those with `ShowPerception`.
It's a gizmo group, so Bevy's `GizmoConfigStore` turns it off altogether.

[`Perceived<Sight>`]: crate::perceived::Perceived
[`Space::Planar`]: crate::space::Space::Planar
[`Space::Volume`]: crate::space::Space::Volume
[`Sight`]: crate::sense::Sight
[`Hearing`]: crate::sense::Hearing
[`Touch`]: crate::sense::Touch
[`Shared`]: crate::sense::Shared
[`Perceivable`]: crate::sense::Perceivable
[`Noise`]: crate::sense::Noise
[`Touched`]: crate::sense::Touched
[`Share`]: crate::sense::Share
[`Facing`]: crate::sense::Facing
[`SensorOf`]: crate::sense::SensorOf
[`Sense`]: crate::sense::Sense
[`Sense::MEMORY`]: crate::sense::Sense::MEMORY
[`SensePlugin`]: crate::sense::SensePlugin
[`Candidates<S>`]: crate::perceived::Candidates
[`PerceptionSystems`]: crate::PerceptionSystems
[`Team`]: crate::perceived::Team
[`Noticed<S>`]: crate::perceived::Noticed
[`Lost<S>`]: crate::perceived::Lost
[`Forgotten<S>`]: crate::perceived::Forgotten
