//! Environment query tests that ask avian, with the `eqs` feature: whether an
//! anchor can see each item ([`LineOfSight`]), whether there's room to stand
//! at it ([`Clearance`]), and how far it is from what blocks sight
//! ([`OccluderDistance`]).

mod clearance;
mod distance;
mod sight;

use std::sync::Mutex;

use bevy::ecs::system::{ReadOnlySystemParam, SystemParam, SystemParamItem, SystemState};
use bevy::ecs::world::WorldId;
use bevy::prelude::*;

pub use self::clearance::Clearance;
pub use self::distance::OccluderDistance;
pub use self::sight::LineOfSight;

/// What a test reads from the world, as a system would, kept for the world
/// it's built for: a test runs in any world its query is added to.
struct Reads<P: SystemParam + 'static> {
    kept: Mutex<Option<(WorldId, SystemState<P>)>>,
}

impl<P: ReadOnlySystemParam + 'static> Reads<P> {
    fn new() -> Self {
        Self {
            kept: Mutex::new(None),
        }
    }

    /// Builds it for `world`, unless it's built for it already: one built
    /// for another would fail it.
    fn prepare(&self, world: &mut World) {
        if let Ok(mut kept) = self.kept.lock()
            && kept.as_ref().is_none_or(|(id, _)| *id != world.id())
        {
            *kept = Some((world.id(), SystemState::new(world)));
        }
    }

    /// `read` with what it reads from `world`, if it's built for it.
    fn with<R>(&self, world: &World, read: impl FnOnce(SystemParamItem<P>) -> R) -> Option<R> {
        let mut kept = self.kept.lock().ok()?;
        let (id, state) = kept.as_mut()?;
        if *id != world.id() {
            return None;
        }
        state.get(world).ok().map(read)
    }
}
