# Running queries

The [previous chapter](queries.md) built the guard's `Cover` and `Target`
queries. This one gives them to the guard, runs them from its tree, reads
what they found, and asks queries that things carry. It ends with generators,
tests and anchors of your own.

## Carrying and running a query

A query is a component, and an entity carries it. The guard carries the
queries it asks of itself, spawned alongside its tree:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# struct Cover;
# struct Target;
# fn cover() -> EnvQuery<Cover> { EnvQuery::new(Cover, Donut::around(Querier)) }
# fn target() -> EnvQuery<Target> { EnvQuery::new(Target, Donut::around(Querier)) }
# fn guard() -> Behavior { behavior_tree! { task(Survey::of(Cover)) } }
fn spawn_guard(mut commands: Commands) {
    commands.spawn((
        Transform::default(),
        BehaviorTree::new(guard()),
        cover(),
        target(),
    ));
}
```

In a tree, a query is a task: `task(Survey::of(Cover))` runs the guard's own
`EnvQuery<Cover>`. It succeeds if the query found something, and fails if
not. Out of ammo, the guard stops
shooting. It looks for cover, goes there, and reloads:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# struct Cover;
# #[derive(Component)] struct Ammo(u32);
# #[derive(Component, Clone)] struct TakeCover;
# #[derive(Component, Clone)] struct Reload;
# #[derive(Component, Clone)] struct Shoot;
# fn out_of_ammo(ammo: &Ammo) -> bool { ammo.0 == 0 }
fn fight() -> Behavior {
    behavior_tree! {
        Select => {
            when(out_of_ammo) => Sequence => {
                task(Survey::of(Cover)),
                task(TakeCover),
                task(Reload),
            },
            task(Shoot),
        }
    }
}
```

## Reading what it found

What a query found goes on the agent as a [`Found<Cover>`][`Found`]. A task
system reads it like any component:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# struct Cover;
# #[derive(Component, Clone)] struct TakeCover;
fn take_cover(mut tasks: TaskQuery<&TakeCover, (&Found<Cover>, &mut Transform)>) {
    tasks.for_each(|_, mut task, (cover, mut at)| {
        let Some(spot) = cover.position() else {
            task.fail();
            return;
        };
        // Walk toward the spot, and succeed once there.
        let step = (spot - at.translation).clamp_length_max(0.1);
        at.translation += step;
        if at.translation.distance(spot) < 0.1 {
            task.succeed();
        }
    });
}
```

The [Navigation](navigation.md) chapter walks the guard there with
`MoveTo::found::<Cover>()` instead of a task of its own.

`Found` has:

- `position()`: where the best item is.
- `entity()`: what the best item is, if it's a thing. The `Target` query's
  intruder.
- `best()`: the best item and its score.
- `iter()`: every item, best first. That's one item, unless the query picks
  `Pick::All`.

`Found<Cover>` is removed when a run finds nothing. It changes only when a run
finds something else. So [`found::<Cover>`][found-fn] is a typed condition: the
guard has cover in mind.

## Queries carried by things

A thing can carry queries too: those agents ask of it. A turret knows where to
stand to use it. It carries that query, measured from itself with the
[`Carrier`] anchor:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
/// Where to stand to use a turret.
struct UseSpot;

#[derive(Component)]
struct Turret;

fn use_spot() -> EnvQuery<UseSpot> {
    // Round the turret, the nearer its front the better.
    EnvQuery::new(UseSpot, Donut::around(Carrier).radius(1.0, 2.0))
        .score(Dot::of(Carrier), Score::higher())
}

fn spawn_turrets(mut commands: Commands) {
    // One query, cloned onto every turret: clones share it.
    let use_spot = use_spot();
    for x in [-10.0, 10.0] {
        commands.spawn((Turret, Transform::from_xyz(x, 0.0, 0.0), use_spot.clone()));
    }
}
```

`Carrier` is what carries the query. In the agent's own queries, it's the
agent, as `Querier` is. In the turret's, it's the turret, and `Querier` is
still the agent asking.

