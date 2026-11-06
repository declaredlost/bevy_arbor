<div align="center">

# bevy_arbor

**Reactive behaviour trees for Bevy: guards re-checked every tick, tasks as entities.**

[![Build](https://img.shields.io/github/actions/workflow/status/declaredlost/bevy_arbor/build.yml?branch=main&label=build&style=flat-square)](https://github.com/declaredlost/bevy_arbor/actions/workflows/build.yml)
[![Security](https://img.shields.io/github/actions/workflow/status/declaredlost/bevy_arbor/security.yml?branch=main&label=security&style=flat-square)](https://github.com/declaredlost/bevy_arbor/actions/workflows/security.yml)

[**github.com/declaredlost**](https://github.com/declaredlost)

</div>

An agent carries its tree as a component; each task the tree reaches is an
entity of its own, ticked by your systems, and the tree despawns it when it's
done or no longer wanted.

```rust,ignore
fn has_target(In(agent): In<Entity>, targets: Query<&Target>) -> bool {
    targets.get(agent).is_ok_and(|target| target.entity.is_some())
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

> [!WARNING]
> **Early development.** The API may change at any time.

## Crates

| Crate | Description |
|---|---|
| [`bevy_arbor`](crates/bevy_arbor) | Reactive behaviour trees: guards re-checked every tick, tasks as entities |
| [`bevy_arbor_macros`](crates/bevy_arbor_macros) | `bevy_arbor`'s `behaviour_tree!` macro |

## Requirements

- **Rust + Cargo**: 1.99+, Edition 2024
- **Bevy**: 0.20

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and guidelines, and
[CHANGELOG.md](CHANGELOG.md) for release notes.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

