#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

pub mod move_to;
#[cfg(feature = "eqs")]
#[cfg_attr(docsrs, doc(cfg(feature = "eqs")))]
pub mod path;
mod plugin;
pub mod prelude;
// The tests run queries too: with `eqs`, on by default.
#[cfg(all(test, feature = "eqs"))]
mod testing;
pub mod travel;

pub use self::plugin::{NavPlugin, NavSystems};