An agent asks a thing with `.on(..)`: `Survey::of(UseSpot).on(turret)` runs
the query `turret` carries. Or it asks whatever it found with
`.on_found::<J>()`. Here the agent finds the nearest turret, then where to
stand to use it:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# struct UseSpot;
# #[derive(Component)] struct Turret;
# #[derive(Component, Clone)] struct Man;
/// The nearest turret.
struct Turrets;

fn turrets() -> EnvQuery<Turrets> {
    EnvQuery::new(Turrets, Nearby::<Turret>::within(Querier, 20.0))
        .score(Distance::to(Querier), Score::lower())
}

fn man_a_turret() -> Behavior {
    behavior_tree! {
        Sequence => {
            task(Survey::of(Turrets)),
            task(Survey::of(UseSpot).on_found::<Turrets>()),
            task(Man),
        }
    }
}
```

The agent carries `turrets()`; each turret carries `use_spot()`. Which turret
is read from `Found<Turrets>` as the survey runs, and the task fails if
nothing is found. What it found still goes on the agent, as `Found<UseSpot>`.
It's like Unreal's EQS reading a query template from a blackboard key.

## When queries run

Queries run in [`EnvQuerySystems`], after trees tick in the same schedule. A
query a tree asks for runs that tick. The tree reads its task's result the next
tick.

All queries together get at most the [`EnvQueryBudget`] each tick. It's one
millisecond unless you set it:

```rust
# use std::time::Duration;
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# fn setup(app: &mut App) {
app.insert_resource(EnvQueryBudget(Duration::from_micros(500)));
# }
```

Queries left over wait for the next tick, oldest first. At least one runs
each tick. A hundred guards all looking for cover at once spread over a few
ticks.

## Outside a tree

A `Survey` is a task, run within the budget. A game system wanting an answer
at once runs the query itself, with [`EnvQuery::run`]: what it picks, best
first. `run_on(world, agent, carrier)` runs one a thing carries.

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# struct Cover;
/// Where each guard would take cover now, for a debug overlay.
fn cover_now(world: &mut World) -> Vec<(Entity, Vec3)> {
    let guards: Vec<(Entity, EnvQuery<Cover>)> = world
        .query::<(Entity, &EnvQuery<Cover>)>()
        .iter(world)
        .map(|(guard, query)| (guard, query.clone()))
        .collect();
    guards
        .into_iter()
        .filter_map(|(guard, query)| {
            let best = query.run(world, guard).first()?.item.position;
            Some((guard, best))
        })
        .collect()
}
```

It doesn't write `Found`: that's the survey's.

## A query of your own parts

