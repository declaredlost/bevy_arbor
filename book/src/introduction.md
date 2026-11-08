# Introduction

bevy_arbor is AI for [Bevy](https://bevyengine.org). It gives your agents
behavior trees to decide what to do, and perception to tell them what they
sense. Environment queries help them pick a place or a target. Smart objects
let them find and use things in the world. Navigation connects them to your
game's movement.

This book builds one agent from start to finish: a guard in a 2D
side-scroller. Here it is after the first chapter:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component)] struct Guard;
# #[derive(Component, Clone)] struct Patrol { from: f32, to: f32 }
# #[derive(Component, Clone)] struct Chase;
fn sees_intruder(seen: &Perceived<Sight>) -> bool {
    seen.active().any(Percept::is_hostile)
}

fn spawn_guard(mut commands: Commands) {
    commands.spawn((
        Guard,
        Sight::new(12.0).with_lose_range(15.0),
        Team(1),
        Facing(Dir3::X),
        Transform::from_xyz(0.0, 0.0, 0.0),
        BehaviorTree::new(behavior_tree! {
            Forever => {
                Select => {
                    when(sees_intruder) => task(Chase),
                    task(Patrol { from: -10.0, to: 10.0 }),
                }
            }
        }),
    ));
}
```

The guard patrols until it sees the intruder. Then it chases. When the
intruder gets away, the guard goes back to its patrol. Later chapters teach
it to shoot, reload, take cover, call for help and man a turret.

## What you get

- **Reactive behavior trees.** A branch switches the moment its condition
  changes. What was running is canceled.
- **Trees that sleep.** A tree is ticked only when something it waits on
  changes. An idle tree costs almost nothing.
- **Tasks as entities.** Each task the tree starts is an entity. Your own
  systems do the work.
- **Perception.** Sight, hearing, touch, and allies sharing what they know.
  It works in 2D and 3D, and you can add senses of your own.
- **Environment queries.** Score places or things around an agent and pick
  the best one, such as cover or a target.
- **Smart objects.** Things in the world with slots an agent can claim and
  use, such as a turret with a gunner's seat.
- **Navigation, your way.** A `MoveTo` task and path costs for queries,
  over whatever pathfinding and movement your game has.
- **No physics required.** Line of sight is a system you can replace. With
  avian, line of sight and touch come ready-made.

## The crates

Most projects only depend on `bevy_arbor`. It re-exports the other crates
behind feature flags.

| Crate | Contains |
|---|---|
| `bevy_arbor` | Every part, with one prelude and one plugin group |
| `bevy_arbor_bt` | Behavior trees, tasks and decorators |
| `bevy_arbor_perception` | Senses, sensors and memory |
| `bevy_arbor_eqs` | Environment queries |
| `bevy_arbor_smart` | Smart objects |
| `bevy_arbor_nav` | Navigation: `MoveTo` and path costs |
| `bevy_arbor_gizmos` | Debug drawing |
| `bevy_arbor_avian2d`, `bevy_arbor_avian3d` | Line of sight and touch with avian |
| `bevy_arbor_macros` | The `behavior_tree!` macro |

This book is the guide. The API reference is on
[docs.rs](https://docs.rs/bevy_arbor).
