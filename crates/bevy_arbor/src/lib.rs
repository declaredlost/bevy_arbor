#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]

// The macro names this crate by its path, here too: in its own tests.
extern crate self as bevy_arbor;

mod behave;
mod behaviour;
mod guard;
mod plugin;
pub mod prelude;
mod task;
#[cfg(test)]
mod testing;
mod tree;

pub use bevy_arbor_macros::behaviour_tree;

pub use self::behave::{Behave, Context, Outcome, Status};
pub use self::behaviour::{Behaviour, Branch, ScoredBranch};
pub use self::guard::{Guard, IntoGuard};
pub use self::plugin::{ArborPlugin, ArborSystems};
pub use self::task::{Graceful, Task, Tasks};
pub use self::tree::{BehaviourTree, TreeFinished, TreeOutline, TreePaused, TreeTrace};
