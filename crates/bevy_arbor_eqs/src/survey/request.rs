use std::collections::VecDeque;
use std::fmt;
use std::marker::PhantomData;
use std::sync::Arc;

use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;
use bevy::utils::prelude::ShortName;

use super::found::{Found, write};
use crate::query::{EnvQuery, EnvQueryKey, Parts, ScoredItem};

/// A task running the query for `K` for its agent: `task(Survey::of(Cover))`.
///
/// The query is an [`EnvQuery<K>`](crate::query::EnvQuery) the agent carries,
/// or, asked [`on`](Self::on) something, that something carries: a turret's
/// "where to stand to use me". What it found goes on the agent as a
/// [`Found<K>`](super::Found), and the task succeeds if it found anything, and
/// fails if not.
///
/// Queries run after trees tick, as many each tick as the
/// [`EnvQueryBudget`](crate::EnvQueryBudget) allows, oldest first. One whose
/// carrier has no query for `K` is reported, and its task failed. For an
/// answer at once, outside a tree, run the query with
/// [`EnvQuery::run`](crate::query::EnvQuery::run).
#[derive(Component)]
#[component(on_add = queue::<K>)]
pub struct Survey<K: EnvQueryKey> {
    carrier: Carrier,
    key: PhantomData<fn() -> K>,
}

/// What carries the query a survey runs.
#[derive(Clone, Copy)]
enum Carrier {
    /// The agent itself.
    Querier,
    /// This entity.
    Entity(Entity),
    /// What the agent's last query of another kind found, read as it runs.
    Found(fn(&World, Entity) -> Option<Entity>),
}

/// Queues a survey as it's asked for.
fn queue<K: EnvQueryKey>(mut world: DeferredWorld, context: HookContext) {
    if let Some(mut pending) = world.get_resource_mut::<Pending>() {
        pending.0.push_back(Request::of::<K>(context.entity));
    } else {
        warn!(
            "{} asked for a `Survey<{}>`, but nothing runs queries: add `EnvQueryPlugin`",
            context.entity,
            ShortName::of::<K>()
        );
    }
}

impl<K: EnvQueryKey> Survey<K> {
    /// The query for `key` the agent carries.
    #[must_use]
    pub fn of(_key: K) -> Self {
        Self {
            carrier: Carrier::Querier,
            key: PhantomData,
        }
    }

    /// The query `carrier` carries, not the agent's own.
    #[must_use]
    pub fn on(mut self, carrier: Entity) -> Self {
        self.carrier = Carrier::Entity(carrier);
        self
    }

    /// The query carried by what the agent's [`Found<J>`] found, as it runs.
    ///
    /// None found, its task fails.
    #[must_use]
    pub fn on_found<J: EnvQueryKey>(mut self) -> Self {
        self.carrier = Carrier::Found(|world, agent| world.get::<Found<J>>(agent)?.entity());
        self
    }

    /// What carries its query, for `agent`, if anything.
    pub(super) fn carrier(&self, world: &World, agent: Entity) -> Option<Entity> {
        match self.carrier {
            Carrier::Querier => Some(agent),
            Carrier::Entity(entity) => Some(entity),
            Carrier::Found(found) => found(world, agent),
        }
    }
}

impl<K: EnvQueryKey> Clone for Survey<K> {
    fn clone(&self) -> Self {
        Self {
            carrier: self.carrier,
            key: PhantomData,
        }
    }
}

impl<K: EnvQueryKey> fmt::Debug for Survey<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Survey<{}>", ShortName::of::<K>())
    }
}

/// A survey waiting to run: what asks, and what's done for its key.
pub(crate) struct Request {
    /// What asks: a task, or the agent itself.
    pub(super) entity: Entity,
    /// What its key's survey does, without the key's type.
    pub(super) key: Box<dyn Keyed>,
}

impl Request {
    /// `entity`'s request for the query for `K`.
    pub(super) fn of<K: EnvQueryKey>(entity: Entity) -> Self {
        Self {
            entity,
            key: Box::new(For::<K>(PhantomData)),
        }
    }
}

/// What a waiting survey is asked of, as it runs.
pub(super) enum Asked {
    /// Nothing: it's no longer asked for, cancelled or gone.
    No,
    /// What would carry it, none found yet.
    OfNothing,
    /// This carrier.
    Of(Entity),
}

/// What a survey does for its key, without the key's type.
pub(super) trait Keyed: Send + Sync {
    /// The key's name, for saying what's missing.
    fn name(&self) -> String;

    /// What carries the query `entity` asks for, for `agent`.
    fn carrier(&self, world: &World, entity: Entity, agent: Entity) -> Asked;

    /// The query `carrier` carries, if it does.
    fn query(&self, world: &World, carrier: Entity) -> Option<Arc<Parts>>;

    /// Puts what it found on `agent`.
    fn write(&self, world: &mut World, agent: Entity, found: Vec<ScoredItem>);

    /// Takes the survey off `entity`.
    fn take(&self, world: &mut World, entity: Entity);
}

/// [`Keyed`] for `K`.
struct For<K>(PhantomData<fn() -> K>);

impl<K: EnvQueryKey> Keyed for For<K> {
    fn name(&self) -> String {
        ShortName::of::<K>().to_string()
    }

    fn carrier(&self, world: &World, entity: Entity, agent: Entity) -> Asked {
        match world.get::<Survey<K>>(entity) {
            None => Asked::No,
            Some(survey) => survey
                .carrier(world, agent)
                .map_or(Asked::OfNothing, Asked::Of),
        }
    }

    fn query(&self, world: &World, carrier: Entity) -> Option<Arc<Parts>> {
        world
            .get::<EnvQuery<K>>(carrier)
            .map(|query| Arc::clone(query.parts()))
    }

    fn write(&self, world: &mut World, agent: Entity, found: Vec<ScoredItem>) {
        write::<K>(world, agent, found);
    }

    fn take(&self, world: &mut World, entity: Entity) {
        if let Ok(mut entity) = world.get_entity_mut(entity) {
            entity.remove::<Survey<K>>();
        }
    }
}

/// The requests waiting, oldest first.
#[derive(Resource, Default)]
pub(crate) struct Pending(pub(super) VecDeque<Request>);
