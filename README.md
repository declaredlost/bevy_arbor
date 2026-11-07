<div align="center">

# bevy_arbor

**AI for Bevy: reactive behaviour trees that sleep until something changes, with tasks as entities.**

[![Build](https://img.shields.io/github/actions/workflow/status/declaredlost/bevy_arbor/build.yml?branch=main&label=build&style=flat-square)](https://github.com/declaredlost/bevy_arbor/actions/workflows/build.yml)
[![Security](https://img.shields.io/github/actions/workflow/status/declaredlost/bevy_arbor/security.yml?branch=main&label=security&style=flat-square)](https://github.com/declaredlost/bevy_arbor/actions/workflows/security.yml)

</div>

An agent carries its tree as a component; each task the tree reaches is an
entity of its own, ticked by your systems, and the tree despawns it when it's
done or no longer wanted. Between ticks a tree sleeps, until a component its
guards read changes, or its task reports.

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
```

The [book](https://declaredlost.github.io/bevy_arbor) is the guide: how the
pieces fit together, chapter by chapter. The API reference is on
[docs.rs](https://docs.rs/bevy_arbor).

> [!WARNING]
> **Early development.** The API may change at any time.

## Crates

| Crate | Description |
|---|---|
| [`bevy_arbor`](crates/bevy_arbor) | The facade: every part behind a feature, one prelude, one plugin group |
| [`bevy_arbor_bt`](crates/bevy_arbor_bt) | Reactive behaviour trees that sleep until something they wait on changes; tasks as entities |
| [`bevy_arbor_avian2d`, `bevy_arbor_avian3d`](crates/bevy_arbor_avian) | Perception with avian: line of sight, touch from contacts |
| [`bevy_arbor_perception`](crates/bevy_arbor_perception) | Perception: what an agent senses, in 2D or 3D, written as a component on it |
| [`bevy_arbor_macros`](crates/bevy_arbor_macros) | The macros of every part: `behaviour_tree!` for trees |

## Requirements

- **Rust + Cargo**: 1.99+, Edition 2024
- **Bevy**: 0.20

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and guidelines, and
[CHANGELOG.md](CHANGELOG.md) for release notes.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

