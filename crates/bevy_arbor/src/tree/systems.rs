use bevy::prelude::*;

use super::components::{BehaviourTree, TreePaused, TreeTrace};
use super::events::TreeFinished;
use crate::behave::Context;

/// Ticks every tree. Each one's root comes out of its component for the
/// tick, as guards and acts need the world: one of them may even take the
/// tree away, or replace it.
pub(crate) fn tick(
    world: &mut World,
    trees: &mut QueryState<Entity, (With<BehaviourTree>, Without<TreePaused>)>,
    mut agents: Local<Vec<Entity>>,
) {
    agents.extend(trees.iter(world));
    for agent in agents.drain(..) {
        let Some(mut tree) = world.get_mut::<BehaviourTree>(agent) else {
            continue;
        };
        if tree.outcome.is_some() {
            continue;
        }
        let Some(mut root) = tree.root.take() else {
            continue;
        };
        let mut cx = Context::new(world, agent);
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
        {
            world.entity_mut(agent).insert(TreeTrace(outline));
        }
    }
    world.flush();
}
