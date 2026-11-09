# bevy_arbor_eqs

[![Build Status][action-badge]][action-url]
[![Crate Docs][docs-badge]][docs-url]
[![Crate Version][crates-badge]][crates-url]

**Part of [bevy_arbor](https://github.com/declaredlost/bevy_arbor), AI for Bevy.**

[action-badge]: https://img.shields.io/github/actions/workflow/status/declaredlost/bevy_arbor/build.yml?branch=main&label=build&logo=github&style=flat-square
[action-url]: https://github.com/declaredlost/bevy_arbor/actions/workflows/build.yml
[crates-badge]: https://img.shields.io/crates/v/bevy_arbor_eqs.svg?logo=rust&style=flat-square
[crates-url]: https://crates.io/crates/bevy_arbor_eqs
[docs-badge]: https://img.shields.io/docsrs/bevy_arbor_eqs?logo=Docs.rs&style=flat-square
[docs-url]: https://docs.rs/bevy_arbor_eqs

Environment queries for Bevy AI: where, or what, is best for an agent right
now.

## Overview

- An [`EnvQuery`] generates items, filters them, scores what's left, and picks
  the best. Generators make places round an anchor ([`Donut`], [`Grid`],
  [`Fan`], [`Row`]) or find things ([`Percepts<S>`], [`Nearby<C>`]).
- Tests measure each item from an anchor: [`Distance`], [`Dot`],
  [`Perceives<S>`], [`Random`], line of sight with the avian crates, or your
  own [`Test`]. Anchors are the agent, a point, a thing, or the hostiles it
  perceives.
- A behavior tree runs a query as a task, [`Survey<Q>`]. What it found goes on
  the agent as [`Found<Q>`], for conditions to read with `found::<Q>`.
- Queries run after the trees, within a per-tick budget, so many agents asking
  at once spread across ticks.

## Installation

Most games depend on `bevy_arbor` with its `eqs` feature, where this crate is
`bevy_arbor::eqs`:

```toml
[dependencies]
bevy_arbor = { version = "0.1", features = ["eqs"] }
```

Or depend on it directly, with `bevy_arbor_bt` and `bevy_arbor_perception`,
which it runs with:

```toml
[dependencies]
bevy_arbor_eqs = "0.1"
```

## Usage

```rust
use bevy::prelude::*;
use bevy_arbor_bt::prelude::*;
use bevy_arbor_eqs::prelude::*;
use bevy_arbor_perception::prelude::*;

/// Somewhere away from what's hostile, where it is or was last seen.
struct Cover;

#[derive(Component, Clone)]
struct TakeCover;

fn setup(app: &mut App) {
    app.add_env_query(
        Cover,
        // Points on rings round the agent.
        EnvQuery::new(Donut::around(Agent).radius(3.0, 9.0).rings(3).points(16))
            // Near the agent, and far from what's hostile.
            .score(Distance::to(Agent), Score::lower())
            .score(
                Distance::to(Hostiles::<Sight>::remembered()),
                Score::higher().weight(0.5),
            ),
    );
}

fn tree() -> Behavior {
    behavior_tree! {
        Select => {
            // Found: go there.
            when(found::<Cover>) => task(TakeCover),
            // Else look for it.
            task(Survey::of(Cover)),
        }
    }
}
```

## Learn more

- The book's [Queries](https://declaredlost.github.io/bevy_arbor/queries.html)
  and [Running queries](https://declaredlost.github.io/bevy_arbor/running-queries.html)
  pages, and [Feature flags](https://declaredlost.github.io/bevy_arbor/features.html)
- The [API documentation](https://docs.rs/bevy_arbor_eqs)

## Contributing

See [CONTRIBUTING.md](https://github.com/declaredlost/bevy_arbor/blob/main/CONTRIBUTING.md)
for development setup and guidelines.

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/declaredlost/bevy_arbor/blob/main/LICENSE-APACHE)
or [MIT license](https://github.com/declaredlost/bevy_arbor/blob/main/LICENSE-MIT) at your option.

[`EnvQuery`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/query/struct.EnvQuery.html
[`Donut`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/generate/struct.Donut.html
[`Grid`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/generate/struct.Grid.html
[`Fan`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/generate/struct.Fan.html
[`Row`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/generate/struct.Row.html
[`Percepts<S>`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/generate/struct.Percepts.html
[`Nearby<C>`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/generate/struct.Nearby.html
[`Distance`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/test/struct.Distance.html
[`Dot`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/test/struct.Dot.html
[`Perceives<S>`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/test/struct.Perceives.html
[`Random`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/test/struct.Random.html
[`Test`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/test/trait.Test.html
[`Survey<Q>`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/survey/struct.Survey.html
[`Found<Q>`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/survey/struct.Found.html
