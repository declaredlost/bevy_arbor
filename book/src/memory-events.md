# Memory and events

## Memory

Each `Perceived<S>` remembers for its sense's `MEMORY`: ten seconds for sight
and what's shared, five for hearing and touch. What's no longer active fades
that long after it was last sensed, and is then forgotten. What's active, in
view, never fades.

Memory is the agent's, not the eye's, so two eyes share one. Give an agent its
own by putting its `Perceived` on it:

```rust
# use std::time::Duration;
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# fn spawn(mut commands: Commands) {
commands.spawn((
    Sight::new(12.0),
    Hearing::new(20.0),
    Perceived::<Hearing>::remembering(Duration::from_secs(30)),
    Transform::default(),
));
# }
```

`Duration::MAX` remembers for good. An agent with nothing due to fade costs
nothing that tick.

## Events

Triggered on the agent, for observers, by sense:

| Event | When |
|---|---|
| `Noticed<S>` | Something new comes to its attention, or is active again |
| `Lost<S>` | Something stops being active: out of view, for sight |
| `Forgotten<S>` | It forgets something |

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
fn bark(noticed: On<Noticed<Sight>>, names: Query<&Name>) {
    if let Ok(name) = names.get(noticed.what) {
        info!("{} spotted {name}", noticed.entity);
    }
}

# fn add(app: &mut App) {
app.add_observer(bark);
# }
```

Trees don't need them: they react to `Perceived<S>` changing, which carries
the same news. Events are for barks, alerts and UI.
