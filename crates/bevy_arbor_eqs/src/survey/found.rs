use std::fmt;
use std::marker::PhantomData;

use bevy::prelude::*;
use bevy::utils::prelude::ShortName;

use crate::query::ScoredItem;

/// On an agent: what the query keyed by `Q` last found for it, best first.
///
/// Gone once a run finds nothing, so [`found::<Q>`](found) is whether it has
/// something; changed only when a run finds something else.
#[derive(Component)]
pub struct Found<Q: Send + Sync + 'static> {
    items: Vec<ScoredItem>,
    key: PhantomData<fn() -> Q>,
}

impl<Q: Send + Sync + 'static> Found<Q> {
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

    /// Everything it found, best first: one, unless its query picks all.
    pub fn iter(&self) -> impl Iterator<Item = &ScoredItem> {
        self.items.iter()
    }

    pub(crate) fn items(&self) -> &[ScoredItem] {
        &self.items
    }
}

impl<Q: Send + Sync + 'static> fmt::Debug for Found<Q> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(&format!("Found<{}>", ShortName::of::<Q>()))
            .field("items", &self.items)
            .finish()
    }
}

/// A typed condition: whether the agent has something the query keyed by `Q`
/// found.
///
/// Used as `when(found::<Cover>) => task(TakeCover)`.
#[must_use]
pub fn found<Q: Send + Sync + 'static>(_: &Found<Q>) -> bool {
    true
}