A generator, test or anchor of your own implements a trait. A test can also
be a system, which is easier ([below](#a-test-from-a-system)). Each trait is
given a [`EnvQueryContext`]. It has:

- `world()`: the world, read-only.
- `querier()`: whose query it is.
- `carrier()`: what carries it: the agent, or the thing it was asked of.
- `space()`: 2D or 3D, as perception works in.
- `position(entity)` and `ahead(entity)`: where a thing is, and which way it
  looks.
- `locate(&anchor)`: where an anchor is. Each anchor is located once a run,
  however many items ask.
- `roll()`: a random number from 0 to 1, the query's own.

### An anchor

An anchor implements [`Locate`]. The guard has a post it shouldn't stray far
from:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
use bevy_arbor::eqs::anchor::{Locate, Located};
use bevy_arbor::eqs::query::EnvQueryContext;

/// Where a guard is posted.
#[derive(Component)]
struct GuardPost(Vec3);

/// The agent's post, as an anchor.
struct Post;

impl Locate for Post {
    fn locate(&self, cx: &EnvQueryContext, out: &mut Vec<Located>) {
        if let Some(post) = cx.world().get::<GuardPost>(cx.querier()) {
            out.push(Located { position: post.0, ahead: Vec3::ZERO, entity: None });
        }
    }
}
```

`.score(Distance::to(Post), Score::lower())` then prefers cover near the
post. An anchor can push no places, or several.

### A test from a system

Dark spots hide a guard well. The easy way to a test of your own is a
read-only system, made a test with [`Measure::by`][`Measure`]. Here it's how
far each item is from the nearest lamp:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# struct Cover;
#[derive(Component)]
struct Lamp;

/// How far each item is from the nearest lamp.
fn darkness(batch: Batch, lamps: Query<&GlobalTransform, With<Lamp>>) -> Vec<Option<f32>> {
    batch
        .items
        .iter()
        .map(|item| {
            let lamps = lamps.iter().map(|lamp| lamp.translation().distance(item.position));
            lamps.reduce(f32::min)
        })
        .collect()
}

fn dark_cover() -> EnvQuery<Cover> {
    EnvQuery::new(Cover, Donut::around(Querier).radius(2.0, 8.0).rings(3).points(16))
        .score(Measure::by(darkness), Score::higher())
}
```

- The system's first parameter is a [`Batch`]: the agent, and every item. Any
  read-only system parameters follow it.
- It answers every item at once, in order: a number, or `None` for no answer.
  So setup, like finding the lamps, happens once a run.
- An answer of the wrong length, or a system that can't run (a resource
  missing), is no answer for any item.
- It's `Cost::High` unless you say otherwise with `.costing(Cost::Medium)`.

### The `Test` trait

For full control, implement [`Test`]. The same test, as a trait:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
use bevy_arbor::eqs::query::{EnvQueryItem, EnvQueryContext};
use bevy_arbor::eqs::test::{Cost, Test};

#[derive(Component)]
struct Lamp;

/// How far an item is from the nearest lamp.
struct Darkness;

impl Test for Darkness {
    // The lamps' query, kept between runs.
    type State = QueryState<&'static GlobalTransform, With<Lamp>>;

    // Every item at once: the lamps are found once a run, not once an item.
    fn test(
        &self,
        lamps: &mut Self::State,
        cx: &EnvQueryContext,
        items: &[EnvQueryItem],
    ) -> Vec<Option<f32>> {
        let lamps: Vec<Vec3> = cx.query(lamps).iter().map(GlobalTransform::translation).collect();
        items
            .iter()
            .map(|item| lamps.iter().map(|lamp| lamp.distance(item.position)).reduce(f32::min))
            .collect()
    }

    fn cost(&self) -> Cost {
        Cost::Medium
    }
}
```

- `test` gives every item's number at once, in order, or `None` for no
  answer. Setup, as finding the lamps here, happens once a run.
- `State` is what it keeps in the world between runs: `()` for nothing, a
  `QueryState`, a `SystemState`. It's made the first time its query runs
  there, and [`EnvQueryContext::query`] reads a `QueryState`.
- `cost` is `Low`, `Medium` or `High`. Filters run cheapest first.
- `prepare(&mut State, &mut World)` runs before each run, for what needs the
  world to change: initializing a system, as a `Measure` does.

### A generator

A generator implements [`Generate`]: `generate(&self, state, cx, out)` pushes
[`EnvQueryItem`]s onto `out`, with a `State` of its own as a test has. An item is a place,
`EnvQueryItem::place(position)`, or a thing,
`EnvQueryItem::thing(entity, position)`. Tests read its `position`, and its
`entity` if it's a thing.

## Pitfalls

- **A query not carried.** A `Survey` whose carrier carries no `EnvQuery` for
  its key logs a warning and fails its task. So does `on_found` while nothing
  is found, quietly. A `Survey` without `EnvQueryPlugin` logs a warning and
  never runs.
- **Reading `Found` too soon.** A task right after the `Survey` in a
  `Sequence` is fine: the sequence waits for the survey. A system reading
  `Found` on its own may see last time's answer.
- **Stale answers.** `Found<Cover>` stays until the next run. If the intruder
  moves, ask again.

[`Carrier`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/anchor/struct.Carrier.html
[`Found`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/survey/struct.Found.html
[found-fn]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/survey/fn.found.html
[`EnvQuerySystems`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/struct.EnvQuerySystems.html
[`EnvQueryBudget`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/struct.EnvQueryBudget.html
[`EnvQueryContext`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/query/struct.EnvQueryContext.html
[`Locate`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/anchor/trait.Locate.html
[`Test`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/test/trait.Test.html
[`Measure`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/test/struct.Measure.html
[`Batch`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/test/struct.Batch.html
[`Generate`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/generate/trait.Generate.html
[`EnvQueryItem`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/query/struct.EnvQueryItem.html
[`EnvQueryContext::query`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/query/struct.EnvQueryContext.html#method.query
