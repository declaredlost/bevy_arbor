#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

pub mod anchor;
pub mod generate;
mod plugin;
pub mod prelude;
pub mod query;
pub mod survey;
pub mod test;
#[cfg(test)]
mod testing;

pub use self::plugin::{EqsBudget, EqsPlugin, EqsSystems};
