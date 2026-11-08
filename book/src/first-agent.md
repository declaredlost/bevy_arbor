# Your first agent

In this chapter the guard learns to patrol. Then it learns to chase the
intruder when it sees one.

## The app

Start with the plugins from [Installation](installation.md). Perception is
set to 2D, and the task systems run after [`ArborSystems`]:

```rust,no_run
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# fn spawn(_: Commands) {}
# fn patrol() {}
fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            ArborPlugins.set(PerceptionPlugin::default().with_space(Space::Planar)),
        ))
        .add_systems(Startup, spawn)
        .add_systems(FixedUpdate, patrol.after(ArborSystems))
        .run();
}
```

## Patrolling

A tree decides what to do. A task does it. `Patrol` is a task: a component
of yours, with what the guard needs to know.

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
#[derive(Component)]
struct Guard;

#[derive(Component, Clone)]
struct Patrol {
    from: f32,
    to: f32,
}

fn guard() -> Behavior {
    behavior_tree! {
        Forever => {
            task(Patrol { from: -10.0, to: 10.0 })
        }
    }
}

fn spawn(mut commands: Commands) {
    commands.spawn((
        Guard,
        Facing(Dir3::X),
        Transform::from_xyz(0.0, 0.0, 0.0),
        BehaviorTree::new(guard()),
    ));
}
```

When the tree reaches `task(Patrol { .. })`, it spawns a task entity with a
`Patrol` component. The task is linked to the guard. The tree keeps it alive
until the branch ends.

`Forever` restarts its child whenever it finishes. Here, the patrol never
finishes, so it just keeps going.

## Doing the work

Your systems do the work of a task. [`Tasks`] pairs each task with
components of its agent. Here it pairs each `Patrol` with the guard's
transform and facing:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component, Clone)] struct Patrol { from: f32, to: f32 }
fn patrol(
    mut tasks: Tasks<&Patrol, (&mut Transform, &mut Facing)>,
    time: Res<Time>,
) {
    tasks.for_each(|patrol, _task, (mut transform, mut facing)| {
        let x = transform.translation.x;
        // Only a turn changes `Facing`, so the guard's tree isn't woken
        // every tick.
        if x >= patrol.to {
            facing.set_if_neq(Facing(Dir3::NEG_X));
        } else if x <= patrol.from {
            facing.set_if_neq(Facing(Dir3::X));
        }
        transform.translation += facing.0 * 3.0 * time.delta_secs();
    });
}
```

The guard walks to one end, turns, and walks back. It keeps [`Facing`] up to
date, so it looks where it walks. In a side-scroller the sprite flips but the
transform doesn't turn, so sight reads `Facing` instead.

The second argument is the task's [`Task`] component. A task that should end
reports through it, with `task.succeed()` or `task.fail()`. The patrol never
ends, so it ignores it.

## Seeing the intruder

Now give the guard eyes, and something to see:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component)] struct Guard;
# #[derive(Component, Clone)] struct Patrol { from: f32, to: f32 }
#[derive(Component, Clone)]
struct Chase;

#[derive(Component)]
struct Intruder;

fn sees_intruder(seen: &Perceived<Sight>) -> bool {
    seen.active().any(Percept::is_hostile)
}

fn guard() -> Behavior {
    behavior_tree! {
        Forever => {
            Select => {
                when(sees_intruder) => task(Chase),
                task(Patrol { from: -10.0, to: 10.0 }),
            }
        }
    }
}

fn spawn(mut commands: Commands) {
    commands.spawn((
        Guard,
        Sight::new(12.0).with_lose_range(15.0),
        Team(1),
        Facing(Dir3::X),
        Transform::from_xyz(0.0, 0.0, 0.0),
        BehaviorTree::new(guard()),
    ));
    commands.spawn((
        Intruder,
        Perceivable,
        Team(2),
        Transform::from_xyz(30.0, 0.0, 0.0),
    ));
}
```

The guard sees 12 units ahead. Once it sees something, it keeps seeing it up
to 15 units away. The intruder is [`Perceivable`] and on another team, so to
the guard it is hostile.

`Select` runs the first branch whose condition passes. While the guard sees
the intruder, that's `Chase`. Otherwise it's `Patrol`.

`sees_intruder` is a *typed condition*: a plain function of components on the
agent. The tree asks it again only when [`Perceived<Sight>`] changes. That
happens only when something comes into view or leaves it. In between, the
tree sleeps.

## Chasing

The chase system finds the intruder among what the guard sees, and walks
toward it:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component, Clone)] struct Chase;
fn chase(
    mut tasks: Tasks<&Chase, (&Perceived<Sight>, &mut Transform)>,
    targets: Query<&GlobalTransform>,
    time: Res<Time>,
) {
    tasks.for_each(|_, _, (seen, mut transform)| {
        let target = seen
            .active()
            .find(|percept| percept.is_hostile())
            .and_then(|percept| targets.get(percept.entity()).ok());
        if let Some(target) = target {
            let toward = (target.translation() - transform.translation).normalize_or_zero();
            transform.translation += toward * 4.0 * time.delta_secs();
        }
    });
}
```

Add `chase` next to `patrol`, after `ArborSystems`.

When the intruder comes into view, the tree wakes. `sees_intruder` passes.
The `Patrol` task is despawned and a `Chase` task is spawned, in the same
tick. When the intruder gets away, the opposite happens.

## Pitfalls

- **Live positions.** `Perceived` doesn't hold where things are now. That
  would change it every frame and wake the tree for nothing. Read the
  target's transform, as `chase` does.
- **System order.** Run task systems after `ArborSystems`. Otherwise a task
  spawned this tick may wait a tick before it moves.
- **Task components must be `Clone`.** The tree spawns a fresh copy each time
  the branch starts.

Next, [How it fits together](concepts.md) shows how these parts share a
tick. [Building trees](trees.md) teaches the guard to fight.

[`ArborSystems`]: https://docs.rs/bevy_arbor/latest/bevy_arbor/struct.ArborSystems.html
[`Tasks`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/task/struct.Tasks.html
[`Task`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/task/struct.Task.html
[`Facing`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/sense/struct.Facing.html
[`Perceivable`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/sense/struct.Perceivable.html
[`Perceived<Sight>`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/perceived/struct.Perceived.html
