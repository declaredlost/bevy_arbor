#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

pub mod perceived;
mod plugin;
pub mod prelude;
pub mod sense;
mod space;
#[cfg(test)]
mod testing;

pub use self::plugin::{PerceptionPlugin, PerceptionSystems};
pub use self::space::Space;
