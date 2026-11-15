use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use bevy::ecs::change_detection::{CheckChangeTicks, Tick};
use bevy::ecs::component::ComponentId;
use bevy::prelude::*;

use super::components::{BehaviorTree, BehaviorTreePaused, TreeState};
use super::events::BehaviorTreeFinished;
use super::trace::BehaviorTreeTrace;
use super::wake::{Owner, Owners, Plan, Wakes};
use crate::behavior::system::Systems;
use crate::behavior::{Behavior, Context, Status, Waits};
use crate::task::Task;

/// Set as the tree being ticked goes: by the tree's hook, read by its nodes.
#[derive(Resource, Default)]
pub(crate) struct Ticking(pub(crate) Arc<AtomicBool>);

/// Ticks every tree that's woken, its root taken out of its component for the
/// tick.
///
/// Decorators and acts need the world: one of them may even take the tree away,
/// or replace it, and then nothing more of it runs.
///
/// Which may be woken, the [`Wakes`] index says: a sleeping tree isn't looked
/// at. Each is asked whether its wait is over, and ticked if it is. What its
/// tick waits on goes back in the index, and it sleeps until that comes. Then
/// tasks are told how long they've run.
pub(crate) fn tick(
    world: &mut World,
    tasks: &mut QueryState<&mut Task>,
    mut agents: Local<Vec<Entity>>,
    mut next: Local<Vec<Entity>>,
    mut runs: Local<u64>,
) {
    *runs += 1;
    let gone = world.get_resource_or_init::<Ticking>().0.clone();
    // Nothing in a tick moves the clock, so it's read once for every tree.
    let clock = world.get_resource::<Time>().map(Time::elapsed);
    let now = world.change_tick();
    world.resource_scope(|world, mut wakes: Mut<Wakes>| {
        wakes.due(world, clock, now, &mut agents);
    });
    let mut systems = world.remove_resource::<Systems>().unwrap_or_default();
    let mut run = Run {
        id: *runs,
        clock,
        now,
        gone,
        systems: &mut systems,
        fired: Owners::EMPTY,
        traces: world.component_id::<BehaviorTreeTrace>(),
        next: &mut next,
        spare: Plan::default(),
    };
    for agent in agents.drain(..) {
        tick_tree(world, agent, &mut run);
    }
    world.insert_resource(systems);
    world.resource_mut::<Wakes>().looked_at(&mut next);
    if let Some(clock) = clock {
        for mut task in tasks.iter_mut(world) {
            // Time passing isn't a change a task system should react to.
            task.bypass_change_detection().advance(clock);
        }
    }
    world.flush();
}

/// What one run of the trees shares: which run, its clock, its trees' systems.
struct Run<'a> {
    id: u64,
    clock: Option<Duration>,
    now: Tick,
    gone: Arc<AtomicBool>,
    systems: &'a mut Systems,
    /// Who in the tree being looked at its wait came for.
    fired: Owners,
    /// [`BehaviorTreeTrace`]'s id, once one's been made: looked up once a run.
    traces: Option<ComponentId>,
    /// Trees asking to be looked at next run.
    next: &'a mut Vec<Entity>,
    /// An empty plan, its room kept from a tree's last: the next tree's.
    spare: Plan,
}

/// Ticks `agent`'s tree, if it's due and hasn't been this run.
fn tick_tree(world: &mut World, agent: Entity, run: &mut Run<'_>) {
    let Ok(mut entity) = world.get_entity_mut(agent) else {
        return;
    };
    // Asked again, as one ticked before it may have paused it.
    if entity.contains::<BehaviorTreePaused>() {
        return;
    }
    let Some(tree) = entity.get::<BehaviorTree>() else {
        return;
    };
    // Looked at once a run, however many times it comes up; and only
    // ticked if its wait is over.
    let due = tree.ran != run.id
        && match &tree.state {
            TreeState::Fresh => true,
            TreeState::Asleep(plan) => plan.due(entity.world(), run.clock, run.now, &mut run.fired),
            TreeState::Finished(_) => false,
        };
    if !due {
        return;
    }
    let Some(mut tree) = entity.get_mut::<BehaviorTree>() else {
        return;
    };
    // Its own doing: not a change for anything to see.
    let tree = tree.bypass_change_detection();
    tree.ran = run.id;
    tree.ticks += 1;
    let (tick, ids) = (tree.ticks, tree.ids);
    let Some(mut root) = tree.root.take() else {
        return;
    };
    let before = match std::mem::take(&mut tree.state) {
        TreeState::Asleep(plan) => Some(plan),
        TreeState::Fresh | TreeState::Finished(_) => None,
    };
    run.gone.store(false, Ordering::Relaxed);
    let waits = Waits::new(
        before.as_ref(),
        run.fired,
        tick,
        ids,
        std::mem::take(&mut run.spare),
    );
    let mut cx = Context::ticking(
        world,
        agent,
        &run.gone,
        run.clock.unwrap_or_default(),
        Some(run.systems),
        waits,
    );
    let status = root.tick(&mut cx);
    let (plan, ids) = cx.finish();
    put_back(world, agent, run, root, status, before, plan, ids);
}

