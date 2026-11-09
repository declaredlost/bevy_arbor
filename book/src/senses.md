# Senses

Senses tell the guard what's around it. It sees the intruder, hears footsteps,
feels a hit, and hears what other guards shout.

Each sense is a component on the guard. What it senses is written back to the
guard as [`Perceived<S>`], one per sense, with when and where in
[`Traces<S>`]. The guard's conditions read them:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component)] struct Guard;
# #[derive(Component)] struct Ammo(u32);
fn sees_intruder(seen: &Perceived<Sight>) -> bool {
    seen.active().any(Percept::is_hostile)
}

fn heard_something(heard: &Perceived<Hearing>) -> bool {
    !heard.is_empty()
}

# fn spawn(mut commands: Commands) {
commands.spawn((
    Guard,
    Sight::new(12.0).with_lose_range(15.0),
    Hearing::new(8.0),
    Team(1),
    Ammo(6),
    Facing(Dir3::X),
    Transform::from_xyz(0.0, 0.0, 0.0),
));
# }
```

`Perceived<S>` changes only when something is noticed, lost or forgotten. It
doesn't change as things move, or as something already heard is heard again.
So a tree reading `Perceived<Sight>` sleeps until something comes into view,
and a noise doesn't wake it.

## 2D and 3D

Perception works in 3D by default. The guard lives in a side-scroller, so
tell perception to work in the plane:

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

With [`Space::Planar`], perception works in the XY plane. It ignores z, which
2D uses for layering. Ahead is +X. In 3D, ahead is -Z, Bevy's forward.

## The senses

| Sense | Sustained | Senses |
|---|---|---|
| `Sight` | Yes | What's `Perceivable`, in range and in its cone |
| `Hearing` | No | `Noise` messages within range |
| `Touch` | No | `Touched` messages |
| `Shared` | No | `Share` messages from allies |

A sustained sense keeps a thing *active* while it lasts: in view, for sight.
When it stops, the thing is lost. The other senses are moments: a noise is
heard once.

## Sight

[`Sight`] sees what's [`Perceivable`] in range and in its cone. Make the
intruder perceivable:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component)] struct Intruder;
# fn spawn(mut commands: Commands) {
commands.spawn((Intruder, Perceivable, Team(2), Transform::from_xyz(8.0, 0.0, 0.0)));
# }
```

Sight has a few options:

```rust
# use std::f32::consts::FRAC_PI_4;
# use std::time::Duration;
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# fn sight() -> Sight {
Sight::new(12.0)
    .with_lose_range(15.0)
    .with_half_angle(FRAC_PI_4)
    .with_eye(Vec3::new(0.0, 1.6, 0.0))
    .with_interval(Duration::from_millis(100))
# }
```

| Option | What it does | Default |
|---|---|---|
| `range` | How far it notices things | Given |
| `lose_range` | How far it keeps seeing what it already sees | `range` |
| `half_angle` | How far off ahead it sees, to either side, in radians | π/2, the front half |
| `eye` | Where it looks from, relative to the agent | The origin |
| `interval` | How long between looks | 0.2 s |

A `lose_range` larger than `range` stops the intruder from flickering in and
out of view at the edge. A `half_angle` of π sees all around. An interval of
zero looks every tick.

Each agent looks at its own moment within its interval. So looks spread
across ticks instead of all landing on one.

### Facing

Sight looks the way the agent's transform does. In 2D that's +X, and a
negative x scale flips it. But a sprite flipped with `Sprite::flip_x` doesn't
turn its transform. Give the guard a [`Facing`] and keep it up to date
instead:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component)] struct Guard;
# // Bevy's own, without the sprite feature the doctests build with.
# #[derive(Component)] struct Sprite { flip_x: bool }
fn face(mut guards: Query<(&Sprite, &mut Facing), (With<Guard>, Changed<Sprite>)>) {
    for (sprite, mut facing) in &mut guards {
        let way = if sprite.flip_x { Dir3::NEG_X } else { Dir3::X };
        facing.set_if_neq(Facing(way));
    }
}
```

The guard needs a `Facing` for this query to find it. Spawn it with
`Facing(Dir3::X)`. The guard's sensors look the same way, unless they have a
`Facing` of their own.

### What's seen

What's in view is active. Its trace, in `Traces<Sight>`, says when and where
it came into view. Once lost, it's no longer active. Its trace then says when
and where it was lost. It stays like that until the guard forgets it
([Sensors, memory and events](sensors-memory.md)).

For where the intruder is *now*, read its `Transform`. Perception keeps no
live positions.

## Hearing

An agent with [`Hearing`] hears a [`Noise`] within its range. Your systems
write the noises. Here, the intruder's footsteps:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component)] struct Intruder;
# #[derive(Component)] struct Step;
fn footsteps(
    mut noises: MessageWriter<Noise>,
    steps: Query<(Entity, &GlobalTransform), (With<Intruder>, Added<Step>)>,
) {
    for (intruder, at) in &steps {
        noises.write(Noise::new(intruder, at.translation()).with_loudness(0.5));
    }
}
```

