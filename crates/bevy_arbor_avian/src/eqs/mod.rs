//! Environment query tests that ask avian, with the `eqs` feature: whether an
//! anchor can see each item ([`LineOfSight`]), whether there's room to stand
//! at it ([`Clearance`]), and how far it is from what blocks sight
//! ([`OccluderDistance`]).

mod clearance;
mod distance;
mod sight;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};

use bevy::ecs::system::{ReadOnlySystemParam, SystemParam, SystemParamItem, SystemState};
use bevy::ecs::world::WorldId;
use bevy::prelude::*;

pub use self::clearance::Clearance;
pub use self::distance::OccluderDistance;
pub use self::sight::LineOfSight;

/// What a test reads from the world, as a system would, kept for the world
/// it's built for: a test runs in any world its query is added to.
struct Reads<P: SystemParam + 'static> {
    /// The test's name, for what it warns of.
    test: &'static str,
    kept: Mutex<Option<(WorldId, SystemState<P>)>>,
    /// Whether it's warned that it can't read, so it says so once.
    warned: AtomicBool,
}

impl<P: ReadOnlySystemParam + 'static> Reads<P> {
    fn new(test: &'static str) -> Self {
        Self {
            test,
            kept: Mutex::new(None),
            warned: AtomicBool::new(false),
        }
    }

    /// Builds it for `world`, unless it's built for it already: one built
    /// for another would fail it.
    fn prepare(&self, world: &mut World) {
        let mut kept = self.kept.lock().unwrap_or_else(PoisonError::into_inner);
        if kept.as_ref().is_none_or(|(id, _)| *id != world.id()) {
            *kept = Some((world.id(), SystemState::new(world)));
        }
    }

    /// `read` with what it reads from `world`, if it's built for it, and
    /// what it reads is there. If not, it warns, once.
    fn with<R>(&self, world: &World, read: impl FnOnce(SystemParamItem<P>) -> R) -> Option<R> {
        let mut kept = self.kept.lock().unwrap_or_else(PoisonError::into_inner);
        let Some((_, state)) = kept.as_mut().filter(|(id, _)| *id == world.id()) else {
            self.warn("wasn't prepared for this world: call `Test::prepare` first");
            return None;
        };
        let Ok(param) = state.get(world) else {
            self.warn("can't read the world: it needs avian's `PhysicsPlugins`");
            return None;
        };
        Some(read(param))
    }

    fn warn(&self, why: &str) {
        if !self.warned.swap(true, Ordering::Relaxed) {
            warn!("`{}` has no answer: it {why}", self.test);
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use bevy_arbor_eqs::query::{Item, QueryContext};
    use bevy_arbor_eqs::test::Test;

    use super::LineOfSight;
    use crate::testing::*;

    #[test]
    fn unprepared_or_without_physics_it_has_no_answer() {
        let test = LineOfSight::from(bevy_arbor_eqs::anchor::Agent);
        let mut app = app();
        let agent = looker(&mut app);
        app.tick();
        let cx = QueryContext::new(app.world(), agent, SPACE, 1);
        assert_eq!(test.test(&cx, &Item::Point(ahead(3.0))), None, "unprepared");
        let mut bare = World::new();
        let agent = bare.spawn(GlobalTransform::IDENTITY).id();
        test.prepare(&mut bare);
        let cx = QueryContext::new(&bare, agent, SPACE, 1);
        assert_eq!(test.test(&cx, &Item::Point(ahead(3.0))), None, "no physics");
    }
}
