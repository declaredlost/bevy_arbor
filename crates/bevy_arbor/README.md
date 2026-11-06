# bevy_arbor

Reactive behaviour trees for Bevy. An agent carries its tree as a
component; each task the tree reaches is an entity of its own, ticked by
your systems, and the tree despawns it when it's done or no longer wanted.

```rust,ignore
fn has_target(In(agent): In<Entity>, targets: Query<&Target>) -> bool {
    targets.get(agent).is_ok_and(|target| target.entity.is_some())
}

let tree = behaviour_tree! {
    Forever => {
        Select => {
            when(has_target) => task("Chase", Chase),
            task("Wander", Wander::default()),
        }
    }
};
commands.entity(agent).insert(BehaviourTree::new(tree));

fn chase(mut tasks: Tasks<&Chase, (&Target, &mut Steer)>) {
    tasks.each(|_, mut task, (target, mut steer)| {
        if task.elapsed() > Duration::from_secs(10) {
            task.fail();
            return;
        }
        *steer = Steer::towards(target);
    });
}
```

`Wander` needs no idea of `Chase`: once `has_target` passes, the tree
cancels it.

## Nodes

| Node | What it does |
|---|---|
| `Forever => { .. }` | Runs its child again each time it ends |
| `Select => { .. }` | The first branch whose guard passes, re-asked every tick |
| `Fallback => { .. }` | The same, asking each guard once: what's started isn't interrupted |
| `Scored => { .. }` | Branches from the highest `score(system)` down, until one succeeds |
| `Random(roll) => { .. }` | Children in a random order, until one succeeds |
| `Sequence => { .. }` | Each child in turn, until one fails |
| `All => { .. }` | Children together; fails at the first failure, succeeds once all have |
| `Any => { .. }` | Children together; succeeds at the first success, fails once all have |
| `Race => { .. }` | Children together; the first to end settles it |
| `While(guard) => { .. }` | Its child over and over while the guard passes; succeeds when it stops |
| `Invert => { .. }` | Its child's result, reversed |
| `Succeed => { .. }` | Its child, then success whatever it did |
| `Commit => { .. }` | Its child, not to be interrupted once started |
| `Timeout(length) => { .. }` | Its child, cancelled and failed after `length` |
| `Wait(length)` | Succeeds after `length` |
| `task(name, bundle)` | Spawns a task entity with `bundle`, until it reports |
| `act(system)` | Runs a system once; what it returns is its result |
| `@ expr`, `@[ expr ]` | A node built elsewhere, or an iterator of them |

A `length` is a number of seconds (`Wait(1.5)`), checked as it compiles, or
any `Duration` (`Wait(cooldown)`). Several children under a single-child
node run as a `Sequence`. Where
something cancels its children (`All`, `Any`, `Race`), it doesn't leave them
running.

## Select

Each tick, `Select` goes through its branches in order, asking guards:

- The first whose guard passes runs, and one that was running below it is
  cancelled. So is the one running if its own guard stops passing.
- A branch without a guard always passes.
- A branch that fails falls through to the next. It stays out until its
  guard stops passing or the `Select` restarts, so it can't restart itself
  in a loop.
- A branch that succeeds ends the `Select`.

## Guards

Guards are read-only systems taking the agent, as they're asked every tick:
`when` won't take one that changes anything. `act` is for what does. Bevy
caches them by type, so they can't capture anything; what varies goes in as
a parameter.

```rust,ignore
fn within(In((agent, reach)): In<(Entity, f32)>, targets: Query<&Target>) -> bool { .. }

behaviour_tree! {
    Select => {
        when(has_target && !calm) => task("Chase", Chase),
        when(within, 5.0) => task("Bite", Bite),
        when(Guard::with(within, 9.0).or(has_target)) => task("Stalk", Stalk),
    }
}
```

`act(system, param)` takes a parameter the same way.

## Not being interrupted

A `Select` or `While` lets what's running go on if it's holding, and ticks it
knowing it's wanted to stop, until it isn't. So there are three ways not to
be cut off:

- `Fallback`: nothing in it is interrupted by its own guards.
- `Commit => { .. }`: this part isn't, once started. In a `Sequence`, what
  comes after it still can be.
- [`Graceful`] in a task's bundle: the task is told
  ([`Task::interrupted`]), winds down and reports, and only then does the
  tree move on. One that never reports holds the tree there for good, so
  put a `Timeout` around one that might not.

However it ends then, what asked it to stop is next. Only force gets past
these: a `Timeout`, or the tree going.

## Tasks

A task entity holds a [`Task`] and your bundle. Report with `succeed` or
`fail`, as many times as you like: the first counts, and the tree reads it
on its next tick. Cancelling despawns the entity, so clean up in an
`On<Remove, _>` observer if a task has anything to undo. [`Tasks`] pairs
each task with what you want of its agent.

`task.elapsed()` is how long the task has been running, by the clock the
tree ticks on, whichever schedule your task system runs in.

## Trees

- [`TreeFinished`] is triggered on the agent as its tree ends, and
  `BehaviourTree::outcome` says how. One under `Forever` never does.
  `BehaviourTree::restart` starts a finished tree again from the top.
- [`TreePaused`] on the agent holds its tree where it is: no guards asked,
  nothing started, its task not ticked by [`Tasks`]. Time goes on, so a
  `Wait` may be over when it's taken away.

## Custom nodes

The built-in nodes are written against [`Behave`] and nothing else, so
yours can do whatever they do. Implement `tick` and `cancel`, then put it in
a tree with `@ MyNode::new(..)`. Children are [`Behaviour`]s: run them with
`child.tick(cx)`; `cx` is the world and the agent. `outline` is optional,
for how it looks in a trace. See [`Behave`] for one written out.

Anything holding under a custom node keeps holding without it doing
anything. Two things are up to a node that does more than tick its child:

- On a tick it doesn't tick a running child, call `child.skip(cx)`, or the
  child's hold lapses for that tick.
- To switch away from a child that [`holds`](Behaviour::holds), tick it
  with `child.tick_interrupted(cx)` until it ends or stops holding, as
  `Select` and `While` do.

## Scheduling

`ArborPlugin` runs trees in [`ArborSystems`] (in `FixedUpdate`
unless you say otherwise). Put it after whatever guards read, and before
your task systems: a change is then acted on the tick it happens, and a new
task is ticked the tick it starts.

Ticking takes exclusive access to the world. It's one pass for each tree,
which runs until a task is under way; `Forever` starts its child at most
twice in a tick, so a tree that does nothing can't hang.

## Seeing it

`BehaviourTree` displays as an outline, with `>` on what's running. An agent
with a [`TreeTrace`] keeps that outline in it, for an inspector.
