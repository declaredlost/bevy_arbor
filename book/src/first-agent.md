# Your first agent

This chapter builds the monster from the introduction: it wanders until it
sees the player, chases it, and wanders again once it's lost it. It's a 2D
game, so perception works in the plane.

## The plugins

```rust,no_run
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component, Clone)] struct Chase;
# #[derive(Component, Clone)] struct Wander;
# fn spawn(mut commands: Commands) {}
# fn chase() {}
# fn wander() {}
fn main() {
    App::new()
        .add_plugins((
            MinimalPlugins,
            ArborPlugins.set(PerceptionPlugin::default().with_space(Space::Planar)),
        ))
        .add_systems(Startup, spawn)
        .add_systems(FixedUpdate, (chase, wander).after(ArborSystems))
        .run();
}
```

Trees and perception run in `FixedUpdate`, in [`ArborSystems`]: perception
first, then trees. Task systems, which do the tree's work, go after it, so a
task is ticked the tick it starts.

## The agent

The monster sees 12 units ahead, keeps seeing what it already sees out to 15,
and is on team 1. Its tree is a `Select`: the first branch whose guard passes
runs.

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
#[derive(Component, Clone)]
struct Chase;

#[derive(Component, Clone)]
struct Wander;

fn sees_enemy(seen: &Perceived<Sight>) -> bool {
    seen.active().any(Percept::is_hostile)
}

fn spawn(mut commands: Commands) {
    commands.spawn((
        Sight::new(12.0).with_lose_range(15.0),
        Team(1),
        Transform::default(),
        BehaviourTree::new(behaviour_tree! {
            Forever => {
                Select => {
                    when(sees_enemy) => task("Chase", Chase),
                    task("Wander", Wander),
                }
            }
        }),
    ));
    commands.spawn((Perceivable, Team(2), Transform::from_xyz(30.0, 0.0, 0.0)));
}
```

The player is `Perceivable`, on team 2: to the monster, hostile.

`sees_enemy` is a *typed guard*: a function of components on the agent. The
tree asks it again only when the monster's `Perceived<Sight>` changes, which
happens only when something comes into view or goes out of it. In between,
the tree sleeps.

## The tasks

`task("Chase", Chase)` spawns an entity with a `Chase` component while its
branch runs, and despawns it when the branch is cancelled. Your systems tick
tasks, through [`Tasks`], which pairs each task with what you want of its
agent:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component, Clone)] struct Chase;
# #[derive(Component, Clone)] struct Wander;
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
            let towards = (target.translation() - transform.translation).normalize_or_zero();
            transform.translation += towards * 4.0 * time.delta_secs();
        }
    });
}

fn wander(mut tasks: Tasks<&Wander, &mut Transform>, time: Res<Time>) {
    tasks.for_each(|_, _, mut transform| {
        transform.translation.x += time.delta_secs();
    });
}
```

The chase reads where the player is now from its transform. `Perceived`
doesn't keep live positions: that would change it every frame and wake the
tree for nothing.

When the player comes into view, the tree wakes, `sees_enemy` passes, the
`Wander` task is despawned and a `Chase` task spawned, in the same tick. When
the player is lost, the opposite.

[`ArborSystems`]: https://docs.rs/bevy_arbor/latest/bevy_arbor/struct.ArborSystems.html
[`Tasks`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/task/struct.Tasks.html
