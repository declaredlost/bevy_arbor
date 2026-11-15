use bevy::prelude::*;

/// What a query weighs: a place, or a thing at a place, as its generator made
/// it.
#[derive(Reflect, Debug, Clone, Copy, PartialEq)]
#[reflect(Debug, PartialEq)]
pub struct EnvQueryItem {
    /// Where it is: a thing's, where it was as it was generated.
    pub position: Vec3,
    /// What it is, if it's a thing.
    pub entity: Option<Entity>,
}

impl EnvQueryItem {
    /// A place, at `position`.
    #[must_use]
    pub fn place(position: Vec3) -> Self {
        Self {
            position,
            entity: None,
        }
    }

    /// A thing, `entity`, at `position`.
    #[must_use]
    pub fn thing(entity: Entity, position: Vec3) -> Self {
        Self {
            position,
            entity: Some(entity),
        }
    }
}

/// An item a query kept, and its score: higher is better, 1 if it only
/// filters.
#[derive(Reflect, Debug, Clone, Copy, PartialEq)]
#[reflect(Debug, PartialEq)]
pub struct ScoredItem {
    /// What it is.
    pub item: EnvQueryItem,
    /// How well it did: the scores' weighted sum, or 1 for a query that only
    /// filters.
    pub score: f32,
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
