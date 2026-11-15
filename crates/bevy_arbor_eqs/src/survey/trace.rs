use bevy::prelude::*;

use crate::query::Weighed;

/// On an agent: how its last query weighed every item, for gizmos or a test.
///
/// Kept only where it's put, as it costs a list a run.
#[derive(Component, Reflect, Debug, Clone, Default, PartialEq)]
#[reflect(Component, Debug, Default, PartialEq)]
pub struct EnvQueryTrace {
    /// Which query it was: its key's name.
    pub query: String,
    /// Every item, its score, or `None` if a filter dropped it, and whether
    /// it was picked.
    pub items: Vec<Weighed>,
}
