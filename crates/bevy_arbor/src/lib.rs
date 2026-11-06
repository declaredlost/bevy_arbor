#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]

// The macro names this crate by its path, here too: in its own tests.
extern crate self as bevy_arbor;

pub mod behaviour;
mod plugin;
pub mod prelude;
pub mod task;
#[cfg(test)]
mod testing;
pub mod tree;

pub use bevy_arbor_macros::behaviour_tree;

pub use self::plugin::{ArborPlugin, ArborSystems};
