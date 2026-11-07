# Testing

AI is easiest to test headless, a tick at a time. [bevy_harness] steps a Bevy
app tick by tick, with time that moves one fixed step a tick, and checks it
between ticks:

```rust
# use std::f32::consts::PI;
# use std::time::Duration;
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
use bevy_harness::TestApp;

#[derive(Component, Clone)]
struct Chase;

#[derive(Component, Clone)]
struct Wander;

fn sees_enemy(seen: &Perceived<Sight>) -> bool {
    seen.active().any(Percept::is_hostile)
}

# fn main() {
let mut app = TestApp::builder().plugins(ArborPlugins).build();
app.spawn((
    Sight::new(10.0).with_half_angle(PI).with_interval(Duration::ZERO),
    Team(1),
    GlobalTransform::IDENTITY,
    BehaviourTree::new(behaviour_tree! {
        Forever => {
            Select => {
                when(sees_enemy) => task("Chase", Chase),
                task("Wander", Wander),
            }
        }
    }),
));
let enemy = app.spawn((Perceivable, Team(2), GlobalTransform::from_xyz(0.0, 0.0, -20.0)));

app.tick();
assert_eq!(app.count::<With<Wander>>(), 1);

app.entity_mut(enemy).insert(GlobalTransform::from_xyz(0.0, 0.0, -5.0));
app.tick();
assert_eq!(app.count::<With<Chase>>(), 1);
# }
```

A few things make tests simpler:

- **Look every tick**: `with_interval(Duration::ZERO)`, so a test needn't wait
  for an agent's look.
- **Set `GlobalTransform` directly**: without Bevy's transform plugin, nothing
  propagates `Transform`, and perception reads the global one.
- **Write messages with `app.write`**: a `Noise` or a `Touched`, read the next
  tick.
- **Let time pass with `app.secs`**: for waits, timeouts and memory.

[bevy_harness]: https://github.com/declaredlost/bevy_harness
