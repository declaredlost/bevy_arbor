# Running queries

The [previous chapter](queries.md) built the guard's `Cover` and `Target`
queries. This one adds them to the app, runs them from the guard's tree, and
reads what they found. It ends with generators, tests and anchors of your own.

## Adding and running a query

Add a query to the app under its key with [`add_env_query`]:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# struct Cover;
# fn cover() -> EnvQuery { EnvQuery::new(Donut::around(Agent)) }
fn setup(app: &mut App) {
    app.add_env_query(Cover, cover());
}
```

In a tree, a query is a task: `task(Survey::of(Cover))`. It succeeds if the
query found something, and fails if not. Out of ammo, the guard stops
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
fn take_cover(mut tasks: Tasks<&TakeCover, (&Found<Cover>, &mut Transform)>) {
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

## When queries run

Queries run in [`EqsSystems`], after trees tick in the same schedule. A query
a tree asks for runs that tick. The tree reads its task's result the next
tick.

All queries together get at most the [`EqsBudget`] each tick. It's one
millisecond unless you set it:

```rust
# use std::time::Duration;
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# fn setup(app: &mut App) {
app.insert_resource(EqsBudget(Duration::from_micros(500)));
# }
```

Queries left over wait for the next tick, oldest first. At least one runs
each tick. A hundred guards all looking for cover at once spread over a few
ticks.

## Outside a tree

Insert a `Survey::of(Cover)` on an agent to run the query for it once. It's
taken off when it has run, and `Found<Cover>` is written as usual. This suits
a game system that wants an answer without a tree.

## A query of your own parts

A generator, test or anchor of your own implements a trait. A test can also
be a system, which is easier ([below](#a-test-from-a-system)). Each trait is
given a [`QueryContext`]. It has:

- `world()`: the world, read-only.
- `agent()`: whose query it is.
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
use bevy_arbor::eqs::query::QueryContext;

/// Where a guard is posted.
#[derive(Component)]
struct GuardPost(Vec3);

/// The agent's post, as an anchor.
struct Post;

impl Locate for Post {
    fn locate(&self, cx: &QueryContext, out: &mut Vec<Located>) {
        if let Some(post) = cx.world().get::<GuardPost>(cx.agent()) {
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
#[derive(Component)]
struct Lamp;

/// How far each item is from the nearest lamp.
fn darkness(batch: Batch, lamps: Query<&GlobalTransform, With<Lamp>>) -> Vec<Option<f32>> {
    batch
        .items
        .iter()
        .map(|item| {
            let lamps = lamps.iter().map(|lamp| lamp.translation().distance(item.position()));
            lamps.reduce(f32::min)
        })
        .collect()
}

fn dark_cover() -> EnvQuery {
    EnvQuery::new(Donut::around(Agent).radius(2.0, 8.0).rings(3).points(16))
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
use bevy_arbor::eqs::query::{Item, QueryContext};
use bevy_arbor::eqs::test::{Cost, Test};

#[derive(Component)]
struct Lamp;

/// How far an item is from the nearest lamp.
struct Darkness;

impl Test for Darkness {
    fn test(&self, cx: &QueryContext, item: &Item) -> Option<f32> {
        self.test_all(cx, std::slice::from_ref(item)).pop().flatten()
    }

    // Find the lamps once a run, not once an item.
    fn test_all(&self, cx: &QueryContext, items: &[Item]) -> Vec<Option<f32>> {
        let world = cx.world();
        let lamps: Vec<Vec3> = world
            .try_query_filtered::<&GlobalTransform, With<Lamp>>()
            .map(|mut lamps| lamps.iter(world).map(GlobalTransform::translation).collect())
            .unwrap_or_default();
        items
            .iter()
            .map(|item| lamps.iter().map(|lamp| lamp.distance(item.position())).reduce(f32::min))
            .collect()
    }

    fn cost(&self) -> Cost {
        Cost::Medium
    }
}
```

- `test` gives one item's number, or `None` for no answer.
- `test_all` gives every item's at once. Override it when a test has setup to
  do once a run, as here.
- `cost` is `Low`, `Medium` or `High`. Filters run cheapest first.
- `prepare(&mut World)` runs before each query the test is in, for setup that
  needs to change the world.

### A generator

A generator implements [`Generate`]: `generate(&self, cx, out)` pushes
[`Item`]s onto `out`. An item is a place, `Item::Point(position)`, or a thing,
`Item::Entity { entity, position }`.

## Pitfalls

- **A key never added.** A `Survey` for a key with no `add_env_query` logs a
  warning and fails its task.
- **Reading `Found` too soon.** A task right after the `Survey` in a
  `Sequence` is fine: the sequence waits for the survey. A system reading
  `Found` on its own may see last time's answer.
- **Stale answers.** `Found<Cover>` stays until the next run. If the intruder
  moves, ask again.

[`add_env_query`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/survey/trait.EnvQueryApp.html
[`Found`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/survey/struct.Found.html
[found-fn]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/survey/fn.found.html
[`EqsSystems`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/struct.EqsSystems.html
[`EqsBudget`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/struct.EqsBudget.html
[`QueryContext`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/query/struct.QueryContext.html
[`Locate`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/anchor/trait.Locate.html
[`Test`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/test/trait.Test.html
[`Measure`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/test/struct.Measure.html
[`Batch`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/test/struct.Batch.html
[`Generate`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/generate/trait.Generate.html
[`Item`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/query/enum.Item.html
