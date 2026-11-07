#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

#[cfg(feature = "gizmos")]
#[cfg_attr(docsrs, doc(cfg(feature = "gizmos")))]
pub mod gizmos;
pub mod perceived;
mod plugin;
pub mod prelude;
pub mod sense;
pub mod space;
#[cfg(test)]
mod testing;

pub use self::plugin::{PerceptionPlugin, PerceptionSystems};
