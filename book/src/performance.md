# Performance

What things cost, measured on one machine in release builds. Your numbers will
differ; the shape won't.

## Trees

`examples/load` in `bevy_arbor_bt`: each agent's tree is a `Select` of four
guards over one running task.

| Agents | System guards | Typed guards, nothing changing | Typed guards, a tenth changing each tick |
|---|---|---|---|
| 100 | 17 µs | 3.7 µs | 5.5 µs |
| 500 | 78 µs | 12 µs | 21 µs |
| 2,000 | 305 µs | 41 µs | 78 µs |

Per agent, that's about 150 nanoseconds asking four system guards every tick,
and about 20 asleep. Run it with
`cargo run --release -p bevy_arbor_bt --example load`.

What keeps trees cheap:

- **Typed guards over system guards.** A system guard is asked every tick; a
  typed guard only once what it reads changes.
- **`set_if_neq`.** A component written every frame keeps every tree watching
  it awake ([Sleeping trees](sleeping.md)).
- **Noises at the rate they matter.** A noise each tick wakes every tree
  listening each tick.

## Sight

Sight finds what's near through a spatial grid, cells as large as the longest
sight, rebuilt each tick. In a 2D micro-benchmark, agents a tenth of whom look
each tick:

| Agents | Through everything | Through the grid |
|---|---|---|
| 1,000 | 58 µs | 38 µs |
| 2,000 | 196 µs | 81 µs |
| 10,000 | 4.13 ms | 435 µs |

Looking through everything grows with the square of the agents; the grid
grows with them, at about 10 nanoseconds a thing to rebuild. A longer
`interval` spreads looks thinner still.
