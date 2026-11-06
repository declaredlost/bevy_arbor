//! The tree on an agent, and the system ticking them all.

mod components;
mod events;
mod outline;
mod systems;

pub use self::components::{BehaviourTree, TreePaused, TreeTrace};
pub use self::events::TreeFinished;
pub use self::outline::TreeOutline;
pub(crate) use self::systems::tick;
