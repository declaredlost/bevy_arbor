# Custom nodes

The built-in nodes are written against one trait, [`Behave`]. Yours can be
too, and do anything they do. The guard gets a node of its own: one that
alternates between two children, so it walks a different route each time.

## Alternate

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component, Clone)] struct Patrol { from: f32, to: f32 }
use bevy_arbor::bt::behavior::{Behave, Context};

/// Runs one child, then the other the next time, and so on.
#[derive(Inspect)]
struct Alternate {
    #[inspect(children)]
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
it in a tree with `@`. `#[derive(Inspect)]` is how it shows in a trace: see
[In a trace](#in-a-trace).

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

How a node shows in a [`BehaviorTreeTrace`] is apart from what it does: it's the
node's [`Inspect`], as `Debug` is how a type shows in `{:?}`. Every node,
decorator and condition has one.

`#[derive(Inspect)]` shows it as its type's name. Mark fields to show more:

- `#[inspect(param)]` shows the field in brackets after the name: on
  `next`, it reads `Alternate(1)`.
- `#[inspect(child)]` puts a child under it, `#[inspect(children)]` each of a
  list of them.

For anything else, write it. The [`Inspection`] it writes into takes a name,
parameters, and children, each with a note above it if you like, as `Select`
writes its decorators:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
use bevy_arbor::bt::inspect::Inspection;

# struct Alternate { children: [Behavior; 2], next: usize, running: usize }
impl Inspect for Alternate {
    fn inspect(&self, inspection: &mut Inspection<'_>) {
        inspection
            .name("Alternate")
            .param(format_args!("next: {}", self.next))
            .noted("first", &self.children[0])
            .noted("second", &self.children[1]);
    }
}
```

It's built only when the tree is traced, never as it ticks. Leaving a child
out only hides it from the trace.

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
- To switch away from a child, it calls `child.wind_down(cx)` first. While
  the child holds, that ticks it asked to stop and gives `WindDown::Holding`:
  wait, and switch on a later tick. Once it gives `Ended(outcome)` or `Free`,
  cancel the child and switch. That's what `Select` and `RepeatWhile` do.
- While `cx.is_interrupted()` is true, it starts nothing new. It goes on with
  the running child, and ends when that child does.

A node of your own can hold too: call `cx.hold()` on each tick it doesn't
want to be stopped. See [Interruption](interruption.md).

## Pitfalls

- Forgetting to cancel children in `end` leaves their tasks running.
- `start` runs each time the node starts again, as under `Forever`. Reset
  per-run state there, not in the constructor.

[`Behave`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/behavior/trait.Behave.html
[`Behavior`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/behavior/struct.Behavior.html
[`Context`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/behavior/struct.Context.html
[`Inspect`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/inspect/trait.Inspect.html
[`Inspection`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/inspect/struct.Inspection.html
[`BehaviorTreeTrace`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/tree/struct.BehaviorTreeTrace.html
