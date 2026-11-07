# Sleeping trees

A tree is ticked only when something it waits on comes. Each tick records what
that is, and the tree sleeps until then:

| It waits on | Because |
|---|---|
| A component a typed guard read | The guard's answer may have changed |
| Its task's report | The task may have ended |
| A timer | A `Wait` or `Timeout` is up |
| Nothing: every tick | A system guard could matter, or a custom node polls |

A sleeping tree costs a look at the change ticks of what it watches: about 20
nanoseconds an agent, against about 150 for one asking four system guards
every tick ([Performance](performance.md)).

## What "changed" means

A watch remembers the component's change tick as the guard looked, and the tree
wakes once it moves past that, or the component comes or goes. So the tree's
own writes before the look don't wake it, but a change after it does, even one
made by a later act in the same tick.

## Keeping trees asleep

Bevy marks a component changed on any mutable access, not only on a change of
value. A system that writes a component every frame keeps every tree watching
it awake. Write with `set_if_neq`, and only what changes:

```rust
# use bevy::prelude::*;
#[derive(Component, PartialEq)]
struct Hungry(bool);

#[derive(Component)]
struct Food(f32);

fn hunger(mut agents: Query<(&Food, &mut Hungry)>) {
    for (food, mut hungry) in &mut agents {
        hungry.set_if_neq(Hungry(food.0 < 10.0));
    }
}
```

A tree whose guard reads `Hungry` wakes once, when hunger flips, not each
frame `Food` drops.
