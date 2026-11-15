# Decorators

A decorator sits on a branch, `when(..) => ..`, and decides whether it may
run. The guard uses them to reload when it's out of ammo, to shoot only in
range, and to call for help no more than once in a while.

## Reloading

A plain function of the guard's components is a decorator:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component, Clone)] struct Patrol { from: f32, to: f32 }
# #[derive(Component, Clone)] struct Chase;
# #[derive(Component, Clone)] struct Shoot;
# #[derive(Component, Clone)] struct Reload;
# fn sees_intruder(seen: &Perceived<Sight>) -> bool { seen.active().any(Percept::is_hostile) }
#[derive(Component, PartialEq)]
struct Ammo(u32);

fn out_of_ammo(ammo: &Ammo) -> bool {
    ammo.0 == 0
}

# fn tree() -> Behavior {
behavior_tree! {
    Forever => {
        Select => {
            when(out_of_ammo) => task(Reload),
            when(sees_intruder) => Sequence => { task(Chase), task(Shoot) },
            task(Patrol { from: -10.0, to: 10.0 }),
        }
    }
}
# }
```

When the last shot empties `Ammo`, the `Select` cancels the fight and the
guard reloads.

## Typed conditions

`out_of_ammo` is a typed condition. It takes `&T` or `Option<&T>` of up to
four of the agent's components, and returns `bool`.

- It's asked again only when one of those components changes, comes or goes.
  Until then, the tree can sleep. See [Sleeping](sleeping.md).
- If the agent lacks a component it takes as `&T`, it doesn't pass.
- A closure is one too, so it can capture what varies. A function making one
  returns `impl Condition`:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
use bevy_arbor::bt::decorator::{Condition, IntoCondition};

# #[derive(Component)] struct Ammo(u32);
/// At least `least` rounds left.
fn ammo(least: u32) -> impl Condition {
    (move |ammo: &Ammo| ammo.0 >= least).into_condition()
}
# #[derive(Component, Clone)] struct Shoot;
# fn tree() -> Behavior {
# behavior_tree! { Select => { when(ammo(3)) => task(Shoot) } }
# }
```

## Conditions of your own

A typed condition reads only the agent. For anything else, implement
[`Condition`]: it reads the whole world through `cx.world()`. The guard
shoots only when the intruder is in range:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component, Clone)] struct Chase;
# #[derive(Component, Clone)] struct Shoot;
use bevy_arbor::bt::behavior::Context;
use bevy_arbor::bt::decorator::Condition;

/// An intruder within this far of the guard.
struct Within(f32);

impl Condition for Within {
    fn passes(&mut self, cx: &mut Context) -> bool {
        let world = cx.world();
        let guard = cx.agent();
        let (Some(seen), Some(at)) = (
            world.get::<Perceived<Sight>>(guard),
            world.get::<GlobalTransform>(guard),
        ) else {
            return false;
        };
        seen.active().filter(|p| p.is_hostile()).any(|p| {
            world
                .get::<GlobalTransform>(p.entity())
                .is_some_and(|there| there.translation().distance(at.translation()) <= self.0)
        })
    }
}

# fn tree() -> Behavior {
behavior_tree! {
    Select => {
        when(Within(6.0)) => task(Shoot),
        task(Chase),
    }
}
# }
```

- It's asked as the tree ticks, side by side with other trees, so it only
  reads: what changes things is a task, or an `act`.
- Say what it waits on, with `cx.watch::<T>(entity)` or `cx.wake_at(time)`,
  and the tree sleeps until then. One that says nothing, as `Within`, which
  reads positions that move, is asked every tick: while it could matter, the
  tree doesn't sleep.
- Often better: a system of your own puts what it would read on the agent, as
  a component, say `InRange`, written with `set_if_neq`. A typed condition
  reads that, and the tree sleeps until it changes. Perception works this way.

## Combining

Conditions combine with `&&`, `||` and `!`:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component, Clone)] struct Shoot;
# #[derive(Component)] struct Ammo(u32);
# fn sees_intruder(seen: &Perceived<Sight>) -> bool { seen.active().any(Percept::is_hostile) }
# fn out_of_ammo(ammo: &Ammo) -> bool { ammo.0 == 0 }
# fn tree() -> Behavior {
behavior_tree! {
    Select => {
        when(sees_intruder && !out_of_ammo) => task(Shoot),
    }
}
# }
```

Outside the macro, they combine with [`IntoCondition`]'s `and` and `or`, and
`not`: `sees_intruder.and(not(out_of_ammo))`. They aren't in the prelude,
since Bevy's run conditions have those names.