Loudness scales the range. A guard with `Hearing::new(8.0)` hears a noise of
loudness 0.5 within 4 units. The default loudness is 1.

What makes the noise is what's heard. An explosion is its own entity. An agent
doesn't hear its own noises.

## Touch

An agent with [`Touch`] feels what [`Touched`] it. Write the message when
something hits the guard:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Message)] struct Hit { victim: Entity, attacker: Entity }
fn hits(mut hit: MessageReader<Hit>, mut touches: MessageWriter<Touched>) {
    for hit in hit.read() {
        touches.write(Touched { agent: hit.victim, by: hit.attacker });
    }
}
```

With avian, contacts can write these for you
([Occlusion and attitude](occlusion-attitude.md)).

## Sharing

A guard that spots the intruder can tell the others. That's the guard's
`Shout` task. It writes a [`Share`]:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component, Clone)] struct Shout;
fn shout(mut tasks: Tasks<&Shout, Entity>, mut shares: MessageWriter<Share>) {
    tasks.for_each(|_, mut task, guard| {
        shares.write(Share::allies(guard, 20.0));
        task.succeed();
    });
}
```

Listeners are agents with [`Shared`]. They get what they're told as
`Perceived<Shared>`, and when and where as `Traces<Shared>`. Give the other guards `Shared` so they hear the shout.

| Builder | Who's told |
|---|---|
| `Share::allies(from, range)` | Agents on its `Team` within `range` |
| `Share::to(from, [..])` | The agents listed, wherever they are |
| `.about(entity)` | Only about that one thing |

Without `about`, the guard passes on everything it perceives, by every sense.
What it sees now is passed on where it is now. What it only remembers is
passed on from when and where it last sensed it. Each listener gives it its
own attitude. A listener keeps what it already knew if that's newer.

## Perceived, Percept and Traces

[`Perceived<S>`] holds one [`Percept`] per thing the agent perceives by `S`.
[`Traces<S>`] holds when and where each was sensed. It comes with
`Perceived<S>` and goes with it.

| On `Perceived<S>` | Gives |
|---|---|
| `iter()` | Everything it perceives or remembers |
| `active()` | Only what's active: in view, for sight |
| `get(entity)` | That thing's percept, if any |
| `is_empty()` | Whether it perceives or remembers nothing |

| On `Percept` | Gives |
|---|---|
| `entity()` | What it is |
| `attitude()` | Hostile, neutral or friendly |
| `is_hostile()`, `is_friendly()` | Shortcuts for the attitude |
| `is_active()` | Whether it's active now |

| On `Traces<S>` | Gives |
|---|---|
| `get(entity)` | When and where that thing was sensed: as it came into view or was lost, for sight; last, for the others |
| `iter()` | Each thing and its trace |

The two are apart so a tree can sleep through what it doesn't ask about. A
footstep from the intruder the guard already heard changes its trace, not
`Perceived<Hearing>`. A condition asking whether the guard heard anything
stays asleep. One reading `Traces<Hearing>` wakes.

## Pitfalls

- **Noisy noises.** Something writing a noise every tick wakes every
  listening tree every tick. Write noises at the rate they matter: a
  footstep, not a frame.
- **Nothing is seen.** The intruder needs `Perceivable`. And in 2D, set
  `Space::Planar`, or sight looks down -Z into the screen.
- **A flipped sprite sees behind it.** Use `Facing`, as above.
- **Where is it now?** A trace is where it was sensed, not where it is.
  Read the transform for that.

[`Perceived<S>`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/perceived/struct.Perceived.html
[`Percept`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/perceived/struct.Percept.html
[`Traces<S>`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/perceived/struct.Traces.html
[`Space::Planar`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/enum.Space.html
[`Sight`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/sense/struct.Sight.html
[`Perceivable`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/sense/struct.Perceivable.html
[`Facing`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/sense/struct.Facing.html
[`Hearing`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/sense/struct.Hearing.html
[`Noise`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/sense/struct.Noise.html
[`Touch`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/sense/struct.Touch.html
[`Touched`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/sense/struct.Touched.html
[`Share`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/sense/struct.Share.html
[`Shared`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/sense/struct.Shared.html
