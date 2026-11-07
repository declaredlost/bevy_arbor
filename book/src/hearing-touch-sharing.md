# Hearing, touch and sharing

The other built-in senses come from messages your systems write.

## Hearing

An agent with [`Hearing`] hears a [`Noise`] within its range times the noise's
loudness:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
fn footsteps(mut noises: MessageWriter<Noise>, walkers: Query<(Entity, &GlobalTransform), Added<Walking>>) {
    for (walker, at) in &walkers {
        noises.write(Noise::new(walker, at.translation()).with_loudness(0.5));
    }
}
# #[derive(Component)] struct Walking;
```

What makes the noise is what's heard, so an explosion is its own entity. An
agent doesn't hear its own noises. Something making a noise every tick wakes
every tree listening every tick: make noises at the rate they matter, a
footstep, not a frame.

## Touch

An agent with [`Touch`] feels what [`Touched`] it, where that is:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
fn hits(mut touches: MessageWriter<Touched>, mut hit: MessageReader<Hit>) {
    for hit in hit.read() {
        touches.write(Touched { agent: hit.victim, by: hit.attacker });
    }
}
# #[derive(Message)] struct Hit { victim: Entity, attacker: Entity }
```

## Sharing

An agent tells others what it perceives, by every sense, with [`Share`]: a
whole squad reacts when one spots you. Listeners are agents with [`Shared`],
and take it as `Perceived<Shared>`:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
fn shout(mut shares: MessageWriter<Share>, spotted: Query<Entity, Changed<Perceived<Sight>>>) {
    for agent in &spotted {
        shares.write(Share::allies(agent, 20.0));
    }
}
```

`Share::allies(from, range)` tells agents on its team within range;
`Share::to(from, [..])` tells those listed, wherever they are; `.about(entity)`
tells about one thing only.

What the teller has active, in view, is passed on where it is now; what it
remembers, from when and where it last sensed it. Each listener gives it its
own attitude, and keeps what it knew if that's newer.

[`Hearing`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/sense/struct.Hearing.html
[`Noise`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/sense/struct.Noise.html
[`Touch`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/sense/struct.Touch.html
[`Touched`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/sense/struct.Touched.html
[`Share`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/sense/struct.Share.html
[`Shared`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/sense/struct.Shared.html
