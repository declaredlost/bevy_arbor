//! Environment query tests that ask avian, with the `eqs` feature.
//!
//! Whether an anchor can see each item ([`LineOfSight`]), whether there's room
//! to stand at it ([`Clearance`]), and how far it is from what blocks sight
//! ([`OccluderDistance`]).

mod clearance;
mod distance;
mod sight;

use bevy::ecs::system::{ReadOnlySystemParam, SystemParam, SystemParamItem, SystemState};
use bevy::prelude::*;

pub use self::clearance::Clearance;
pub use self::distance::OccluderDistance;
pub use self::sight::LineOfSight;

/// What a test reads from the world, as a system would: its state there.
///
/// Made in the world its query runs in, the first time it does.
#[doc(hidden)]
pub struct Reads<P: SystemParam + 'static> {
    state: SystemState<P>,
    /// Whether it's warned that it can't read, so it says so once.
    warned: bool,
}

impl<P: SystemParam + 'static> FromWorld for Reads<P> {
    fn from_world(world: &mut World) -> Self {
        Self {
            state: SystemState::new(world),
            warned: false,
        }
    }
}

impl<P: ReadOnlySystemParam + 'static> Reads<P> {
    /// `read` with what it reads from `world`, for the test named `test`.
    ///
    /// Only if what it reads is there; if not, it warns, once.
    fn with<R>(
        &mut self,
        world: &World,
        test: &str,
        read: impl FnOnce(SystemParamItem<P>) -> R,
    ) -> Option<R> {
        let Ok(param) = self.state.get(world) else {
            if !std::mem::replace(&mut self.warned, true) {
                warn!(
                    "`{test}` has no answer: it can't read the world: it needs avian's `PhysicsPlugins`"
                );
            }
            return None;
        };
        Some(read(param))
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use bevy_arbor_eqs::query::EnvQueryItem;

    use super::LineOfSight;
    use crate::testing::*;

    #[test]
    fn without_physics_it_has_no_answer() {
        let test = LineOfSight::seen_by(bevy_arbor_eqs::anchor::Querier);
        let mut bare = World::new();
        let agent = bare.spawn(GlobalTransform::IDENTITY).id();
        assert_eq!(
            tested(&mut bare, agent, &test, EnvQueryItem::place(ahead(3.0))),
            None
        );
    }
}
