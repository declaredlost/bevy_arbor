# Performance

One guard costs next to nothing. A level with hundreds of them is where cost
shows. This chapter gives the measured numbers, then what keeps each part
cheap.

The numbers are from one machine, in release builds. Yours will differ. The
shape won't.

## Trees

`examples/load` in `bevy_arbor_bt` runs many agents. Each agent's tree is a
`Select` of four decorators over one running task.

| Agents | System conditions | Typed conditions, nothing changing | Typed conditions, a tenth changing each tick |
|---|---|---|---|
| 100 | 17 µs | 3.7 µs | 5.5 µs |
| 500 | 78 µs | 12 µs | 21 µs |
| 2,000 | 305 µs | 41 µs | 78 µs |

Per agent, that's about 150 nanoseconds when it asks four system conditions
every tick. Asleep, it's about 20. Run it yourself:

```sh
cargo run --release -p bevy_arbor_bt --example load
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

Sight finds what's near through a spatial grid. Its cells are as large as the
longest sight, and it's rebuilt each tick. This is a 2D micro-benchmark, in
which a tenth of the agents look each tick:

| Agents | Through everything | Through the grid |
|---|---|---|
| 1,000 | 58 µs | 38 µs |
| 2,000 | 196 µs | 81 µs |
| 10,000 | 4.13 ms | 435 µs |

Looking through everything grows with the square of the agents. The grid
grows in step with them, at about 10 nanoseconds a thing to rebuild.

What keeps perception cheap:

- **A longer `interval`.** Sight looks five times a second by default, each
  agent at a moment of its own. Guards far from the player can look less
  often.
- **Short sight.** The grid's cells are as large as the longest sight. One
  agent that sees very far makes every look check more things.
- **Occlusion only where needed.** A line-of-sight ray is cast only for what's
  already in range and in the cone. Fewer colliders on the occluding layers
  make each ray cheaper. See [Occlusion and attitude](occlusion-attitude.md).

## Queries

Queries share a time budget, [`EqsBudget`], a millisecond each tick unless you
set it. Queries over budget wait for the next tick, oldest first. At least one
runs each tick. So many guards asking at once delay each other, rather than
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

`TreeTrace` keeps a string per node and `SurveyTrace` a list per run. They're
kept only on agents you put them on. Leave them off in a shipped build. See
[Debugging](debugging.md).

[`EqsBudget`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/struct.EqsBudget.html
