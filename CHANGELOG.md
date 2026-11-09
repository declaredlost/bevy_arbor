# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `bevy_arbor_bt`: reactive behavior trees, with tasks as entities. Moved
  from the game repository.
  - Trees sleep between ticks, until something they wait on comes: a task's
    report, a timer, a change a typed condition would see, or a system condition that
    has to be asked every tick.
  - Typed conditions: functions of the agent's components (`fn(&Target) -> bool`),
    asked again only once one of them changes.
  - `State(bundle) => { .. }`: a bundle on the agent while its branch runs.
  - Nodes run from `Behave::start`, through `tick`, to `end` (with how they
    went, or `None` if cancelled), called by the tree. In a trace, a node
    reads as its `label` and lists its `children`. They say what they wait
    on as they tick (`Context::wake_at`, `Context::watch`); a running node
    whose subtree said nothing is ticked every tick.
  - Decorators: what's in `when(..)`. One at rest says so with
    `Context::wait_on_nothing`, so its tree can sleep. Typed and system conditions
    are decorators, combined with `!`, `&&` and `||` into a `Decorator`.
    `Decorate` is for your own, told as its branch starts and ends;
    `cooldown(seconds)` keeps a branch from running again for that long after
    it ends.
  - System conditions and acts run from a table of their own, not Bevy's system
    registry: several times cheaper to ask.
  - `Repeat(times) => { .. }`: its child that many times, until it fails.
    `cooldown_of::<K>(seconds)`: a cooldown shared by every branch keyed by
    `K`, kept on the agent as `Cooling<K>`.
  - `Socket(Weapon)`: a place in a tree whose subtree is plugged in at
    runtime, by the agent's `Plugged<Weapon>`, and swapped as that is (once
    what runs stops holding); `plugged::<Weapon>` passes while something is.
  - `task(Chase)` names a task after its bundle's type; `task(name, bundle)`
    names it yourself.
  - `TaskCommands`: `commands.entity(task).succeed_task()`, for observers and
    systems that don't query `Task`. A task is set up and cleaned up with
    Bevy's own `On<Add<_>>` and `On<Remove<_>>` observers.
  - `TreeTrace` keeps the tree as nodes (`TraceNode`: parent, label, notes,
    running, held), each node's index its id; written out, it's the outline.
- `bevy_arbor_perception`: perception, in 2D or 3D, physics-free, generic
  over its senses. Each sense is a component (`Sight`, `Hearing`, `Touch`,
  `Shared`, or your own, registered with `SensePlugin`), and what it senses
  is written to the agent's `Perceived<S>`, which changes only when something
  is noticed, lost or forgotten by that sense; when and where each was
  sensed is in `Traces<S>` beside it, so a footstep from what an agent
  already heard wakes only trees reading the traces. Senses can sit on sensor
  entities (`SensorOf`), each sensing from its own transform. Sight goes
  through a spatial index (its own grid, or yours through `SightIndex`), by
  range, cone and `lose_range`, looks staggered across ticks; hearing from `Noise`, touch from `Touched`, and allies pass on
  what they perceive with `Share`, by every sense, each share's listeners
  found once. Systems of your own block and classify
  candidates in `PerceptionSystems::Occlude` and `Classify`; `Team` gives the
  attitude nothing else did. `Noticed<S>`, `Lost<S>` and `Forgotten<S>` are
  triggered on the agent.
- `bevy_arbor_avian2d` and `bevy_arbor_avian3d`: perception with avian, one
  source built for each dimension. `ColliderSightPlugin` lets sight find
  what's near through avian's colliders (`PerceivableLayers`) instead of
  perception's grid. `LineOfSightPlugin` blocks what's out of
  sight with ray casts (`Occluders`, `SensorOccluders`, `SightPoints`);
  `ContactTouchPlugin` turns collisions into `Touched`. In `ArborPlugins`
  through the facade's `avian2d` and `avian3d` features.
- `bevy_arbor_eqs`: environment queries. A query
  generates places (`Donut`, `Grid`, `Fan` round an anchor) or things
  (`Percepts<S>`, `Nearby<C>`), filters and scores them (`Distance`, `Dot`,
  `Perceives<S>`, `Random`, a system of yours as a `Measure`, or your own
  `Test`), and picks the best, a random
  one of the best, or all. Added with `add_env_query`, run by a tree as
  `task(Survey::of(Cover))` within a per-tick budget, and written to the
  agent as `Found<Cover>`. In the avian crates, with their `eqs`
  feature, tests for cover: `LineOfSight`, `Clearance` (room to stand) and
  `OccluderDistance` (how near what blocks sight). In the facade through the
  `eqs` feature.
- `bevy_arbor_smart`: smart objects. Slots are entities `SlotOf` an object,
  offering what a component of yours on them says; `FreeSlots<A>` finds the
  ones an agent could claim, for environment queries; the `claim::<Q>()`
  decorator holds the slot a query found while its branch runs (`ClaimedBy`,
  `Claims`), and plugs the slot's own `Plugged<Using>` into the agent, for
  `Socket(Using)`. `ClaimedSlots` is a query anchor at the slots an agent
  holds. In the facade through the `smart` feature.
- `bevy_arbor_nav`: navigation, as the contract between AI and the game's
  own movement and pathfinding. `Destination` is where an agent is asked to
  go, `Travel` how it's going (written by your movement), and `Route` the way
  it's taking. The `MoveTo` task sets the destination, succeeds on arrival,
  fails when there's no way, and takes the destination off however it ends;
  `MoveTo::found::<Q>()` goes where a query found. `PathDistance` measures path
  costs in queries with the pathfinder set by `set_path_costs`. In the facade
  through the `nav` feature.
- `bevy_arbor_gizmos`: drawing with Bevy's gizmos, one `ArborGizmosPlugin`:
  what agents perceive, and, with its `eqs`, `smart` and `nav` features, how
  queries weighed their items (a `SurveyTrace`), slots, free, claimed or
  disabled, and where agents are going.
  In the facade through the `gizmos` feature.
- The book, `book/`: a guide to bevy_arbor, chapter by chapter, its examples
  compiled and run as doctests, published to GitHub Pages.
- `bevy_arbor_macros`: the macros of every part, `behavior_tree!` so far.
- `bevy_arbor`: a facade over the parts, each crate re-exported whole behind
  a feature, with one prelude, an `ArborPlugins` group, and `ArborSystems` ordering the parts
  (perception, then trees) in each schedule they share.
