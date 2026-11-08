# bevy_arbor_smart

[![Build Status][action-badge]][action-url]
[![Crate Docs][docs-badge]][docs-url]
[![Crate Version][crates-badge]][crates-url]

**Part of [bevy_arbor](https://github.com/declaredlost/bevy_arbor), AI for Bevy.**

[action-badge]: https://img.shields.io/github/actions/workflow/status/declaredlost/bevy_arbor/build.yml?branch=main&label=build&logo=github&style=flat-square
[action-url]: https://github.com/declaredlost/bevy_arbor/actions/workflows/build.yml
[crates-badge]: https://img.shields.io/crates/v/bevy_arbor_smart.svg?logo=rust&style=flat-square
[crates-url]: https://crates.io/crates/bevy_arbor_smart
[docs-badge]: https://img.shields.io/docsrs/bevy_arbor_smart?logo=Docs.rs&style=flat-square
[docs-url]: https://docs.rs/bevy_arbor_smart

Smart objects for Bevy AI: things in the world with slots an agent finds,
claims and uses, running behavior the object brings.

## Overview

- A slot is an entity [`SlotOf`] its object, offering what a component of
  yours on it says: a bench seat offers `Sit`, a turret's place `Gunner`. A
  [`SlotDisabled`] slot can't be found or claimed.
- [`FreeSlots<A>`] is an environment query generator: the slots offering `A`
  near an anchor that the agent could claim. Score and pick among them as with
  any query.
- The [`claim::<Q>()`](https://docs.rs/bevy_arbor_smart/latest/bevy_arbor_smart/claim/fn.claim.html)
  decorator holds the slot that query found while its branch runs, so no one
  else takes it. The claim is a relationship, [`ClaimedBy`] on the slot and
  [`Claims`] on the agent.
- A slot brings its own behavior with a `Plugged<Using>`. Claiming plugs it
  into the agent, for `Socket(Using)` in its tree to run.
- [`ClaimedSlots`] is a query anchor at the slots an agent holds.

## Installation

Most games depend on `bevy_arbor` with its `smart` feature, where this crate is
`bevy_arbor::smart`:

```toml
[dependencies]
bevy_arbor = { version = "0.1", features = ["smart"] }
```

Or depend on it directly, with the crates it runs with:

```toml
[dependencies]
bevy_arbor_smart = "0.1"
```

## Usage

```rust
use bevy::prelude::*;
use bevy_arbor_bt::prelude::*;
use bevy_arbor_eqs::prelude::*;
use bevy_arbor_smart::prelude::*;

#[derive(Component)]
struct Bench;

/// What a seat offers.
#[derive(Component)]
struct Sit;

/// The nearest free seat, a query's key.
struct Seat;

#[derive(Component, Clone)]
struct GoThere;

#[derive(Component, Clone)]
struct Rest;

#[derive(Component)]
struct Tired;

fn tired(_: &Tired) -> bool {
    true
}

/// What sitting on the bench is, brought by the bench.
fn sit_down() -> Behavior {
    behavior_tree! { task(Rest) }
}

fn spawn(mut commands: Commands) {
    commands
        .spawn((Bench, Transform::from_xyz(4.0, 0.0, 0.0)))
        .with_related::<SlotOf>((
            Sit,
            Plugged::new(Using, sit_down),
            Transform::from_xyz(3.5, 0.0, 0.0),
        ));
}

fn setup(app: &mut App) {
    app.add_env_query(
        Seat,
        EnvQuery::new(FreeSlots::<Sit>::within(Agent, 15.0))
            .score(Distance::to(Agent), Score::lower()),
    );
}

fn tree() -> Behavior {
    behavior_tree! {
        Select => {
            when(tired && found::<Seat> && claim::<Seat>()) => Sequence => {
                task(GoThere),
                Socket(Using),
            },
            when(tired) => task(Survey::of(Seat)),
        }
    }
}
```

## Learn more

- The book's [Smart objects](https://declaredlost.github.io/bevy_arbor/smart-objects.html)
  page, and [Feature flags](https://declaredlost.github.io/bevy_arbor/features.html)
- The [API documentation](https://docs.rs/bevy_arbor_smart)

## Contributing

See [CONTRIBUTING.md](https://github.com/declaredlost/bevy_arbor/blob/main/CONTRIBUTING.md)
for development setup and guidelines.

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/declaredlost/bevy_arbor/blob/main/LICENSE-APACHE)
or [MIT license](https://github.com/declaredlost/bevy_arbor/blob/main/LICENSE-MIT) at your option.

[`SlotOf`]: https://docs.rs/bevy_arbor_smart/latest/bevy_arbor_smart/slot/struct.SlotOf.html
[`SlotDisabled`]: https://docs.rs/bevy_arbor_smart/latest/bevy_arbor_smart/slot/struct.SlotDisabled.html
[`FreeSlots<A>`]: https://docs.rs/bevy_arbor_smart/latest/bevy_arbor_smart/query/struct.FreeSlots.html
[`ClaimedBy`]: https://docs.rs/bevy_arbor_smart/latest/bevy_arbor_smart/claim/struct.ClaimedBy.html
[`Claims`]: https://docs.rs/bevy_arbor_smart/latest/bevy_arbor_smart/claim/struct.Claims.html
[`ClaimedSlots`]: https://docs.rs/bevy_arbor_smart/latest/bevy_arbor_smart/query/struct.ClaimedSlots.html
