use std::sync::{Mutex, PoisonError};

use bevy::ecs::query::{QueryFilter, ReadOnlyQueryData};
use bevy::ecs::world::WorldId;
use bevy::prelude::*;

/// A Bevy query a generator keeps between runs, for the world it runs in:
/// made once, not each run, and brought up to date with what's been spawned
/// since. A generator of things with a component, as [`Nearby`](super::Nearby)
/// is, holds one.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_arbor_eqs::generate::{CachedQuery, Generate};
/// # use bevy_arbor_eqs::query::{Item, QueryContext};
/// #[derive(Component)]
/// struct Crate;
///
/// /// Every crate, wherever it is.
/// #[derive(Default)]
/// struct Crates(CachedQuery<(Entity, &'static GlobalTransform), With<Crate>>);
///
/// impl Generate for Crates {
///     fn generate(&self, cx: &QueryContext, out: &mut Vec<Item>) {
///         self.0.with(cx.world(), |crates| {
///             for (entity, transform) in &crates {
///                 let position = cx.space().flatten(transform.translation());
///                 out.push(Item::Entity { entity, position });
///             }
///         });
///     }
/// }
/// ```
pub struct CachedQuery<D: ReadOnlyQueryData + 'static, F: QueryFilter + 'static = ()> {
    kept: Mutex<Option<(WorldId, QueryState<D, F>)>>,
}

impl<D: ReadOnlyQueryData + 'static, F: QueryFilter + 'static> Default for CachedQuery<D, F> {
    fn default() -> Self {
        Self {
            kept: Mutex::new(None),
        }
    }
}

impl<D: ReadOnlyQueryData + 'static, F: QueryFilter + 'static> CachedQuery<D, F> {
    /// Runs `run` with the query over `world`, and what it gives back. `None`
    /// while something it asks for isn't known to the world yet: no entity
    /// could match.
    pub fn with<R>(
        &self,
        world: &World,
        run: impl FnOnce(Query<'_, '_, D::ReadOnly, F>) -> R,
    ) -> Option<R> {
        let mut kept = self.kept.lock().unwrap_or_else(PoisonError::into_inner);
        if kept.as_ref().is_none_or(|(id, _)| *id != world.id()) {
            *kept = QueryState::try_new(world).map(|state| (world.id(), state));
        }
        let (_, state) = kept.as_mut()?;
        state.update_archetypes(world);
        Some(run(state.query_manual(world)))
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use super::CachedQuery;

    #[derive(Component)]
    struct Crate;

    #[derive(Component)]
    struct Heavy;

    fn count(query: &CachedQuery<Entity, With<Crate>>, world: &World) -> Option<usize> {
        query.with(world, |crates| crates.iter().count())
    }

    #[test]
    fn it_finds_what_was_spawned_since_and_what_was_unknown_at_first() {
        let mut world = World::new();
        let query = CachedQuery::<Entity, With<Crate>>::default();
        assert_eq!(count(&query, &world), None, "`Crate` unknown yet");
        world.spawn(Crate);
        assert_eq!(count(&query, &world), Some(1));
        // A new archetype, after the query was made.
        world.spawn((Crate, Heavy));
        assert_eq!(count(&query, &world), Some(2));
    }

    #[test]
    fn another_world_gets_a_query_of_its_own() {
        let query = CachedQuery::<Entity, With<Crate>>::default();
        let mut first = World::new();
        first.spawn(Crate);
        assert_eq!(count(&query, &first), Some(1));
        let mut second = World::new();
        second.spawn(Crate);
        second.spawn(Crate);
        assert_eq!(count(&query, &second), Some(2));
    }
}
