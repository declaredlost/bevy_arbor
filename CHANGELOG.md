# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `bevy_arbor_bt`: reactive behaviour trees, with tasks as entities. Moved
  from the game repository.
  - Trees sleep between ticks, until something they wait on comes: a task's
    report, a timer, a change a typed guard would see, or a system guard that
    has to be asked every tick.
  - Typed guards: functions of the agent's components (`fn(&Target) -> bool`),
    asked again only once one of them changes.
  - `State(bundle) => { .. }`: a bundle on the agent while its branch runs.
  - Custom nodes say what they wait on (`Behave::polls`, `Context::wake_at`,
    `Context::watch`); one that doesn't is ticked every tick.
  - System guards and acts run from a table of their own, not Bevy's system
    registry: several times cheaper to ask.
- `bevy_arbor_perception`: perception, in 2D or 3D, physics-free, generic
  over its senses. Each sense is a component (`Sight`, `Hearing`, `Touch`,
  `Shared`, or your own, registered with `SensePlugin`), and what it senses
  is written to the agent's `Perceived<S>`, which changes only when something
  is noticed, lost or forgotten by that sense. Senses can sit on sensor
  entities (`SensorOf`), each sensing from its own transform. Sight goes
  through a spatial grid, by range, cone and `lose_range`, looks staggered
  across ticks; hearing from `Noise`, touch from `Touched`, and allies pass on
  what they perceive with `Share`. Systems of your own block and classify
  candidates in `PerceptionSystems::Occlude` and `Classify`; `Team` gives the
  attitude nothing else did. `Noticed<S>`, `Lost<S>` and `Forgotten<S>` are
  triggered on the agent. With the `gizmos` feature, `PerceptionGizmosPlugin`
  draws it all.
- `bevy_arbor_avian2d` and `bevy_arbor_avian3d`: perception with avian, one
  source built for each dimension. `LineOfSightPlugin` blocks what's out of
  sight with ray casts (`Occluders`, `SensorOccluders`, `SightPoints`);
  `ContactTouchPlugin` turns collisions into `Touched`. In `ArborPlugins`
  through the facade's `avian2d` and `avian3d` features.
- The book, `book/`: a guide to bevy_arbor, chapter by chapter, its examples
  compiled and run as doctests, published to GitHub Pages.
- `bevy_arbor_macros`: the macros of every part, `behaviour_tree!` so far.
- `bevy_arbor`: a facade over the parts, each behind a feature, with one
  prelude, an `ArborPlugins` group, and `ArborSystems` ordering the parts
  (perception, then trees) in each schedule they share.