## Cooldowns

`cooldown(seconds)` keeps its branch from running again until that long after
it last ended. The guard calls for help at most every ten seconds:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component, Clone)] struct Chase;
# #[derive(Component, Clone)] struct Shout;
# fn sees_intruder(seen: &Perceived<Sight>) -> bool { seen.active().any(Percept::is_hostile) }
# fn tree() -> Behavior {
behavior_tree! {
    Forever => {
        Select => {
            when(sees_intruder, cooldown(10.0)) => task(Shout),
            when(sees_intruder) => task(Chase),
        }
    }
}
# }
```

- It goes after a comma: every part of `when(..)` must pass. A cooldown acts
  as its branch ends, so it can't go under `!` or `||`, where it might not
  have been asked, or have passed, when the branch started. That doesn't
  compile.
- It cools however the branch ended: success, failure or cancellation.
- The tree sleeps while it cools, and wakes when it's done.
- Each agent cools on its own.

### Shared cooldowns

`cooldown_of::<K>(seconds)` shares one cooldown between every branch keyed by
`K` on the agent. Running one cools them all. The cooldown is the
[`Cooling<K>`] component on the agent. Insert one yourself to start it from
elsewhere. Here, a guard that hears another's alarm doesn't raise its own:

```rust
# use std::time::Duration;
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component, Clone)] struct Shout;
# #[derive(Component)] struct Guard;
# fn sees_intruder(seen: &Perceived<Sight>) -> bool { seen.active().any(Percept::is_hostile) }
/// What every alarm shares.
struct Alarm;

# fn tree() -> Behavior {
behavior_tree! {
    Select => {
        when(sees_intruder, cooldown_of::<Alarm>(10.0)) => task(Shout),
    }
}
# }
fn alarm_heard(time: Res<Time>, guards: Query<Entity, With<Guard>>, mut commands: Commands) {
    let over = time.elapsed() + Duration::from_secs(10);
    for guard in &guards {
        commands.entity(guard).insert(Cooling::<Alarm>::until(over));
    }
}
```

## Decorators of your own

A decorator that acts as its branch starts and ends, as a cooldown does,
implements [`Decorate`]. `passes` answers. `start` and `end` hear its branch
start and end. It goes after a comma, as a cooldown does.

At most two guards may chase at once. They count chasers on a shared entity:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
use bevy_arbor::bt::behavior::Context;
use bevy_arbor::bt::decorator::Decorate;

/// How many guards are chasing.
#[derive(Component)]
struct Chasers(u32);

/// At most `max` agents in its branch, counted on `pool`.
struct Limit {
    max: u32,
    pool: Entity,
}

impl Decorate for Limit {
    fn passes(&mut self, cx: &mut Context) -> bool {
        // Ask again when the count changes.
        cx.watch::<Chasers>(self.pool);
        cx.world()
            .get::<Chasers>(self.pool)
            .is_some_and(|chasers| chasers.0 < self.max)
    }

    fn start(&mut self, cx: &mut Context) {
        cx.commands()
            .entity(self.pool)
            .entry::<Chasers>()
            .and_modify(|mut chasers| chasers.0 += 1);
    }

    fn end(&mut self, cx: &mut Context, _outcome: Option<Outcome>) {
        cx.commands()
            .entity(self.pool)
            .entry::<Chasers>()
            .and_modify(|mut chasers| chasers.0 -= 1);
    }
}
```

Then `when(sees_intruder, Limit { max: 2, pool })` lets two guards in.

- It reads the world, and changes it with `cx.commands()`. They're applied
  once its tree's tick is done, after the trees ticked beside it, so two
  guards ticked together may both see one chaser and both start. Where that
  matters, check again in the command, as [`claim`] does.
- Say what it waits on each time it's asked: `cx.watch::<T>(entity)`,
  `cx.wake_at(time)`, or `cx.wait_on_nothing()` when only its own branch
  starting or ending changes its answer. One that says nothing is asked every
  tick.

## Pitfalls

- A component written every frame wakes every typed condition reading it.
  Write with `set_if_neq`. See [Sleeping](sleeping.md).
- A condition only reads. To change something as a branch runs, use a task,
  `State`, or a [`Decorate`].

[`Cooling<K>`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/decorator/struct.Cooling.html
[`Condition`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/decorator/trait.Condition.html
[`IntoCondition`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/decorator/trait.IntoCondition.html
[`Decorate`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/decorator/trait.Decorate.html
[`claim`]: https://docs.rs/bevy_arbor_smart/latest/bevy_arbor_smart/claim/fn.claim.html
