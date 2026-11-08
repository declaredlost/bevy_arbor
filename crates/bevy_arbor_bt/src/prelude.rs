//! What using trees takes: `use bevy_arbor_bt::prelude::*`.
//!
//! Writing a node of your own takes [`Behave`](crate::behavior::Behave) and
//! [`Context`](crate::behavior::Context) too, and a decorator of your own
//! [`Decorate`](crate::decorator::Decorate).

#[doc(no_inline)]
pub use crate::behavior::{Behavior, Branch, Outcome, Plugged, ScoredBranch, Status, plugged};
#[doc(no_inline)]
pub use crate::decorator::{Cooldown, Cooling, Decorator, IntoDecorator, cooldown, cooldown_of};
#[doc(no_inline)]
pub use crate::task::{AgentTasks, Graceful, Task, TaskCommands, TaskOf, Tasks};
#[doc(no_inline)]
pub use crate::tree::{BehaviorTree, TreeFinished, TreePaused, TreeTrace};
#[doc(no_inline)]
pub use crate::{TreePlugin, TreeSystems, behavior_tree};
