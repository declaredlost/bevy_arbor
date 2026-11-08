use std::any::TypeId;
use std::collections::VecDeque;
use std::sync::Arc;

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::utils::prelude::ShortName;

use super::found::Found;
use super::request::Survey;
use crate::query::{EnvQuery, Scored};

/// Adding queries to an app.
pub trait EnvQueryApp {
    /// Adds `query`, keyed by `Q`: what a [`Survey<Q>`] runs, and its
    /// [`Found<Q>`] holds. Adding another keyed alike replaces it.
    fn add_env_query<Q: Send + Sync + 'static>(&mut self, key: Q, query: EnvQuery) -> &mut Self;
}

impl EnvQueryApp for App {
    fn add_env_query<Q: Send + Sync + 'static>(&mut self, _key: Q, query: EnvQuery) -> &mut Self {
        let mut queries = self.world_mut().get_resource_or_init::<EnvQueries>();
        Arc::make_mut(&mut queries.0).insert(
            TypeId::of::<Q>(),
            Registered {
                query: Arc::new(query),
                write: write::<Q>,
            },
        );
        self
    }
}

/// The queries there are, by their key's type.
#[derive(Resource, Clone, Default)]
pub(crate) struct EnvQueries(pub(super) Arc<HashMap<TypeId, Registered>>);

/// A query, and what's done with what it found, without its key's type.
#[derive(Clone)]
pub(super) struct Registered {
    pub(super) query: Arc<EnvQuery>,
    /// Puts what it found on an agent.
    pub(super) write: fn(&mut World, Entity, Vec<Scored>),
}

/// A survey waiting to run, without its key's type.
pub(crate) struct Request {
    /// What asks.
    pub(super) entity: Entity,
    /// Its query's key.
    pub(super) key: TypeId,
    /// Its key's name, for saying it isn't there.
    pub(super) name: fn() -> String,
    /// Whether the entity still asks.
    pub(super) asked: fn(&World, Entity) -> bool,
    /// Takes the request off the entity.
    pub(super) taken: fn(&mut World, Entity),
}

impl Request {
    /// `entity`'s request for the query keyed by `Q`.
    pub(super) fn of<Q: Send + Sync + 'static>(entity: Entity) -> Self {
        Self {
            entity,
            key: TypeId::of::<Q>(),
            name: || ShortName::of::<Q>().to_string(),
            asked: |world, entity| world.get::<Survey<Q>>(entity).is_some(),
            taken: |world, entity| {
                if let Ok(mut entity) = world.get_entity_mut(entity) {
                    entity.remove::<Survey<Q>>();
                }
            },
        }
    }
}

/// The requests waiting, oldest first.
#[derive(Resource, Default)]
pub(crate) struct Pending(pub(super) VecDeque<Request>);

/// Where queries' randomness comes from, moved on each run.
#[derive(Resource)]
pub(crate) struct Seed(pub(super) u64);

impl Default for Seed {
    fn default() -> Self {
        Self(0x9E37_79B9_7F4A_7C15)
    }
}

/// Puts `found` on `agent` as its `Found<Q>`, or takes it off if it's empty;
/// changed only if it's different.
fn write<Q: Send + Sync + 'static>(world: &mut World, agent: Entity, found: Vec<Scored>) {
    let Ok(mut agent) = world.get_entity_mut(agent) else {
        return;
    };
    if found.is_empty() {
        agent.remove::<Found<Q>>();
    } else if agent
        .get::<Found<Q>>()
        .is_none_or(|known| known.items() != found.as_slice())
    {
        agent.insert(Found::<Q>::new(found));
    }
}
