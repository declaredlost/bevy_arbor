# bevy_arbor_macros

[![Build Status][action-badge]][action-url]
[![Crate Docs][docs-badge]][docs-url]
[![Crate Version][crates-badge]][crates-url]

**Part of [bevy_arbor](https://github.com/declaredlost/bevy_arbor), AI for Bevy.**

[action-badge]: https://img.shields.io/github/actions/workflow/status/declaredlost/bevy_arbor/build.yml?branch=main&label=build&logo=github&style=flat-square
[action-url]: https://github.com/declaredlost/bevy_arbor/actions/workflows/build.yml
[crates-badge]: https://img.shields.io/crates/v/bevy_arbor_macros.svg?logo=rust&style=flat-square
[crates-url]: https://crates.io/crates/bevy_arbor_macros
[docs-badge]: https://img.shields.io/docsrs/bevy_arbor_macros?logo=Docs.rs&style=flat-square
[docs-url]: https://docs.rs/bevy_arbor_macros

The procedural macros of [bevy_arbor](https://github.com/declaredlost/bevy_arbor).
Don't depend on it directly: each macro is re-exported by its part.

## Overview

| Macro | Re-exported by |
|---|---|
| `behavior_tree!` | `bevy_arbor_bt`, and the `bevy_arbor` facade's prelude |

The macros find the part they expand to whether you depend on the part's
crate or on `bevy_arbor`, however it's named in your `Cargo.toml`.

## Learn more

- The book's [Building trees](https://declaredlost.github.io/bevy_arbor/trees.html)
  page, and [Feature flags](https://declaredlost.github.io/bevy_arbor/features.html)
- The [API documentation](https://docs.rs/bevy_arbor_macros)

## Contributing

See [CONTRIBUTING.md](https://github.com/declaredlost/bevy_arbor/blob/main/CONTRIBUTING.md)
for development setup and guidelines.

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/declaredlost/bevy_arbor/blob/main/LICENSE-APACHE)
or [MIT license](https://github.com/declaredlost/bevy_arbor/blob/main/LICENSE-MIT) at your option.
