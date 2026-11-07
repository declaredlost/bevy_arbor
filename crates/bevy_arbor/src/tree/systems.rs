use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use bevy::prelude::*;

use super::components::{BehaviourTree, TreePaused, TreeTrace};
use super::events::TreeFinished;
use crate::behaviour::Context;

/// Set as the tree being ticked goes: by the tree's hook, read by its nodes.
#[derive(Resource, Default)]
pub(crate) struct Ticking(pub(crate) Arc<AtomicBool>);

/// Ticks every tree. Each one's root comes out of its component for the tick,
/// as guards and acts need the world: one of them may even take the tree away,
/// or replace it, and then nothing more of it runs.
pub(crate) fn tick(
    world: &mut World,
    trees: &mut QueryState<Entity, (With<BehaviourTree>, Without<TreePaused>)>,
    mut agents: Local<Vec<Entity>>,
) {
    let gone = world.get_resource_or_init::<Ticking>().0.clone();
    agents.extend(trees.iter(world));
    for agent in agents.drain(..) {
        let Ok(mut entity) = world.get_entity_mut(agent) else {
            continue;
        };
        // Asked again, as one ticked before it may have paused it.
        if entity.contains::<TreePaused>() {
            continue;
        }
        let Some(mut tree) = entity.get_mut::<BehaviourTree>() else {
            continue;
        };
        if tree.outcome.is_some() {
            continue;
        }
        let Some(mut root) = tree.root.take() else {
            continue;
        };
        gone.store(false, Ordering::Relaxed);
        let mut cx = Context::ticking(world, agent, &gone);
        let status = root.tick(&mut cx);
        match world.get_mut::<BehaviourTree>(agent) {
            Some(mut tree) if tree.root.is_none() => {
                tree.root = Some(root);
                tree.outcome = status.outcome();
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
            && let Some(outline) = world.get::<BehaviourTree>(agent).map(ToString::to_string)
            && let Some(mut trace) = world.get_mut::<TreeTrace>(agent)
        {
            // Changed only when what's running is.
            trace.set_if_neq(TreeTrace(outline));
        }
    }
    world.flush();
}