/// Puts `agent`'s tree back after its tick, with how it went.
///
/// It sleeps on what it waits on now, having slept on `before`: or what it ran
/// is canceled, if it went as it ticked.
fn put_back(
    world: &mut World,
    agent: Entity,
    run: &mut Run<'_>,
    root: Behavior,
    status: Status,
    before: Option<Plan>,
    plan: Plan,
    ids: Owner,
) {
    restore(world, agent, run, root, status, before.as_ref(), plan, ids);
    // What it slept on, emptied, is the next tree's plan.
    if let Some(mut before) = before {
        before.clear();
        run.spare = before;
    }
}

/// Puts `agent`'s tree back, as [`put_back`] says, having slept on `before`.
fn restore(
    world: &mut World,
    agent: Entity,
    run: &mut Run<'_>,
    root: Behavior,
    status: Status,
    before: Option<&Plan>,
    plan: Plan,
    ids: Owner,
) {
    // Most often it waits on just what it did: then it's put to sleep as it's
    // put back, with nothing in the index to change.
    let same = status.outcome().is_none() && Wakes::same(before, &plan);
    let mut root = Some(root);
    let mut plan = Some(plan);
    // Back where it was, unless it went as it ticked.
    let traced = world.get_entity_mut(agent).ok().and_then(|mut entity| {
        let mut tree = entity.get_mut::<BehaviorTree>()?;
        let tree = tree.bypass_change_detection();
        if tree.root.is_some() {
            return None;
        }
        tree.root = root.take();
        tree.ids = ids;
        if same && let Some(plan) = plan.take() {
            if plan.is_poll() {
                run.next.push(agent);
            }
            tree.state = TreeState::Asleep(plan);
        }
        Some(run.traces.is_some_and(|traces| entity.contains_id(traces)))
    });
    let Some(traced) = traced else {
        // Gone as it ticked: what it slept on before, it no longer waits
        // on, and what it ran is cancelled.
        if let Some(before) = before {
            world.resource_mut::<Wakes>().forget(agent, before);
        }
        if let Some(mut root) = root {
            root.cancel(&mut Context::new(world, agent));
        }
        return;
    };
    if let Some(plan) = plan {
        let state = if let Some(outcome) = status.outcome() {
            let mut wakes = world.resource_mut::<Wakes>();
            if let Some(before) = before {
                wakes.forget(agent, before);
            }
            wakes.finished(agent);
            TreeState::Finished(outcome)
        } else {
            Wakes::sleep(world, agent, before, &plan, run.next);
            TreeState::Asleep(plan)
        };
        if let Some(mut tree) = world.get_mut::<BehaviorTree>(agent) {
            tree.bypass_change_detection().state = state;
        }
    }
    if let Some(outcome) = status.outcome() {
        world.trigger(BehaviorTreeFinished {
            entity: agent,
            outcome,
        });
    }
    if traced
        && let Some(traced) = world
            .get::<BehaviorTree>(agent)
            .and_then(|tree| tree.root.as_ref())
            .map(BehaviorTreeTrace::of)
        && let Some(mut trace) = world.get_mut::<BehaviorTreeTrace>(agent)
    {
        // Changed only when the tree is.
        trace.set_if_neq(traced);
    }
}

/// Keeps the change ticks sleeping trees and their systems hold from getting
/// too old.
///
/// A schedule does the same for its own systems.
pub(crate) fn check_ticks(
    check: On<CheckChangeTicks>,
    mut trees: Query<&mut BehaviorTree>,
    wakes: Option<ResMut<Wakes>>,
    systems: Option<ResMut<Systems>>,
) {
    for mut tree in &mut trees {
        if let TreeState::Asleep(plan) = &mut tree.bypass_change_detection().state {
            plan.check_ticks(*check);
        }
    }
    if let Some(mut wakes) = wakes {
        wakes.check_ticks(*check);
    }
    if let Some(mut systems) = systems {
        systems.check_ticks(*check);
    }
}
