# Sensors, memory and events

This chapter gives the guard a lookout camera, a memory of where it last saw
the intruder, and a line to shout when the intruder slips away.

## Sensors

A sense can sit on its own entity, a *sensor*, instead of on the agent. The
sensor is [`SensorOf`] the agent. What it senses goes to the agent's
`Perceived<S>`, as if the agent sensed it itself.

The guard's post has a camera high on the wall, looking along the corridor
the guard patrols:

```rust
# use std::f32::consts::PI;
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# fn spawn(mut commands: Commands, guard: Entity) {
commands.spawn((
    Sight::new(30.0).with_half_angle(PI / 6.0),
    SensorOf(guard),
    Transform::from_xyz(-15.0, 4.0, 0.0),
));
# }
```

When the camera sees the intruder, so does the guard. Its `sees_intruder`
condition doesn't change.

- A sensor looks from its own transform. The camera stays on the wall while
  the guard walks.
- For a sensor that moves with the agent, eyes on a head say, parent it with
  `ChildOf(agent)` as well. `SensorOf` is only about perception.
- The sensor keeps nothing itself. Everything goes to the agent.
- Despawning the agent despawns its sensors. The agent's [`Sensors`] lists
  them.

An agent can have several sensors of one sense, and its own as well. For
sight, what any of them sees is active. It's lost only once none of them sees
it. The camera looking away doesn't lose what the guard still sees.

## Memory

When the intruder goes out of view, the guard doesn't forget it at once. The
percept stays in `Perceived<Sight>`, no longer active. Its trace says when and
where the guard lost it. That's what `remembers_intruder` reads:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component, Clone)] struct Patrol { from: f32, to: f32 }
# #[derive(Component, Clone)] struct Chase;
# #[derive(Component, Clone)] struct Search;
fn sees_intruder(seen: &Perceived<Sight>) -> bool {
    seen.active().any(Percept::is_hostile)
}

fn remembers_intruder(seen: &Perceived<Sight>) -> bool {
    seen.iter().any(|p| p.is_hostile() && !p.is_active())
}

# fn tree() -> Behavior {
behavior_tree! {
    Forever => {
        Select => {
            when(sees_intruder) => task(Chase),
            when(remembers_intruder) => task(Search),
            task(Patrol { from: -10.0, to: 10.0 }),
        }
    }
}
# }
```

The `Search` task walks to where the intruder was lost:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component, Clone)] struct Search;
fn search(time: Res<Time>, mut tasks: Tasks<&Search, (&Perceived<Sight>, &mut Transform)>) {
    tasks.for_each(|_, mut task, (seen, mut at)| {
        let last = seen
            .iter()
            .filter(|p| p.is_hostile() && !p.is_active())
            .find_map(Percept::trace);
        let Some(last) = last else {
            task.fail();
            return;
        };
        let gap = last.position.x - at.translation.x;
        if gap.abs() < 0.5 {
            task.succeed();
        } else {
            at.translation.x += gap.signum() * 3.0 * time.delta_secs();
        }
    });
}
```

When the memory fades, the intruder is forgotten. `remembers_intruder` turns
false, and the guard goes back to its patrol.

### How long it remembers

Each sense remembers for its [`Sense::MEMORY`]:

| Sense | Remembers for |
|---|---|
| `Sight` | 10 s |
| `Shared` | 10 s |
| `Hearing` | 5 s |
| `Touch` | 5 s |

Memory counts from when the thing was last sensed. What's active never fades:
the guard doesn't forget the intruder while it's looking at it. An agent with
nothing due to fade costs nothing to check that tick.

To change it for one agent, spawn it with its own `Perceived`:

```rust
# use std::time::Duration;
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component)] struct Guard;
# fn spawn(mut commands: Commands) {
commands.spawn((
    Guard,
    Sight::new(12.0).with_lose_range(15.0),
    Perceived::<Sight>::remembering(Duration::from_secs(30)),
    Transform::default(),
));
# }
```

`Duration::MAX` remembers for good.

Memory belongs to the agent, not to a sensor. The guard and its camera share
one memory of sight.

## Events

Perception triggers events on the agent, one set per sense:

| Event | When |
|---|---|
| [`Noticed<S>`] | Something comes to its attention, or is active again |
| [`Lost<S>`] | Something stops being active: out of view, for sight |
| [`Forgotten<S>`] | It forgets something |

`entity` is the agent. `what` is the thing. Observe them for barks, alerts and
UI:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
fn where_did_they_go(lost: On<Lost<Sight>>, names: Query<&Name>) {
    if let Ok(name) = names.get(lost.what) {
        info!("{}: where did {name} go?", lost.entity);
    }
}

# fn add(app: &mut App) {
app.add_observer(where_did_they_go);
# }
```

Only sustained senses, like sight, lose things. A noise is noticed, then
forgotten.

## Pitfalls

- **Don't drive trees with events.** A tree already wakes when
  `Perceived<S>` changes, which carries the same news. Use conditions such as
  `remembers_intruder`, and keep events for side effects.
- **A sensor that doesn't follow.** `SensorOf` doesn't move the sensor. Add
  `ChildOf` for that.
- **`trace` is the past.** For the intruder's position while in view, read
  its transform.

[`SensorOf`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/sense/struct.SensorOf.html
[`Sensors`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/sense/struct.Sensors.html
[`Sense::MEMORY`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/sense/trait.Sense.html#associatedconstant.MEMORY
[`Noticed<S>`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/perceived/struct.Noticed.html
[`Lost<S>`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/perceived/struct.Lost.html
[`Forgotten<S>`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/perceived/struct.Forgotten.html
