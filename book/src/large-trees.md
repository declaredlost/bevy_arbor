# Large trees and sockets

The guard's tree grows with every chapter. One `behavior_tree!` holding all of
it gets hard to read. Three habits keep it readable: split it into subtrees,
leave timing to the tasks, and plug in behavior that varies with sockets.

## Subtrees

A subtree is a function returning a [`Behavior`]. Put it in with `@`. The top
of the guard's tree becomes a list of priorities, one per line:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component, Clone)] struct Patrol { from: f32, to: f32 }
# #[derive(Component, Clone)] struct Chase;
# #[derive(Component, Clone)] struct Search;
# #[derive(Component, Clone)] struct Shoot;
# #[derive(Component, Clone)] struct Reload;
# #[derive(Component)] struct Ammo(u32);
# fn sees_intruder(seen: &Perceived<Sight>) -> bool { seen.active().any(Percept::is_hostile) }
# fn remembers_intruder(seen: &Perceived<Sight>) -> bool { seen.iter().any(|p| p.is_hostile() && !p.is_active()) }
# fn out_of_ammo(ammo: &Ammo) -> bool { ammo.0 == 0 }
fn guard() -> Behavior {
    behavior_tree! {
        Forever => {
            Select => {
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
            when(out_of_ammo) => task((Reload, Graceful)),
            Sequence => {
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
```

- `@` costs nothing. It puts the subtree's nodes straight into the tree, so
  the tree runs and traces as if written in one piece.
- A subtree can take arguments: `@ patrol(-10.0, 10.0)`.
- Several kinds of agent can share a subtree.
- `@[ iter ]` puts in every node of an iterator, for a list of children built
  in code.

## Timing in tasks

How long a search lasts is the search's business. Keep it in the task's
system, not in the tree:

```rust
# use std::time::Duration;
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
#[derive(Component, Clone)]
struct Search;

const GIVE_UP: Duration = Duration::from_secs(8);

fn search(mut tasks: Tasks<&Search, &mut Transform>) {
    tasks.for_each(|_, mut task, mut at| {
        if task.elapsed() > GIVE_UP {
            task.fail();
            return;
        }
        // Walk toward where the intruder was last seen.
#       let _ = &mut at;
    });
}
```

The tree then says what to do and when, not for how long. Keep `Timeout` for
a part of the tree whose pieces don't know about each other, such as a whole
`Sequence` that must finish in time.

## Sockets

A subtree put in with `@` is fixed when the tree is built. A socket is filled
at runtime, from the agent. The tree says where with `Socket(key)`. A
[`Plugged<S>`] on the agent says what runs there.

Some guards carry a crossbow and keep their distance. Others carry a sword and
close in. The tree doesn't need to know which:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component, Clone)] struct Chase;
# #[derive(Component, Clone)] struct Shoot;
# #[derive(Component, Clone)] struct Slash;
# fn sees_intruder(seen: &Perceived<Sight>) -> bool { seen.active().any(Percept::is_hostile) }
/// The socket weapons plug into.
struct Weapon;

fn fight() -> Behavior {
    behavior_tree! {
        Select => {
            when(plugged::<Weapon>) => Socket(Weapon),
            task(Chase),
        }
    }
}

fn crossbow() -> Behavior {
    behavior_tree! { Sequence => { task(Shoot), Wait(1.5) } }
}

fn sword() -> Behavior {
    behavior_tree! { Sequence => { task(Chase), task(Slash) } }
}

fn arm(commands: &mut Commands, guard: Entity, archer: bool) {
    let weapon = if archer {
        Plugged::new(Weapon, crossbow)
    } else {
        Plugged::new(Weapon, sword)
    };
    commands.entity(guard).insert(weapon);
}
```

- Inserting another `Plugged<Weapon>` swaps it. The socket cancels what it
  ran, once that stops holding, and starts the new one.
- Removing it unplugs the socket. A socket with nothing plugged in fails.
- `plugged::<Weapon>` is a decorator that passes while something is plugged
  in. It keeps an empty socket out of the way, and wakes the tree when
  something is plugged in.
- Each agent builds its own copy of the plugged subtree.

[Smart objects](smart-objects.md) use a socket too: a slot plugs in how to
use it.

## Pitfalls

- Reach for a socket when the *decisions* differ: a crossbow keeps its
  distance, a sword closes in. When only how a task is done differs, one task
  whose system reads the weapon is simpler.
- A `Timeout` around every task is noise. Put the time limit in the task.

[`Behavior`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/behavior/struct.Behavior.html
[`Plugged<S>`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/behavior/struct.Plugged.html
