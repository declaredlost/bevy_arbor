//! Task entities: what a tree spawns to do its work, and reads back.

mod components;
mod params;

pub use self::components::{Graceful, Task};
pub use self::params::Tasks;
