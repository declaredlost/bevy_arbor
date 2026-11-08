use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use bevy::ecs::change_detection::CheckChangeTicks;
use bevy::prelude::*;

use super::components::{BehaviorTree, TreePaused};
use super::events::TreeFinished;
use super::trace::TreeTrace;
use crate::behavior::Context;
use crate::behavior::system::Systems;
use crate::task::Task;

/// Set as the tree being ticked goes: by the tree's hook, read by its nodes.
#[derive(Resource, Default)]
pub(crate) struct Ticking(pub(crate) Arc<AtomicBool>);

/// Ticks every tree that's woken. Each one's root comes out of its component
/// for the tick, as decorators and acts need the world: one of them may even
/// take the tree away, or replace it, and then nothing more of it runs.
///
/// What each tick waits on is kept on the tree, and it sleeps until that
/// comes. Then tasks are told how long they've run.
pub(crate) fn tick(
    world: &mut World,
    trees: &mut QueryState<(Entity, &BehaviorTree), Without<TreePaused>>,
    tasks: &mut QueryState<&mut Task>,
    mut agents: Local<Vec<Entity>>,
) {
    let gone = world.get_resource_or_init::<Ticking>().0.clone();
    // Nothing in a tick moves the clock, so it's read once for every tree.
    let clock = world.get_resource::<Time>().map(Time::elapsed);
    let now = world.change_tick();
    agents.extend(
        trees
            .iter(world)
            .filter(|(_, tree)| tree.due(world, clock, now))
            .map(|(agent, _)| agent),
    );
    let mut systems = world.remove_resource::<Systems>().unwrap_or_default();
    for agent in agents.drain(..) {
        let Ok(mut entity) = world.get_entity_mut(agent) else {
            continue;
        };
        // Asked again, as one ticked before it may have paused it.
        if entity.contains::<TreePaused>() {
            continue;
        }
        let Some(mut tree) = entity.get_mut::<BehaviorTree>() else {
            continue;
        };
        if tree.outcome.is_some() {
            continue;
        }
        let Some(mut root) = tree.root.take() else {
            continue;
        };
        gone.store(false, Ordering::Relaxed);
        let mut cx = Context::ticking(world, agent, &gone, clock, Some(&mut systems));
        let status = root.tick(&mut cx);
        let plan = cx.finish();
        match world.get_mut::<BehaviorTree>(agent) {
            Some(mut tree) if tree.root.is_none() => {
                tree.root = Some(root);
                tree.outcome = status.outcome();
                tree.plan = plan;
                tree.fresh = false;
                if let Some(outcome) = status.outcome() {
                    world.trigger(TreeFinished {
                        entity: agent,
                        outcome,
                    });
                }
            }
            _ => root.cancel(&mut Context::new(world, agent)),
        }
        if world.get::<TreeTrace>(agent).is_some()
            && let Some(traced) = world
                .get::<BehaviorTree>(agent)
                .and_then(|tree| tree.root.as_ref())
                .map(TreeTrace::of)
            && let Some(mut trace) = world.get_mut::<TreeTrace>(agent)
        {
            // Changed only when the tree is.
            trace.set_if_neq(traced);
        }
    }
    world.insert_resource(systems);
    if let Some(clock) = clock {
        for mut task in tasks.iter_mut(world) {
            // Time passing isn't a change a task system should react to.
            task.bypass_change_detection().advance(clock);
        }
    }
    world.flush();
}

/// Keeps the change ticks trees and their systems hold from getting too old,
/// as a schedule does for its own systems.
pub(crate) fn check_ticks(
    check: On<CheckChangeTicks>,
    mut trees: Query<&mut BehaviorTree>,
    systems: Option<ResMut<Systems>>,
) {
    for mut tree in &mut trees {
        tree.bypass_change_detection().plan.check_ticks(*check);
    }
    if let Some(mut systems) = systems {
        systems.check_ticks(*check);
    }
}
