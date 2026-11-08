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
        when(within, 6.0) => task(Shoot),
        task(Chase),
    }
}
# }
```

- A parameter goes after the system: `when(within, 6.0)`. The system gets it
  in `In` with the agent.
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
- `act(system, param)` takes a parameter the same way.

## Combining

Decorators combine with `&&`, `||` and `!`:

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

A system condition with a parameter can't go straight into `&&`. Build it
with `Decorator::with`, then combine with `.and(..)` and `.or(..)`:
`when(Decorator::with(within, 6.0).and(sees_intruder))`.

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
            when(sees_intruder && cooldown(10.0)) => task(Shout),
            when(sees_intruder) => task(Chase),
        }
    }
}
# }
```

- It cools however the branch ended: success, failure or cancellation.
- The tree sleeps while it cools, and wakes when it's done.
- Each agent cools on its own.
- It's told when its branch ends, even when `&&` or `||` skipped asking it.

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
        when(sees_intruder && cooldown_of::<Alarm>(10.0)) => task(Shout),
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

A decorator of your own implements [`Decorate`]. `passes` answers. `start`
and `end` hear its branch start and end, if it keeps anything.

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

Then `when(sees_intruder && Limit { max: 2, pool })` lets two guards in.

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
  parameter.

[`Cooling<K>`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/decorator/struct.Cooling.html
[`Decorate`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/decorator/trait.Decorate.html
