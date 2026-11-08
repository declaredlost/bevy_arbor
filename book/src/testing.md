# Testing

AI is easiest to test headless, one tick at a time. You place the guard and
the intruder, tick, and check which task the guard is running.
[bevy_harness] does the ticking. Each tick moves time by one fixed step, so
tests are repeatable.

Add it as a dev-dependency:

```toml
[dev-dependencies]
bevy_harness = { git = "https://github.com/declaredlost/bevy_harness" }
```

## The guard chases what it sees

This test gives the guard the tree from [Your first agent](first-agent.md).
It checks that the guard patrols, chases the intruder once it's in view, and
patrols again once it's out of view:

```rust
# use std::time::Duration;
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
use bevy_harness::TestApp;

# #[derive(Component, Clone)] struct Patrol { from: f32, to: f32 }
# #[derive(Component, Clone)] struct Chase;
# fn sees_intruder(seen: &Perceived<Sight>) -> bool { seen.active().any(Percept::is_hostile) }
# fn guard() -> Behavior {
#     behavior_tree! {
#         Forever => {
#             Select => {
#                 when(sees_intruder) => task(Chase),
#                 task(Patrol { from: -10.0, to: 10.0 }),
#             }
#         }
#     }
# }
/// An app with the guard at the origin, looking right.
fn guard_app() -> (TestApp, Entity) {
    let mut app = TestApp::builder()
        .plugins(ArborPlugins.set(PerceptionPlugin::default().with_space(Space::Planar)))
        .build();
    let guard = app.spawn((
        Sight::new(12.0).with_lose_range(15.0).with_interval(Duration::ZERO),
        Team(1),
        Facing(Dir3::X),
        GlobalTransform::IDENTITY,
        BehaviorTree::new(guard()),
    ));
    (app, guard)
}

# fn main() {
// #[test]
// fn the_guard_chases_what_it_sees() {
let (mut app, _guard) = guard_app();
let intruder = app.spawn((Perceivable, Team(2), GlobalTransform::from_xyz(30.0, 0.0, 0.0)));

// Too far to see: it patrols.
app.tick();
assert_eq!(app.count::<With<Patrol>>(), 1);

// In view: it chases.
app.entity_mut(intruder).insert(GlobalTransform::from_xyz(10.0, 0.0, 0.0));
app.tick();
assert_eq!(app.count::<With<Chase>>(), 1);
assert_eq!(app.count::<With<Patrol>>(), 0);

// Out past the lose range: it patrols again.
app.entity_mut(intruder).insert(GlobalTransform::from_xyz(20.0, 0.0, 0.0));
app.tick();
assert_eq!(app.count::<With<Chase>>(), 0);
assert_eq!(app.count::<With<Patrol>>(), 1);
// }
# }
```

In your crate, the body is a `#[test]` function. Tasks are entities, so
`app.count::<With<Chase>>()` counts the guard's running `Chase` tasks.

## Keeping tests simple

- **Look every tick.** `Sight::with_interval(Duration::ZERO)` makes the guard
  look every tick. The default looks five times a second, at a moment of its
  own.
- **Set `GlobalTransform` directly.** The test app has no transform plugin, so
  nothing copies `Transform` into `GlobalTransform`. Perception reads the
  global one.
- **Leave out task systems you don't test.** Without a `chase` system, the
  `Chase` task just keeps running. That's enough to see the tree chose it.
- **Write messages with `app.write`.** A `Noise` or a `Touched` is read on the
  next tick.
- **Let time pass with `app.secs(..)`.** Use it for `Wait`, cooldowns,
  timeouts and memory.
- **Wait for a condition with `app.until(..)`.** It ticks until the condition
  holds, and fails the test after a limit:
  `app.until(|world| ..).within_secs(5.0)`.

## Checking the tree itself

To check more than which task runs, add a [`TreeTrace`] to the guard and look
at what's running. See [Debugging](debugging.md):

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
fn running(trace: &TreeTrace) -> Vec<&str> {
    trace.running().map(|(_, node)| node.label()).collect()
}
```

## Pitfalls

- **One tick isn't always enough.** The tree reads a task's report on the
  tick after the task gives it. A query asked for by a tree runs that tick, and
  the tree sees its result the next. Tick twice, or use `until`.
- **The space.** A 2D game's tests need `Space::Planar` too. Without it,
  perception works in 3D, where ahead is -Z unless `Facing` says otherwise.

[bevy_harness]: https://github.com/declaredlost/bevy_harness
[`TreeTrace`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/tree/struct.TreeTrace.html
