# Custom nodes

The built-in nodes are written against one trait, [`Behave`]. Yours can be
too, and do anything they do. The guard gets a node of its own: one that
alternates between two children, so it walks a different route each time.

## Alternate

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component, Clone)] struct Patrol { from: f32, to: f32 }
use bevy_arbor::bt::behavior::{Behave, Child, Context};

/// Runs one child, then the other the next time, and so on.
struct Alternate {
    children: [Behavior; 2],
    next: usize,
    running: usize,
}

impl Behave for Alternate {
    fn start(&mut self, _cx: &mut Context) {
        self.running = self.next;
        self.next = 1 - self.next;
    }

    fn tick(&mut self, cx: &mut Context) -> Status {
        self.children[self.running].tick(cx)
    }

    fn end(&mut self, cx: &mut Context, _outcome: Option<Outcome>) {
        self.children[self.running].cancel(cx);
    }

    fn children(&self) -> Vec<Child<'_>> {
        self.children.iter().map(Child::new).collect()
    }
}

fn alternate(first: Behavior, second: Behavior) -> Behavior {
    Alternate { children: [first, second], next: 0, running: 0 }.into()
}

fn patrol() -> Behavior {
    behavior_tree! {
        Forever => {
            @ alternate(
                behavior_tree! { task(Patrol { from: -10.0, to: 10.0 }) },
                behavior_tree! { task(Patrol { from: 20.0, to: 30.0 }) },
            )
        }
    }
}
```

Any type implementing `Behave` turns into a [`Behavior`] with `.into()`. Put
it in a tree with `@`.

## The lifecycle

The tree calls three methods. Your node only says what happens at each:

- `start` runs before the first tick, each time the node runs. It's optional.
- `tick` runs the node for a tick. Return `Status::Running` until it's done,
  then `Status::Success` or `Status::Failure`.
- `end` runs once, when the node finishes or is canceled. `outcome` is how
  it finished, or `None` if it was canceled.

Children are `Behavior`s:

- `child.tick(cx)` runs one. It starts it if it isn't running, and ends it
  when it's done.
- `child.cancel(cx)` stops one. A child that isn't running is left as it is,
  so it's safe to call on every child.

`end` must cancel the node's children. If it doesn't, their tasks keep
running after the node is gone.

`cx` is the [`Context`]: `cx.agent()` is the agent, `cx.world()` and
`cx.world_mut()` the world, `cx.now()` the tree's clock.

## In a trace

In a [`TreeTrace`], a node shows as its type's name with nothing under it.
Change that with two optional methods:

- `label` is how it reads. `Alternate` keeps its type's name, so it doesn't
  need one.
- `children` lists what's under it. `Child::noted(child, "..")` writes a
  note above a child, as `Select` writes its decorators.

Both are built only when the tree is traced. Leaving a child out of
`children` only hides it from the trace.

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# use bevy_arbor::bt::behavior::{Behave, Child, Context};
# struct Alternate { children: [Behavior; 2], next: usize, running: usize }
# impl Behave for Alternate {
# fn tick(&mut self, _: &mut Context) -> Status { Status::Running }
# fn end(&mut self, cx: &mut Context, _: Option<Outcome>) { self.children[self.running].cancel(cx); }
fn label(&self) -> String {
    format!("Alternate (next: {})", self.next)
}

fn children(&self) -> Vec<Child<'_>> {
    vec![
        Child::noted(&self.children[0], "first"),
        Child::noted(&self.children[1], "second"),
    ]
}
# }
```

## Sleeping

Trees sleep until something they wait on happens. See [Sleeping](sleeping.md).
A node says what it waits on as it ticks:

- `cx.wake_at(time)` to tick again at a time.
- `cx.watch::<T>(entity)` to tick again when a component changes.
- `cx.poll()` to tick again next frame.

Children say what they wait on themselves. `Alternate` says nothing of its
own: its patrol task's report wakes the tree. A running node that says
nothing, children included, is polled. See
[the polling rule](sleeping.md#the-polling-rule).

## Holding and interruption

A node that just ticks its children, as `Alternate` does, needs nothing more.
Holding below it passes up on its own. A node that does more follows three
rules:

- If it skips a tick of a running child, it calls `child.skip(cx)`.
  Otherwise the child's hold lapses for that tick.
- To switch away from a child that holds, it ticks the child with
  `child.tick_interrupted(cx)` until the child ends or stops holding. That's
  what `Select` and `RepeatWhile` do.
- While `cx.interrupted()` is true, it starts nothing new. It goes on with the
  running child, and ends when that child does.

A node of your own can hold too: call `cx.hold()` on each tick it doesn't
want to be stopped. See [Interruption](interruption.md).

## Pitfalls

- Forgetting to cancel children in `end` leaves their tasks running.
- `start` runs each time the node starts again, as under `Forever`. Reset
  per-run state there, not in the constructor.

[`Behave`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/behavior/trait.Behave.html
[`Behavior`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/behavior/struct.Behavior.html
[`Context`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/behavior/struct.Context.html
[`TreeTrace`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/tree/struct.TreeTrace.html
