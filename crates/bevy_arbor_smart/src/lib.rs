#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

pub mod claim;
mod plugin;
pub mod prelude;
pub mod query;
pub mod slot;
#[cfg(test)]
mod testing;

pub use self::plugin::SmartPlugin;
