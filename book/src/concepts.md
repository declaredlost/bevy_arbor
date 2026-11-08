# How it fits together

The parts of bevy_arbor don't know about each other. They meet in
components on the agent. This chapter shows who writes what, and in which
order.

## Who writes what

| Part | Writes | Reads |
|---|---|---|
| Perception | `Perceived<S>` on the agent, one per sense | Transforms, `Perceivable`, noises, touches |
| Trees | Task entities | Components on the agent, through decorators |
| Queries | `Found<Q>` on the agent | Transforms, `Perceived<S>`, your components |
| Your systems | Facts on the agent, and the work of tasks | Tasks, and anything else |

For the guard, perception writes `Perceived<Sight>`. The tree reads it
through `sees_intruder`, and spawns a `Chase` task. Your `chase` system moves
the guard.

## One tick

Each part runs inside the [`ArborSystems`] set, in `FixedUpdate` by default:

1. **Perception** gathers what each agent might sense. Your systems can block
   or classify it. What's left is written to `Perceived<S>`.
2. **Trees** that something woke are ticked. A condition's component changed,
   a task reported, or a timer ran out. A woken tree gets one pass, which runs
   until a task is under way. `Forever` restarts its child at most twice in a
   tick, so a tree that does nothing can't hang.
3. **Queries** the trees asked for are run, within a budget.
4. **Navigation**: each `MoveTo` task a tree started sets its agent's
   `Destination`, and each reports as the agent's `Travel` says.

Then your task systems and movement run. Put them after `ArborSystems`:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# fn patrol() {}
# fn chase() {}
# fn keep_ammo_count() {}
# let mut app = App::new();
app.add_systems(FixedUpdate, (patrol, chase).after(ArborSystems))
    .add_systems(FixedUpdate, keep_ammo_count.before(ArborSystems));
```

Systems that keep facts on agents, such as ammo or health, go before
`ArborSystems`. Then trees see a change the same tick it happens.

## Facts are components

Many behavior tree libraries keep an agent's facts on a blackboard. In Bevy,
the ECS already is one. The guard's ammo is an `Ammo` component. The systems
that keep it up to date are ordinary systems.

A typed condition such as `out_of_ammo(ammo: &Ammo)` reads the agent's own
components. The tree watches those components, so it doesn't need to ask
again until they change.

A fact about another entity, such as how far away the intruder is, needs
more. Either keep it on the agent with a system of yours, or use a system
condition. System conditions are asked every tick. [Decorators](decorators.md)
covers both.

## Trees sleep

A tree that nothing wakes isn't ticked. A guard patrolling with nothing in
view costs a check of the components it watches. Its conditions don't run.
[Sleeping trees](sleeping.md) has what wakes a tree, and what keeps one
awake.

## Pitfalls

- **Different schedules.** If you move trees and perception to different
  schedules, they no longer share a tick. Perception's changes then reach the
  tree on its next run.
- **Facts changed after trees.** A fact changed after `ArborSystems` wakes
  the tree on the next tick, not this one.
- **Task systems in another schedule.** `.after(ArborSystems)` orders systems
  only within one schedule. Put task systems in the trees' schedule,
  `FixedUpdate` unless you changed it.

[`ArborSystems`]: https://docs.rs/bevy_arbor/latest/bevy_arbor/struct.ArborSystems.html
