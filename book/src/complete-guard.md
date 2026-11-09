# The complete guard

This chapter puts every part of the guard in one listing. Each part is
explained in the chapter linked next to it. Read it top to bottom: the app,
what's in the world, the tree, then the task systems.

## What it does

- It patrols a stretch of wall. ([Your first agent](first-agent.md))
- When it sees the intruder, it shouts to its allies, at most every ten
  seconds. ([Decorators](decorators.md), [Senses](senses.md))
- If a turret is free, it takes it and fires from it.
  ([Smart objects](smart-objects.md))
- Otherwise it chases the best intruder in view and shoots. A shot, once
  started, finishes. ([Queries](queries.md), [Interruption](interruption.md))
- Out of ammo, it finds cover the intruder can't see and reloads there.
  ([Running queries](running-queries.md))
- It walks to cover and to the turret with `MoveTo`, and one movement system
  of its own takes it there. ([Navigation](navigation.md))
- When it loses the intruder, it searches where it was last seen.
  ([Sensors, memory and events](sensors-memory.md))
- Walls block its sight. ([Occlusion and attitude](occlusion-attitude.md))

## The listing

```rust,no_run
use std::time::Duration;

use avian2d::prelude::*;
use bevy::prelude::*;
use bevy_arbor::avian2d::eqs::{Clearance, LineOfSight, OccluderDistance};
use bevy_arbor::prelude::*;

// The guard, the intruder, and what the guard does.

#[derive(Component)]
struct Guard;

#[derive(Component)]
struct Intruder;

#[derive(Component, PartialEq)]
struct Ammo(u32);

#[derive(Component, Clone)]
struct Patrol { from: f32, to: f32 }

#[derive(Component, Clone)]
struct Chase;

#[derive(Component, Clone)]
struct Search;

#[derive(Component, Clone)]
struct Shoot;

#[derive(Component, Clone)]
struct Reload;

#[derive(Component, Clone)]
struct Shout;

#[derive(Component, Clone)]
struct FireTurret;

// The turret on the wall, and its gunner's slot.

#[derive(Component)]
struct WallTurret;

#[derive(Component)]
struct Gunner;

// Query keys.

/// Somewhere out of the intruder's sight.
struct Cover;

/// The intruder to shoot at.
struct Target;

/// The nearest free turret.
struct Turret;

// The app.

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            PhysicsPlugins::default(),
            ArborPlugins.with_space(Space::Planar),
        ))
        .add_env_query(Cover, cover())
        .add_env_query(Target, target())
        .add_env_query(Turret, turret())
        .add_systems(Startup, spawn)
        .add_systems(
            FixedUpdate,
            (
                systems::patrol,
                systems::chase,
                systems::shoot,
                systems::reload,
                systems::search,
                systems::shout,
                systems::travel,
                systems::fire_turret,
            )
                .after(ArborSystems),
        )
        .run();
}

fn spawn(mut commands: Commands) {
    commands.spawn((
        Guard,
        Sight::new(12.0).with_lose_range(15.0),
        Hearing::new(8.0),
        Shared,
        Team(1),
        Facing(Dir3::X),
        Ammo(6),
        Transform::from_xyz(0.0, 0.0, 0.0),
        BehaviorTree::new(guard()),
    ));
    // The intruder: sight finds it through its collider.
    commands.spawn((
        Intruder,
        Perceivable,
        Team(2),
        Collider::capsule(0.4, 1.0),
        Transform::from_xyz(30.0, 0.0, 0.0),
    ));
    // A wall: it blocks sight.
    commands.spawn((RigidBody::Static, Collider::rectangle(1.0, 6.0), Transform::from_xyz(5.0, 0.0, 0.0)));
    // A turret, with one place for a gunner.
    commands
        .spawn((WallTurret, Transform::from_xyz(-6.0, 2.0, 0.0)))
        .with_related::<SlotOf>((Gunner, Plugged::new(Using, fire_turret), Transform::from_xyz(-6.0, 0.0, 0.0)));
}

// Conditions.

fn sees_intruder(seen: &Perceived<Sight>) -> bool {
    seen.active().any(Percept::is_hostile)
}

fn remembers_intruder(seen: &Perceived<Sight>) -> bool {
    seen.iter().any(|p| p.is_hostile() && !p.is_active())
}

fn out_of_ammo(ammo: &Ammo) -> bool {
    ammo.0 == 0
}

// The tree.

fn guard() -> Behavior {
    behavior_tree! {
        Forever => {
            Select => {
                // Call allies, and look for a turret.
                when(sees_intruder && cooldown(10.0)) => Sequence => {
                    task(Shout),
                    task(Survey::of(Turret)),
                },
                when(sees_intruder && found::<Turret> && claim::<Turret>()) => Sequence => {
                    task(MoveTo::found::<Turret>()),
                    Socket(Using),
                },
                when(sees_intruder) => @ fight(),
                when(remembers_intruder) => @ search(),
                @ patrol(),
            }
        }
    }
}

fn fight() -> Behavior {
    behavior_tree! {
        Select => {
            when(out_of_ammo) => Sequence => {
                task(Survey::of(Cover)),
                task(MoveTo::found::<Cover>()),
                task((Reload, Graceful)),
            },
            Sequence => {
                task(Survey::of(Target)),
                task(Chase),
                Commit => { task(Shoot) },
                Wait(0.8),
            },
        }
    }
}

fn search() -> Behavior {
    behavior_tree! { task(Search) }
}

fn patrol() -> Behavior {
    behavior_tree! { task(Patrol { from: -10.0, to: 10.0 }) }
}

/// What the turret's slot brings: fire it.
fn fire_turret() -> Behavior {
    behavior_tree! { task(FireTurret) }
}

// The queries.

fn cover() -> EnvQuery {
    let intruders = || Hostiles::<Sight>::remembered();
    EnvQuery::new(Donut::around(Agent).radius(2.0, 8.0).rings(3).points(16))
        .filter(Clearance::of(Collider::circle(0.4)), Filter::is(true))
        .filter(LineOfSight::from(intruders()), Filter::is(false))
        .score(OccluderDistance::new(), Score::lower())
        .score(Distance::to(Agent), Score::lower().weight(0.5))
        .score(Distance::to(intruders()), Score::higher().weight(0.5))
}

fn target() -> EnvQuery {
    EnvQuery::new(Percepts::<Sight>::all().active().hostile())
        .score(Distance::to(Agent), Score::lower())
        .score(Dot::of(Agent), Score::higher().weight(0.5))
}

fn turret() -> EnvQuery {
    EnvQuery::new(FreeSlots::<Gunner>::within(Agent, 20.0))
        .score(Distance::to(Agent), Score::lower())
}

// The task systems.

mod systems {
    use super::*;

    const SPEED: f32 = 3.0;

    /// Moves `at` toward `x` along the ground; true once there.
    fn walk(at: &mut Transform, x: f32, time: &Time) -> bool {
        let gap = x - at.translation.x;
        at.translation.x += gap.clamp(-1.0, 1.0) * SPEED * time.delta_secs();
        gap.abs() < 0.1
    }

    pub fn patrol(mut tasks: Tasks<&Patrol, (&mut Transform, &mut Facing)>, time: Res<Time>) {
        tasks.for_each(|patrol, _, (mut at, mut facing)| {
            let x = at.translation.x;
            if x >= patrol.to {
                facing.set_if_neq(Facing(Dir3::NEG_X));
            } else if x <= patrol.from {
                facing.set_if_neq(Facing(Dir3::X));
            }
            at.translation += facing.0 * SPEED * time.delta_secs();
        });
    }

    pub fn chase(
        mut tasks: Tasks<&Chase, (&Found<Target>, &mut Transform)>,
        places: Query<&GlobalTransform>,
        time: Res<Time>,
    ) {
        tasks.for_each(|_, mut task, (target, mut at)| {
            let Some(there) = target.entity().and_then(|e| places.get(e).ok()) else {
                task.fail();
                return;
            };
            // Close enough to shoot.
            if (there.translation().x - at.translation.x).abs() < 6.0 {
                task.succeed();
                return;
            }
            walk(&mut at, there.translation().x, &time);
        });
    }

    pub fn shoot(mut tasks: Tasks<&Shoot, &mut Ammo>) {
        tasks.for_each(|_, mut task, mut ammo| {
            if ammo.0 == 0 {
                task.fail();
                return;
            }
            ammo.0 -= 1;
            task.succeed();
        });
    }

    pub fn reload(mut tasks: Tasks<&Reload, &mut Ammo>) {
        const RELOAD: Duration = Duration::from_secs(2);
        tasks.for_each(|_, mut task, mut ammo| {
            if task.elapsed() >= RELOAD {
                ammo.0 = 6;
                task.succeed();
            } else if task.interrupted() && task.elapsed() < RELOAD / 2 {
                task.fail();
            }
        });
    }

    pub fn search(
        mut tasks: Tasks<&Search, (&Perceived<Sight>, &Traces<Sight>, &mut Transform)>,
        time: Res<Time>,
    ) {
        tasks.for_each(|_, mut task, (seen, traces, mut at)| {
            let intruder = seen.iter().find(|p| p.is_hostile());
            let last_seen = intruder.and_then(|p| traces.get(p.entity()));
            if task.elapsed() > Duration::from_secs(8) {
                task.fail();
            } else if let Some(there) = last_seen {
                walk(&mut at, there.position.x, &time);
            }
        });
    }

    pub fn shout(mut tasks: Tasks<&Shout, Entity>, mut shares: MessageWriter<Share>) {
        tasks.for_each(|_, mut task, guard| {
            shares.write(Share::allies(guard, 20.0));
            task.succeed();
        });
    }

    /// The guard's movement: walks it toward its `Destination`, set by a
    /// `MoveTo`, and says how it goes.
    pub fn travel(
        mut guards: Query<(Entity, &Destination, &mut Transform, Option<&Travel>)>,
        places: Query<&GlobalTransform>,
        time: Res<Time>,
        mut commands: Commands,
    ) {
        for (guard, destination, mut at, travel) in &mut guards {
            let Some(goal) = destination.goal.position(&places) else {
                continue;
            };
            let now = if (goal.x - at.translation.x).abs() <= destination.within {
                Travel::Arrived
            } else {
                walk(&mut at, goal.x, &time);
                Travel::Moving
            };
            if travel != Some(&now) {
                commands.entity(guard).insert(now);
            }
        }
    }

    pub fn fire_turret(mut tasks: Tasks<&FireTurret, Entity>) {
        tasks.for_each(|_, _, _gunner| {
            // Aim the turret and fire. It runs until the guard leaves it.
        });
    }
}
```

