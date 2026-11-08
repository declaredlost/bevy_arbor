use bevy::prelude::*;

/// What a query weighs: a place, or a thing at a place.
#[derive(Reflect, Debug, Clone, Copy, PartialEq)]
#[reflect(Debug, PartialEq)]
pub enum Item {
    /// A place.
    Point(Vec3),
    /// A thing, where it was as it was generated.
    Entity {
        /// What it is.
        entity: Entity,
        /// Where it was.
        position: Vec3,
    },
}

impl Item {
    /// Where it is.
    #[must_use]
    pub fn position(&self) -> Vec3 {
        match *self {
            Self::Point(position) | Self::Entity { position, .. } => position,
        }
    }

    /// What it is, if it's a thing.
    #[must_use]
    pub fn entity(&self) -> Option<Entity> {
        match *self {
            Self::Point(_) => None,
            Self::Entity { entity, .. } => Some(entity),
        }
    }
}

/// An item a query kept, and its score: higher is better.
#[derive(Reflect, Debug, Clone, Copy, PartialEq)]
#[reflect(Debug, PartialEq)]
pub struct Scored {
    /// What it is.
    pub item: Item,
    /// How well it did: the scores' weighted sum, or 1 for a query that only
    /// filters.
    pub score: f32,
}
