# Tasks

A task is work the tree hands to your systems: chasing, shooting, reloading.
The tree spawns the task as an entity. Your system does the work and reports
how it went.

## Chasing and shooting

`task(Chase)` spawns an entity with `Chase`, a [`Task`], and a [`TaskOf`]
pointing at the guard. A system finds it with [`Tasks`], which pairs each
task with what you ask of its agent:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
#[derive(Component, Clone)]
struct Chase;

#[derive(Component, Clone)]
struct Shoot;

#[derive(Component, PartialEq)]
struct Ammo(u32);

fn chase(
    mut tasks: Tasks<&Chase, (&Perceived<Sight>, &mut Transform)>,
    places: Query<&GlobalTransform>,
    time: Res<Time>,
) {
    tasks.for_each(|_, mut task, (seen, mut at)| {
        let target = seen.active().find(|p| p.is_hostile());
        let Some(there) = target.and_then(|p| places.get(p.entity()).ok()) else {
            // Lost sight of it.
            task.fail();
            return;
        };
        let gap = there.translation().x - at.translation.x;
        if gap.abs() < 6.0 {
            // In range.
            task.succeed();
            return;
        }
        at.translation.x += gap.signum() * 4.0 * time.delta_secs();
    });
}

fn shoot(mut tasks: Tasks<&Shoot, &mut Ammo>) {
    tasks.for_each(|_, mut task, mut ammo| {
        if ammo.0 == 0 {
            task.fail();
            return;
        }
        ammo.0 -= 1;
        task.succeed();
    });
}
```

Add task systems after the trees run:

```rust,no_run
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# fn chase() {}
# fn shoot() {}
fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            ArborPlugins.with_space(Space::Planar),
        ))
        .add_systems(FixedUpdate, (chase, shoot).after(ArborSystems))
        .run();
}
```

## Reporting

- `task.succeed()` and `task.fail()` report. The first report counts. Later
  ones are ignored.
- A report wakes the tree. It reads the report on its next tick.
- `task.elapsed()` is how long the task has run, by the tree's clock. It
  counts in whatever schedule your task system runs, and goes on while the
  tree sleeps.
- [`Tasks`] skips tasks that already reported and tasks of paused trees.
- [`Tasks`] is a shortcut for a query of tasks and a query of agents, paired
  by `TaskOf`. You can write those two queries yourself.
- A task despawned by anything other than its tree fails.

## Reporting from commands

Observers and systems that don't query [`Task`] report through
[`TaskCommands`]. The guard's `Shout` is done the moment it starts:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
#[derive(Component, Clone)]
struct Shout;

fn shout(started: On<Add<Shout>>, mut commands: Commands) {
    // Call allies here.
    commands.entity(started.entity).succeed_task();
}
```

## Starting and ending

A task's components come and go with it. Use Bevy's own observers to set up
and clean up. The guard shows a muzzle flash while it shoots:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component, Clone)] struct Shoot;
#[derive(Component)]
struct MuzzleFlash;

fn flash_on(started: On<Add<Shoot>>, tasks: Query<&TaskOf>, mut commands: Commands) {
    if let Ok(of) = tasks.get(started.entity) {
        commands.entity(of.agent()).insert(MuzzleFlash);
    }
}

fn flash_off(ended: On<Remove<Shoot>>, tasks: Query<(&Task, &TaskOf)>, mut commands: Commands) {
    if let Ok((task, of)) = tasks.get(ended.entity) {
        // `None` if it was canceled before it reported.
        info!("shot: {:?}", task.outcome());
        commands.entity(of.agent()).remove::<MuzzleFlash>();
    }
}
```

`On<Remove<..>>` runs however the task goes: it reported, it was canceled, or
its agent was despawned. Its [`Task`] is still there, with the outcome.

An agent's [`AgentTasks`] lists its running tasks. Despawning the agent
despawns them.

## Names

A task is named after its bundle's type, or the first component of a tuple:
`task((Reload, Graceful))` is `Reload`. The name goes in its `Name` and in
the [`TreeTrace`]. Give your own with `task("Warning shot", Shoot)`.

## State

`State(bundle) => { .. }` puts `bundle` on the agent while its child runs,
and removes it when the child ends or is canceled. Use it for systems that
should work only during a branch. The guard is `Alert` for the whole fight:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component, Clone)] struct Chase;
# #[derive(Component, Clone)] struct Shoot;
# fn sees_intruder(seen: &Perceived<Sight>) -> bool { seen.active().any(Percept::is_hostile) }
#[derive(Component, Clone)]
struct Alert;

# fn tree() -> Behavior {
behavior_tree! {
    Select => {
        when(sees_intruder) => State(Alert) => {
            task(Chase),
            task(Shoot),
        },
    }
}
# }
```

A system filtering on `With<Alert>` then runs only while the guard fights.
A single task doesn't need `State`: filter on the task itself.

## Pitfalls

- A task that never reports holds its branch forever. Make sure every path
  in its system reports, or put a `Timeout` around it.
- Run task systems in the trees' schedule, `.after(ArborSystems)`. See
  [How it fits together](concepts.md#one-tick).

[`Task`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/task/struct.Task.html
[`TaskOf`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/task/struct.TaskOf.html
[`Tasks`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/task/struct.Tasks.html
[`TaskCommands`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/task/trait.TaskCommands.html
[`AgentTasks`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/task/struct.AgentTasks.html
[`TreeTrace`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/tree/struct.TreeTrace.html
