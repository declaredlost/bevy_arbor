//! The contract between AI and movement: where an agent is asked to go, its
//! [`Destination`], and how getting there goes, its [`Travel`], with the
//! [`Route`] it's taking, if your movement says.
//!
//! AI writes the destination; your movement reads it, finds a way with any
//! pathfinding (a navmesh, a grid, a platformer's jump graph), moves the
//! agent its own way, and writes how it goes. Nothing here moves anything.

use bevy::prelude::*;

/// On an agent: where it's asked to go, and how near counts as there.
/// Written by a [`MoveTo`](crate::move_to::MoveTo) task as it starts, and
/// taken off as it ends, or by you.
#[derive(Component, Reflect, Debug, Clone, Copy, PartialEq)]
#[reflect(Component, Debug, PartialEq)]
pub struct Destination {
    /// Where.
    pub goal: Goal,
    /// How near counts as there.
    pub within: f32,
}

impl Destination {
    /// How near counts as there unless set: half a unit.
    pub const WITHIN: f32 = 0.5;

    /// To a point.
    #[must_use]
    pub fn point(point: Vec3) -> Self {
        Self {
            goal: Goal::Point(point),
            within: Self::WITHIN,
        }
    }

    /// To an entity, wherever it goes: following it.
    #[must_use]
    pub fn entity(entity: Entity) -> Self {
        Self {
            goal: Goal::Entity(entity),
            within: Self::WITHIN,
        }
    }

    /// There when within `distance`.
    #[must_use]
    pub fn within(mut self, distance: f32) -> Self {
        self.within = distance;
        self
    }
}

/// Where a [`Destination`] is.
#[derive(Reflect, Debug, Clone, Copy, PartialEq)]
#[reflect(Debug, PartialEq)]
pub enum Goal {
    /// A point.
    Point(Vec3),
    /// An entity, wherever it goes.
    Entity(Entity),
}

impl Goal {
    /// Where it is now: the point, or the entity's place, if it has one.
    pub fn position(&self, places: &Query<&GlobalTransform>) -> Option<Vec3> {
        match *self {
            Self::Point(point) => Some(point),
            Self::Entity(entity) => places.get(entity).ok().map(GlobalTransform::translation),
        }
    }
}

/// On an agent: how getting to its [`Destination`] goes. Written by your
/// movement; a [`MoveTo`](crate::move_to::MoveTo) task succeeds once it's
/// [`Arrived`](Self::Arrived), and fails once it's
/// [`Unreachable`](Self::Unreachable).
#[derive(Component, Reflect, Debug, Clone, Copy, PartialEq, Eq)]
#[reflect(Component, Debug, PartialEq)]
pub enum Travel {
    /// On its way.
    Moving,
    /// There, within the destination's `within`.
    Arrived,
    /// No way there: no path, or stuck.
    Unreachable,
}

/// On an agent: the way it's taking to its [`Destination`], as points, if
/// your movement says. For gizmos and debugging; nothing reads it otherwise.
#[derive(Component, Reflect, Debug, Clone, PartialEq, Default)]
#[reflect(Component, Debug, PartialEq, Default)]
pub struct Route(pub Vec<Vec3>);
