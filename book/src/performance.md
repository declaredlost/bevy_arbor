# Performance

One guard costs next to nothing. A level with hundreds of them is where cost
shows. This chapter says what each part costs, and what keeps it cheap.

## Trees

A sleeping tree costs next to nothing: it isn't looked at until what it
waits on comes. An awake one costs its tick, and asking its conditions. So
what costs is what keeps trees awake.

To measure, run the benchmarks, of trees awake, asleep, a few changing, their
tasks finishing, and spawning:

```sh
cargo bench -p bevy_arbor_bt --bench trees
```

What keeps trees cheap:

- **Typed conditions over system conditions.** A system condition is asked
  every tick. A typed condition is asked again only when what it reads
  changes. See [Decorators](decorators.md).
- **`set_if_neq`.** A component written every frame keeps every tree that
  reads it awake, even if the value is the same. Write it only when it
  changes. See [Sleeping trees](sleeping.md).
- **Noises at the rate they matter.** A noise each tick wakes every tree
  listening, each tick.
- **Timing in tasks.** A task that counts its own time doesn't wake the tree
  to do it. See [Large trees and sockets](large-trees.md).

## Sight

Without avian, sight finds what's near through a spatial grid of its own. Its
cells are as large as a typical sight, and it's rebuilt on the ticks a sensor
looks. Looking through everything would grow with the square of the agents;
the grid grows in step with them.

With avian, sight asks avian's colliders instead, which avian keeps up to date
anyway: nothing to rebuild. See
[Occlusion and attitude](occlusion-attitude.md#finding-whats-near).

What keeps perception cheap:

- **A longer `interval`.** Sight looks five times a second by default, each
  agent at a moment of its own. Guards far from the player can look less
  often.
- **Perceivables on a layer of their own.** With avian, a look asks for every
  collider in range. Set `PerceivableLayers`, so it skips walls and tiles.
- **Occlusion only where needed.** A line-of-sight ray is cast only for what's
  already in range and in the cone. Fewer colliders on the occluding layers
  make each ray cheaper. See [Occlusion and attitude](occlusion-attitude.md).

## Queries

Queries share a time budget, [`EnvQueryBudget`], a millisecond each tick unless
you set it. Queries over budget wait for the next tick, oldest first. At least
one runs each tick. So many guards asking at once delay each other, rather than
stall the frame. See [Running queries](running-queries.md).

What keeps queries cheap:

- **Few items.** Every item runs every test. A donut of 3 rings of 16 is 48
  points. Keep generators coarse.
- **Cheap filters first.** Filters run cheapest first by each test's `Cost`.
  A `LineOfSight` filter then casts rays only for what's left.
- **`test_all` for setup.** A test of your own that looks something up once a
  run, not once an item, should do it in `test_all`.
- **Ask when needed.** Run a query when the guard needs an answer, not every
  tick.

## Debugging costs

`BehaviorTreeTrace` keeps a string per node and `EnvQueryTrace` a list per run.
They're kept only on agents you put them on. Leave them off in a shipped
build. See [Debugging](debugging.md).

[`EnvQueryBudget`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/struct.EnvQueryBudget.html
