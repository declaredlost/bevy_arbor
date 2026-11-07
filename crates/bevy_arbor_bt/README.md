# bevy_arbor_bt

Reactive behaviour trees for Bevy. An agent carries its tree as a
component; each task the tree reaches is an entity of its own, ticked by
your systems, and the tree despawns it when it's done or no longer wanted.
Between ticks a tree sleeps, until something it waits on changes.

```rust,ignore
fn has_target(target: &Target) -> bool {
    target.entity.is_some()
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
    tasks.for_each(|_, mut task, (target, mut steer)| {
        if task.elapsed() > Duration::from_secs(10) {
            task.fail();
            return;
        }
        *steer = Steer::towards(target);
    });
}
```

`Wander` needs no idea of `Chase`: once the agent's `Target` changes so that
`has_target` passes, the tree wakes and cancels it.

At the root: [`TreePlugin`], [`TreeSystems`] and the [`behaviour_tree!`]
macro. The rest is in three modules, and [`prelude`]
brings in what using trees takes:

| Module | What's in it |
|---|---|
| [`tree`] | The tree on an agent: `BehaviourTree`, `TreeFinished`, `TreePaused`, `TreeTrace` |
| [`behaviour`] | What trees are built of: `Behaviour`, `Branch`, `Guard`; and for a node of your own, `Behave`, `Context` and `Status` |
| [`task`] | What trees spawn: `Task`, `TaskOf`, `AgentTasks`, `TaskEnded`, `Graceful`, `Tasks` |

## Nodes

| Node | What it does |
|---|---|
| `Forever => { .. }` | Runs its child again each time it ends |
| `Select => { .. }` | The first branch whose guard passes, asked again as what it reads changes |
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
| `State(bundle) => { .. }` | Its child, with `bundle` on the agent while it runs |
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

Each time its tree wakes, `Select` goes through its branches in order,
asking guards:

- The first whose guard passes runs, and one that was running below it is
  cancelled. So is the one running if its own guard stops passing.
- A branch without a guard always passes.
- A branch that fails falls through to the next. It stays out until its
  guard stops passing or the `Select` restarts, so it can't restart itself
  in a loop.
- A branch that succeeds ends the `Select`.

## Guards

A guard is one of two things, and its signature says which.

A typed guard is a function of the agent's components: it takes `&T` or
`Option<&T>` of up to four, and returns `bool`. It's asked again only once
one of them changes, comes or goes on the agent, so a tree under typed
guards sleeps until then, and costs next to nothing meanwhile. One taking a
`&T` the agent hasn't got doesn't pass.

```rust,ignore
fn has_target(target: &Target) -> bool {
    target.entity.is_some()
}

fn badly_hurt(health: &Health, armour: Option<&Armour>) -> bool {
    health.0 < 20.0 && armour.is_none()
}
```

A typed guard reads only the agent. For anything else, keep it on the agent
with a system of your own, run before [`TreeSystems`]: a perception or
targeting system writing `Target`, say. Write with `set_if_neq`, so only a
real change wakes trees. Such a system can work only while a branch runs:
filter on its task (`With<Chase>` on the task entity), or on a marker that
`State(Hunting) => { .. }` puts on the agent while its branch runs.

A system guard is a read-only system taking the agent, for what can't be
kept on it. It can read anything, so it's asked every tick, and its tree
doesn't sleep while it could matter. Read-only as a Bevy run condition is:
`when` won't take a `ResMut` or a `&mut` query, though, like a run
condition, it can queue `Commands`, applied as it returns. `act` is for what
changes things. Either can return a `Result<bool>`, to use `?`: an error
doesn't pass (or fails the `act`), and goes to Bevy's error handler, as a
system's would, as does one that can't run. Scores and rolls can return
`Result<f32>` the same way; a NaN score goes last.

Systems are kept by type, so they can't capture anything; what varies goes
in as a parameter. Every node, tree and agent using a system shares the one,
`Local`s and change detection too: a `Local` sees what other agents left in
it, and a `Changed` filter sees what changed since it was last asked about
any agent. Keep what's per agent on the agent.

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

Guards of both kinds combine with `&&`, `||` and `!`. `act(system, param)`
takes a parameter as `when(system, param)` does.

## Not being interrupted

A `Select` or `While` lets what's running go on if it's holding, and ticks it
knowing it's wanted to stop, until it isn't. So there are three ways not to
be cut off:

- `Fallback`: nothing in it is interrupted by its own guards.
- `Commit => { .. }`: this part isn't, once started; not even told, so a
  `Graceful` task in it doesn't wind down. In a `Sequence`, what comes after
  it still can be, and isn't started once it's wanted to stop.
