# Sleeping

A tree only ticks when something it waits on happens. Between those moments
it sleeps, and costs almost nothing. A hundred patrolling guards cost little
until one of them sees something.

## What the patrolling guard waits on

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component, Clone)] struct Patrol { from: f32, to: f32 }
# #[derive(Component, Clone)] struct Chase;
# fn sees_intruder(seen: &Perceived<Sight>) -> bool { seen.active().any(Percept::is_hostile) }
# fn tree() -> Behavior {
behavior_tree! {
    Forever => {
        Select => {
            when(sees_intruder) => task(Chase),
            task(Patrol { from: -10.0, to: 10.0 }),
        }
    }
}
# }
```

While it patrols, this tree waits on two things:

- `Perceived<Sight>` changing, because `sees_intruder` reads it.
- The `Patrol` task reporting.

Nothing else wakes it. The patrol system moves the guard every frame, but the
tree doesn't tick.

## What wakes a tree

| It waits on | Because of |
|---|---|
| A component changing, coming or going | A typed condition that reads it |
| A task's report | A running task |
| A time | A `Wait`, a `Timeout` or a cooldown |
| Every tick | A system condition that could matter |
| Every tick | A custom node or decorator that says nothing |

A sleeping tree costs a check of the change ticks it watches. That's about 20
nanoseconds an agent, against about 150 for one asking four system conditions
every tick. See [Performance](performance.md).

## What "changed" means

The tree remembers each component's change tick when it reads it. It wakes
once the tick moves past that, or the component comes or goes. So the tree
doesn't wake for its own writes made before the read. A change after the read
does wake it, even one made later in the same tick.

## Keeping trees asleep

Bevy marks a component changed on any mutable access, even if the value
stays the same. A system that writes a component every frame keeps every tree
that reads it awake.

Write with `set_if_neq`. An armory refills the guard's ammo, but marks it
changed only when the count really changes:

```rust
# use bevy::prelude::*;
#[derive(Component, PartialEq)]
struct Ammo(u32);

#[derive(Component)]
struct AtArmory;

fn refill(mut guards: Query<&mut Ammo, With<AtArmory>>) {
    for mut ammo in &mut guards {
        ammo.set_if_neq(Ammo(6));
    }
}
```

Keep components small and specific. A typed condition reading a big
component wakes for changes to any part of it.

## The polling rule

A system condition can read anything, so the tree can't know when its answer
changes. While one could matter, its tree ticks every frame.

The same goes for custom nodes and decorators. Each says what it waits on as
it runs, with `cx.watch`, `cx.wake_at` or `cx.poll`. A running node that
says nothing is ticked again next frame. That way a mistake costs time, not a
stuck tree.

A decorator whose answer changes only when its own branch starts or ends says
so with `cx.wait_on_nothing()`. An idle cooldown does this. See
[Custom nodes](custom-nodes.md#sleeping).

## Pitfalls

- One system condition near the top of the tree keeps it awake all the time.
  Prefer typed conditions. Keep what a system condition would compute on the
  agent, with a system of your own that writes it with `set_if_neq`.
- A system writing `Transform` every frame wakes any typed condition reading
  `Transform`. Read a derived component instead.
