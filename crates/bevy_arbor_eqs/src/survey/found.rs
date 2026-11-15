use std::fmt;
use std::marker::PhantomData;

use bevy::prelude::*;
use bevy::utils::prelude::ShortName;

use crate::query::{EnvQueryKey, ScoredItem};

/// On an agent: what the query keyed by `K` last found for it, best first.
///
/// Gone once a run finds nothing, so [`found::<K>`](found) is whether it has
/// something; changed only when a run finds something else.
#[derive(Component)]
pub struct Found<K: EnvQueryKey> {
    items: Vec<ScoredItem>,
    key: PhantomData<fn() -> K>,
}

impl<K: EnvQueryKey> Found<K> {
    pub(crate) fn new(items: Vec<ScoredItem>) -> Self {
        Self {
            items,
            key: PhantomData,
        }
    }

    /// The best of what it found.
    #[must_use]
    pub fn best(&self) -> Option<&ScoredItem> {
        self.items.first()
    }

    /// Where the best is.
    #[must_use]
    pub fn position(&self) -> Option<Vec3> {
        self.best().map(|scored| scored.item.position)
    }

    /// What the best is, if it's a thing.
    #[must_use]
    pub fn entity(&self) -> Option<Entity> {
        self.best().and_then(|scored| scored.item.entity)
    }

    /// How many it found: one, unless its query picks all.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether it found nothing, to go with [`len`](Self::len).
    ///
    /// Never so: it's gone once a run finds nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Everything it found, best first: one, unless its query picks all.
    pub fn iter(&self) -> impl Iterator<Item = &ScoredItem> {
        self.items.iter()
    }

    pub(crate) fn items(&self) -> &[ScoredItem] {
        &self.items
    }
}

impl<K: EnvQueryKey> fmt::Debug for Found<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(&format!("Found<{}>", ShortName::of::<K>()))
            .field("items", &self.items)
            .finish()
    }
}

/// A typed condition: whether the agent has something the query keyed by `K`
/// found.
///
/// Used as `when(found::<Cover>) => task(TakeCover)`.
#[must_use]
pub fn found<K: EnvQueryKey>(_: &Found<K>) -> bool {
    true
}

/// Puts `found` on `agent` as its `Found<K>`, or takes it off if it's empty.
///
/// Changed only if it's different.
pub(super) fn write<K: EnvQueryKey>(world: &mut World, agent: Entity, found: Vec<ScoredItem>) {
    let Ok(mut agent) = world.get_entity_mut(agent) else {
        return;
    };
    if found.is_empty() {
        agent.remove::<Found<K>>();
    } else if agent
        .get::<Found<K>>()
        .is_none_or(|known| known.items() != found.as_slice())
    {
        agent.insert(Found::<K>::new(found));
    }
}