- [`Graceful`] in a task's bundle: the task is told
  ([`Task::interrupted`]), winds down and reports, and only then does the
  tree move on. One that never reports holds the tree there for good, so
  put a `Timeout` around one that might not.

Meanwhile nothing new starts in what's winding down: a `Sequence` doesn't go
on to its next child, nor `Forever` start its own again. However it ends
then, what asked it to stop is next. A task told to stop is told it isn't
any more, if what asked stops asking first.

Only force gets past these: a `Timeout`, an `All`, `Any` or `Race` settling
(cancelling the rest), anything above being cancelled, or the tree going.

## Tasks

A task entity holds a [`Task`], your bundle, and a [`TaskOf`]: a Bevy
relationship to its agent, whose [`AgentTasks`] list the tasks it's running.
Despawning the agent despawns them. Report with `succeed` or `fail`, as many
times as you like: the first counts, and wakes the tree, which reads it on
its next tick.

As the tree takes a task away, [`TaskEnded`] is triggered on it, with how it
went, or `None` if it was cancelled before it reported; then it's despawned.
Clean up there, or in an `On<Remove<_>>` observer, if a task has anything to
undo.

[`Tasks`] pairs each task with what you want of its agent:
`tasks.for_each(|data, task, agent| ..)`. It's a shortcut for a query of
tasks and one of agents, paired by `TaskOf`, which you can write yourself.

`task.elapsed()` is how long the task has been running, by the clock trees
tick on, whichever schedule your task system runs in; it goes on while its
tree sleeps.

## Trees

- [`TreeFinished`] is triggered on the agent as its tree ends, and
  `BehaviourTree::outcome` says how. One under `Forever` never does.
  `BehaviourTree::restart` starts a finished tree again from the top.
- [`TreePaused`] on the agent holds its tree where it is: no guards asked,
  nothing started, its task not ticked by [`Tasks`]. Time goes on, so a
  `Wait` may be over when it's taken away. Put on by a tree ticked earlier
  in the same tick, it holds it already.
- A tree taken away, replaced, or despawned with its agent, by something it
  runs, stops there: nothing after that in it runs, and what it was running
  is cancelled.

## Custom nodes

The built-in nodes are written against [`Behave`] and nothing else, so
yours can do whatever they do. Implement `tick` and `cancel`, then put it in
a tree with `@ MyNode::new(..)`. Children are [`Behaviour`]s: run them with
`child.tick(cx)`; `cx` is the world and the agent. `outline` is optional,
for how it looks in a trace. See [`Behave`] for one written out.

Anything holding under a custom node keeps holding without it doing
anything. Four things are up to a node that does more than tick its child:

- Trees sleep. A custom node is ticked every tick while it runs, unless
  its `polls` says no: then, when it returns `Running`, it says what it
  waits on, with `cx.wake_at(time)` or `cx.watch::<T>(entity)`. What its
  children wait on, they say themselves.
- On a tick it doesn't tick a running child, call `child.skip(cx)`, or the
  child's hold lapses for that tick.
- To switch away from a child that [`holds`](behaviour::Behaviour::holds), tick it
  with `child.tick_interrupted(cx)` until it ends or stops holding, as
  `Select` and `While` do.
- While `cx.interrupted()`, start nothing new: go on with the child that's
  running, and end once it does, as the built-in nodes do.

## Scheduling

`TreePlugin` runs trees in [`TreeSystems`] (in `FixedUpdate`
unless you say otherwise). Put it after whatever guards read, and before
your task systems: a change is then acted on the tick it happens, and a new
task is ticked the tick it starts.

Ticking takes exclusive access to the world, and passes over trees that are
asleep: those with no system guard that could matter, no timer up, and
nothing they watch changed. A sleeping tree costs a look at each thing it
watches. One that's ticked gets one pass, which runs until a task is under
way; `Forever` starts its child at most twice in a tick, so a tree that does
nothing can't hang.

## Seeing it

`BehaviourTree` displays as an outline, with `>` on what's running. An agent
with a [`TreeTrace`] keeps that outline in it, for an inspector, changed only
when what's running is. `Task`, `TaskOf`, `AgentTasks`, `Graceful`,
`TreePaused` and `TreeTrace` are reflected and registered, so inspectors
show and edit them.

[`Graceful`]: task::Graceful
[`Task`]: task::Task
[`Task::interrupted`]: task::Task::interrupted
[`TaskOf`]: task::TaskOf
[`AgentTasks`]: task::AgentTasks
[`TaskEnded`]: task::TaskEnded
[`Tasks`]: task::Tasks
[`TreeFinished`]: tree::TreeFinished
[`TreePaused`]: tree::TreePaused
[`TreeTrace`]: tree::TreeTrace
[`Behave`]: behaviour::Behave
[`Behaviour`]: behaviour::Behaviour
[`BehaviourTree`]: tree::BehaviourTree
