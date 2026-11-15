//! What using trees takes: `use bevy_arbor_bt::prelude::*`.
//!
//! Writing a node of your own takes [`Behave`](crate::behavior::Behave) and
//! [`Context`](crate::behavior::Context) too, a condition of your own
//! [`Condition`](crate::decorator::Condition), and a decorator acting as its
//! branch starts and ends [`Decorate`](crate::decorator::Decorate).
//!
//! The conditions' `and`, `or` and `not` aren't here, as Bevy's prelude has
//! run conditions' by those names: in `when(..)`, write `&&`, `||` and `!`, or
//! import [`IntoCondition`](crate::decorator::IntoCondition) and
//! [`not`](crate::decorator::not).

#[doc(no_inline)]
pub use crate::behavior::{Behavior, Branch, Outcome, Plugged, ScoredBranch, Status, plugged};
#[doc(no_inline)]
pub use crate::decorator::{
    Cooldown, Cooling, Decorator, IntoDecorator, cooldown, cooldown_of, with,
};
#[doc(no_inline)]
pub use crate::task::{AgentTasks, Graceful, Task, TaskCommands, TaskOf, Tasks};
#[doc(no_inline)]
pub use crate::tree::{BehaviorTree, TreeFinished, TreePaused, TreeTrace};
#[doc(no_inline)]
pub use crate::{TreePlugin, TreeSystems, behavior_tree};
