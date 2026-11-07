# Custom nodes

The built-in nodes are written against [`Behave`] and nothing else, so yours
can do whatever they do. Implement `tick` and `cancel`, and put it in a tree
with `@`:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
use bevy_arbor::bt::behaviour::{Behave, Context};

/// Runs its child `times` times, unless it fails.
struct Repeat {
    times: u32,
    done: u32,
    child: Behaviour,
}

impl Behave for Repeat {
    fn tick(&mut self, cx: &mut Context) -> Status {
        match self.child.tick(cx) {
            Status::Success => {
                self.done += 1;
                if self.done < self.times {
                    return Status::Running;
                }
                self.done = 0;
                Status::Success
            }
            Status::Failure => {
                self.done = 0;
                Status::Failure
            }
            Status::Running => Status::Running,
        }
    }

    fn cancel(&mut self, cx: &mut Context) {
        self.done = 0;
        self.child.cancel(cx);
    }
}

# fn tree() -> Behaviour {
behaviour_tree! {
    @ Repeat { times: 3, done: 0, child: behaviour_tree! { Wait(1.0) } }
}
# }
```

Children are `Behaviour`s: run them with `child.tick(cx)`. `cx` is the world
and the agent.

## Sleeping

Trees sleep, but a custom node is ticked every tick while it runs, unless its
`polls` says no. Then, when it returns `Running`, it says what it waits on:
`cx.wake_at(time)` for a timer, `cx.watch::<T>(entity)` for a component. What
its children wait on, they say themselves. `Repeat` above only ticks its
child, so it can say no:

```rust
# use bevy_arbor::bt::behaviour::{Behave, Context};
# use bevy_arbor::prelude::*;
# struct Repeat;
# impl Behave for Repeat {
# fn tick(&mut self, _: &mut Context) -> Status { Status::Running }
# fn cancel(&mut self, _: &mut Context) {}
fn polls(&self) -> bool {
    false
}
# }
```

## Holding and interruption

Anything holding under a custom node keeps holding without it doing anything.
What's up to a node that does more than tick its child:

- On a tick it doesn't tick a running child, call `child.skip(cx)`, or the
  child's hold lapses for that tick.
- To switch away from a child that holds, tick it with
  `child.tick_interrupted(cx)` until it ends or stops holding, as `Select`
  and `While` do.
- While `cx.interrupted()`, start nothing new: go on with the child that's
  running, and end once it does.

[`Behave`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/behaviour/trait.Behave.html
