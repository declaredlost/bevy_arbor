use bevy::prelude::*;

use crate::query::EnvQueryItem;

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

/// An item as a query weighed it: its score, if kept, and whether it was
/// picked.
#[derive(Reflect, Debug, Clone, Copy, PartialEq)]
#[reflect(Debug, PartialEq)]
pub struct Weighed {
    /// What it is.
    pub item: EnvQueryItem,
    /// Its score, or `None` if a filter dropped it.
    pub score: Option<f32>,
    /// Whether it was picked.
    pub picked: bool,
}