## How it reads

- **The top of the tree is a priority list.** `Select` runs the first branch
  that passes. Shouting comes first, so it interrupts a fight once every ten
  seconds. See [Building trees](trees.md) and
  [Large trees and sockets](large-trees.md).
- **The turret branch has three decorators.** It passes only while the guard
  sees the intruder, has found a turret, and can claim it. See
  [Smart objects](smart-objects.md).
- **The fight runs two queries.** `Target` picks whom to chase. `Cover` picks
  where to reload. See [Running queries](running-queries.md).
- **`MoveTo` asks, `travel` moves.** The tree says where the guard should be.
  The `travel` system walks it there and writes `Travel`. See
  [Navigation](navigation.md).
- **Timing lives in the tasks.** The search gives up after eight seconds. The
  reload takes two. See [Tasks](tasks.md).
- **The tree sleeps on patrol.** Every condition is typed, so the tree wakes
  only when what they read changes. See [Sleeping trees](sleeping.md).

## Pitfalls

- **The task schedule.** Task systems go in the trees' schedule, after
  `ArborSystems`. See [How it fits together](concepts.md#one-tick).
- **`Facing` written every tick.** The patrol uses `set_if_neq`. See
  [Sleeping trees](sleeping.md#keeping-trees-asleep).
- **Physics for walls.** The wall and the cover query's `LineOfSight` need
  avian's `PhysicsPlugins`. Without them, sight isn't blocked and the cover
  test has no answer.
