#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]

// The macro names this crate by its path, here too: in its own tests.
extern crate self as bevy_arbor_bt;

pub mod behavior;
pub mod decorator;
mod plugin;
pub mod prelude;
pub mod task;
#[cfg(test)]
mod testing;
pub mod tree;

pub use bevy_arbor_macros::behavior_tree;

pub use self::plugin::{TreePlugin, TreeSystems};
