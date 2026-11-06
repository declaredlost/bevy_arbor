//! What using trees takes: `use bevy_arbor::prelude::*`.
//!
//! Writing a node of your own takes [`Behave`](crate::behaviour::Behave) and
//! [`Context`](crate::behaviour::Context) too.

#[doc(hidden)]
pub use crate::behaviour::{Behaviour, Branch, Guard, IntoGuard, Outcome, ScoredBranch, Status};
#[doc(hidden)]
pub use crate::task::{AgentTasks, Graceful, Task, TaskEnded, TaskOf, Tasks};
#[doc(hidden)]
pub use crate::tree::{BehaviourTree, TreeFinished, TreePaused, TreeTrace};
#[doc(hidden)]
pub use crate::{ArborPlugin, ArborSystems, behaviour_tree};
