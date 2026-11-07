# How it fits together

An **agent** is an entity that carries a [`BehaviourTree`] and senses. Neither
part knows of the other: they meet in components on the agent.

| Part | Writes | Reads |
|---|---|---|
| Perception | `Perceived<S>` on the agent, one per sense | Transforms, `Perceivable`, noises, touches |
| Trees | Task entities, and `State` markers | Components on the agent, through guards |
| Your systems | Facts on the agent, work for tasks | Tasks, and anything else |

## A tick

Each tick, in `FixedUpdate` unless you say otherwise, inside [`ArborSystems`]:

1. **Perception** gathers what each agent might be sensing, lets your
   systems block and classify it, then writes what's left to `Perceived<S>`.
2. **Trees** are ticked, but only those that something woke: a component a
   guard reads changed, a task reported, a timer is up.
3. **Tasks** are ticked by your systems, after `ArborSystems`.

Put systems that keep facts on agents (targeting, health, a cooldown) before
`ArborSystems`, so trees see them the tick they change.

## Facts are components

Unreal keeps an AI's facts on a blackboard. In Bevy, the ECS already is one:
facts are components on the agent, and the systems keeping them up to date
are ordinary systems. A tree's typed guards read them, and the tree watches
them, so it needn't ask again until they change.

That's why perception writes components, and why a guard reads the agent's
own components only. Something about another entity, such as how far away the
target is, is kept on the agent by a system of yours, or read by a system
guard, which is asked every tick.

## Sleeping

A tree that nothing wakes isn't ticked. That's what makes typed guards cheap:
a monster wandering with nothing in view costs a look at the components it
watches, not a run of its guards. [Sleeping trees](sleeping.md) has what
wakes a tree, and what keeps one awake.

[`BehaviourTree`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/tree/struct.BehaviourTree.html
[`ArborSystems`]: https://docs.rs/bevy_arbor/latest/bevy_arbor/struct.ArborSystems.html
