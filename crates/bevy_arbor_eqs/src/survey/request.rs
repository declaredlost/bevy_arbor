use std::fmt;
use std::marker::PhantomData;

use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;
use bevy::utils::prelude::ShortName;

use super::registry::{Pending, Request};

/// A request to run the query keyed by `Q` for an agent.
///
/// In a tree, a task: `task(Survey::of(Cover))`, run for the task's agent,
/// succeeding if it found anything and failing if not. On an agent of its
/// own, it's run for that agent, and taken off once it has. Either way, what
/// it found goes on the agent as a [`Found<Q>`](super::Found).
///
/// Queries run after trees tick, as many each tick as the
/// [`EqsBudget`](crate::EqsBudget) allows, oldest first.
///
/// One asking for a key never added with
/// [`add_env_query`](super::EnvQueryApp::add_env_query) is reported, and
/// taken off, or its task failed.
#[derive(Component)]
#[component(on_add = queue::<Q>)]
pub struct Survey<Q: Send + Sync + 'static> {
    key: PhantomData<fn() -> Q>,
}

/// Queues a survey as it's asked for, whenever that is: before its query is
/// added, too.
fn queue<Q: Send + Sync + 'static>(mut world: DeferredWorld, context: HookContext) {
    if let Some(mut pending) = world.get_resource_mut::<Pending>() {
        pending.0.push_back(Request::of::<Q>(context.entity));
    }
}

impl<Q: Send + Sync + 'static> Survey<Q> {
    /// A request for the query keyed by `Q`.
    #[must_use]
    pub fn of(_key: Q) -> Self {
        Self { key: PhantomData }
    }
}

impl<Q: Send + Sync + 'static> Default for Survey<Q> {
    fn default() -> Self {
        Self { key: PhantomData }
    }
}

impl<Q: Send + Sync + 'static> Clone for Survey<Q> {
    fn clone(&self) -> Self {
        Self { key: PhantomData }
    }
}

impl<Q: Send + Sync + 'static> fmt::Debug for Survey<Q> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Survey<{}>", ShortName::of::<Q>())
    }
}
