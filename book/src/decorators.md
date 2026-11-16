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
  returns `impl Condition`, built with `.into_condition()`:
  `(move |ammo: &Ammo| ammo.0 >= least).into_condition()`.
- It reads only the agent. For anything else, use a system condition, or keep
  what you need on the agent with a system of your own.

## System conditions

A system condition is a read-only system that takes the agent. It can read
anything. The guard shoots only when the intruder is in range:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component, Clone)] struct Chase;
# #[derive(Component, Clone)] struct Shoot;
fn within(
    In((guard, range)): In<(Entity, f32)>,
    guards: Query<(&Perceived<Sight>, &GlobalTransform)>,
    places: Query<&GlobalTransform>,
) -> bool {
    let Ok((seen, at)) = guards.get(guard) else {
        return false;
    };
    seen.active().filter(|p| p.is_hostile()).any(|p| {
        places
            .get(p.entity())
            .is_ok_and(|there| there.translation().distance(at.translation()) <= range)
    })
}

# fn tree() -> Behavior {
behavior_tree! {
    Select => {
        when(with(within, 6.0)) => task(Shoot),
        task(Chase),
    }
}
# }
```

- A parameter goes in with `with`: `when(with(within, 6.0))`. The system gets
  it in `In` with the agent.
- A system condition is asked every tick, because it could read anything.
  While one could matter, the tree doesn't sleep.
- It can't take `ResMut` or a `&mut` query, as with a Bevy run condition.
  It can queue `Commands`. Use `act` to change things.
- It can return `Result<bool>`, so you can use `?`. An error doesn't pass,
  and goes to Bevy's error handler.
- Each system is one instance, shared by every tree and agent. Its `Local`s
  are shared too, so keep per-agent state on the agent.
- For the same reason, a `Changed` filter in it sees what changed since it
  was last asked about any agent, not this one.
- `act(with(system, param))` and `score(with(system, param))` take a parameter
  the same way.

## Conditions of your own

A closure taking the [`Context`] is a condition too, for one that needs no
system and no type of its own: `when(|cx: &mut Context| ..)`. It reads the
world through `cx.world()`. A type of your own implements [`Condition`].
Either way, say what it waits on, with `cx.watch::<T>(entity)` or
`cx.wake_at(time)`, and the tree sleeps until then. One that says nothing is
asked every tick.

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

`with(within, 6.0) && sees_intruder` combines like any other. Outside the
macro, conditions combine with [`IntoCondition`]'s `and` and `or`, and `not`.
They aren't in the prelude, since Bevy's run conditions have those names.

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

`shared_cooldown::<K>(seconds)` shares one cooldown between every branch keyed
by `K` on the agent. Running one cools them all. The cooldown is the
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
        when(sees_intruder, shared_cooldown::<Alarm>(10.0)) => task(Shout),
    }
}
# }
fn alarm_heard(time: Res<Time>, guards: Query<Entity, With<Guard>>, mut commands: Commands) {
    let over = time.elapsed() + Duration::from_secs(10);
    for guard in &guards {
        commands.entity(guard).insert(Cooling::<Alarm>::new(over));
    }
}
```

## Decorators of your own

A decorator that acts as its branch starts and ends, as a cooldown does,
implements [`Decorate`]. `passes` answers. `start` and `end` hear its branch
start and end. It goes after a comma, as a cooldown does, so whenever its
branch runs, it passed, and it hears both.

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
#[derive(Inspect)]
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
        if let Some(mut chasers) = cx.world_mut().get_mut::<Chasers>(self.pool) {
            chasers.0 += 1;
        }
    }

    fn end(&mut self, cx: &mut Context, _outcome: Option<Outcome>) {
        if let Some(mut chasers) = cx.world_mut().get_mut::<Chasers>(self.pool) {
            chasers.0 -= 1;
        }
    }
}
```

Then `when(sees_intruder, Limit { max: 2, pool })` lets two guards in.

Say what the decorator waits on each time it's asked:

- `cx.watch::<T>(entity)` to be asked again when a component changes.
- `cx.wake_at(time)` to be asked again at a time.
- `cx.wait_on_nothing()` when only its own branch starting or ending changes
  its answer.

A decorator that says nothing is asked every tick.

## Pitfalls

- A component written every frame wakes every typed condition reading it.
  Write with `set_if_neq`. See [Sleeping](sleeping.md).
- A system condition can't capture variables. Pass what varies as its
  parameter, with `with`: `when(with(within, 6.0))`.

[`Cooling<K>`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/decorator/struct.Cooling.html
[`Decorate`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/decorator/trait.Decorate.html
[`Condition`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/condition/trait.Condition.html
[`IntoCondition`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/condition/trait.IntoCondition.html
[`Context`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/behavior/struct.Context.html
