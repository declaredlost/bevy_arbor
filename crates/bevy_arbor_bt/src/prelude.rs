//! What using trees takes, as one import: `use bevy_arbor_bt::prelude::*`.
//!
//! Writing a node of your own takes [`Behave`](crate::behavior::Behave) and
//! [`Context`](crate::behavior::Context) too, a condition of your own
//! [`Condition`](crate::condition::Condition), and a decorator acting as its
//! branch starts and ends [`Decorate`](crate::decorator::Decorate).

#[doc(no_inline)]
pub use crate::behavior::{Behavior, Branch, Outcome, Plugged, ScoredBranch, Status, plugged};
#[doc(no_inline)]
pub use crate::condition::{all, any, unless, with};
#[doc(no_inline)]
pub use crate::decorator::{
    Cooldown, Cooling, Decorator, IntoDecorator, cooldown, shared_cooldown,
};
#[doc(no_inline)]
pub use crate::inspect::Inspect;
#[doc(no_inline)]
pub use crate::task::{Graceful, Task, TaskCommands, TaskOf, TaskQuery, Tasks};
#[doc(no_inline)]
pub use crate::tree::{BehaviorTree, BehaviorTreeFinished, BehaviorTreePaused, BehaviorTreeTrace};
#[doc(no_inline)]
pub use crate::{BehaviorTreePlugin, BehaviorTreeSystems, behavior_tree};
