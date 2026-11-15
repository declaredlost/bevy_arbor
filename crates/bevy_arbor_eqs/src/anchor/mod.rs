//! Anchors: where generators and tests measure from: none, one or several
//! places.
//!
//! [`Querier`], the query's [`Carrier`], a fixed point [`At`], a fixed entity
//! [`Thing`], what the agent perceives
//! ([`Percepts`](crate::perceived::Percepts)), or a [`Locate`] of your own.

mod fixed;

use std::sync::Arc;

use bevy::prelude::*;

pub use self::fixed::{At, Carrier, Querier, Thing};
use crate::query::EnvQueryContext;

/// A place an anchor is at, which way it looks there, and the thing it is, if
/// any.
///
/// Which way is zero where it doesn't look. The thing is what a ray from it
/// mustn't be blocked by.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Located {
    /// Where.
    pub position: Vec3,
    /// Which way it looks, unit length, or zero.
    pub ahead: Vec3,
    /// The thing it is, if it's one.
    pub entity: Option<Entity>,
}

/// An anchor as a generator or test keeps it: any [`Locate`], shared.
///
/// Located once a run, however many items and tests ask, by
/// [`EnvQueryContext::locate`].
#[derive(Clone)]
pub struct EnvQueryAnchor(pub(crate) Arc<dyn Locate>);

impl EnvQueryAnchor {
    /// `anchor`, to keep.
    pub fn new(anchor: impl Locate) -> Self {
        Self(Arc::new(anchor))
    }
}

impl Located {
    /// Whether `position` is within `range` of here.
    #[must_use]
    pub fn reaches(&self, position: Vec3, range: f32) -> bool {
        self.position.distance(position) <= range
    }
}

/// A kind of anchor: where a generator or test measures from, for an agent's
/// query.
///
/// None, one, or several places: a test against several takes the nearest.
pub trait Locate: Send + Sync + 'static {
    /// Where it is, for the agent of `cx`: pushed onto `out`.
    fn locate(&self, cx: &EnvQueryContext, out: &mut Vec<Located>);
}

#[cfg(test)]
mod tests {
    use bevy_arbor_perception::prelude::*;
    use bevy_harness::TestApp;

    use super::*;
    use crate::query::EnvQueryContext;
    use crate::testing::*;

    fn located(app: &TestApp, agent: Entity, anchor: &dyn Locate) -> Vec<Vec3> {
        let cx = EnvQueryContext::new(app.world(), agent, Space::Volume, 1);
        let mut out = Vec::new();
        anchor.locate(&cx, &mut out);
        out.into_iter().map(|located| located.position).collect()
    }

    #[test]
    fn the_agent_a_point_and_a_thing_are_where_they_are() {
        let mut app = app(Space::Volume);
        let agent = app.spawn(GlobalTransform::from_xyz(1.0, 0.0, 2.0));
        let thing = app.spawn(GlobalTransform::from_xyz(5.0, 0.0, 0.0));
        app.tick();
        assert_eq!(located(&app, agent, &Querier), [Vec3::new(1.0, 0.0, 2.0)]);
        assert_eq!(located(&app, agent, &At(Vec3::X)), [Vec3::X]);
        assert_eq!(
            located(&app, agent, &Thing(thing)),
            [Vec3::new(5.0, 0.0, 0.0)]
        );
    }
}
