use std::time::Duration;

use bevy::ecs::change_detection::{CheckChangeTicks, Tick};
use bevy::ecs::world::CommandQueue;
use bevy::prelude::*;
use bevy::tasks::{ComputeTaskPool, ParallelSliceMut, TaskPool};

use super::components::{BehaviorTree, TreePaused};
use super::events::TreeFinished;
use super::trace::TreeTrace;
use super::wake::Plan;
use crate::behavior::{Behavior, Context, Status};
use crate::task::Task;

/// How many times one run ticks a tree whose acts answered: each act in a row
/// takes one. What's left over goes on next run.
const PASSES: usize = 16;

/// The fewest trees ticked on one thread: fewer aren't worth handing out.
const CHUNK: usize = 32;

/// A tree out of its component for a tick, and what the tick left.
struct Ticked {
    agent: Entity,
    root: Behavior,
    status: Status,
    plan: Plan,
    acted: bool,
    queue: CommandQueue,
}

impl Ticked {
    /// Ticks it, reading `world`, at `clock`, in `run`.
    fn tick(&mut self, world: &World, clock: Option<Duration>, run: u64) {
        let mut cx = Context::new(world, self.agent, clock, &mut self.queue).in_run(run);
        self.status = self.root.tick(&mut cx);
        (self.plan, self.acted) = cx.finish();
    }
}

/// Ticks every tree that's woken, side by side.
///
/// Each one's root comes out of its component for the tick, and reads the
/// world: what it changes, it queues as commands. Once every tree has ticked,
/// the roots go back, and each tree's commands are applied in turn, in the
/// order the trees were found. A tree whose acts ran with them is ticked again,
/// to see what they said.
///
/// What each tick waits on is kept on the tree, and it sleeps until that
/// comes. Then tasks are told how long they've run.
pub(crate) fn tick(
    world: &mut World,
    trees: &mut QueryState<(Entity, &BehaviorTree), Without<TreePaused>>,
    tasks: &mut QueryState<&mut Task>,
    mut due: Local<Vec<Entity>>,
    mut runs: Local<u64>,
) {
    *runs += 1;
    let run = *runs;
    // Nothing in a tick moves the clock, so it's read once for every tree.
    let clock = world.get_resource::<Time>().map(Time::elapsed);
    let now = world.change_tick();
    due.extend(
        trees
            .iter(world)
            .filter(|(_, tree)| tree.due(world, clock, now))
            .map(|(agent, _)| agent),
    );
    for _ in 0..PASSES {
        if due.is_empty() {
            break;
        }
        let mut ticked: Vec<Ticked> = due
            .drain(..)
            .filter_map(|agent| take(world, agent))
            .collect();
        let shared: &World = world;
        if ticked.len() <= CHUNK {
            for tree in &mut ticked {
                tree.tick(shared, clock, run);
            }
        } else {
            let pool = ComputeTaskPool::get_or_init(TaskPool::default);
            let chunk = ticked.len().div_ceil(pool.thread_num().max(1)).max(CHUNK);
            ticked.par_chunk_map_mut(pool, chunk, |_, chunk| {
                for tree in chunk {
                    tree.tick(shared, clock, run);
                }
            });
        }
        // Every root back first: a command may take any tree away, which
        // cancels what it runs.
        let mut ticked: Vec<_> = ticked
            .into_iter()
            .map(|tree| {
                let unborn = tree.plan.unborn();
                put_back(world, tree.agent, tree.root, tree.status, tree.plan);
                (tree.agent, tree.status, tree.acted, unborn, tree.queue)
            })
            .collect();
        for (agent, status, acted, unborn, queue) in &mut ticked {
            let before = world.change_tick();
            if !queue.is_empty() {
                queue.apply(world);
            }
            if *unborn {
                settle(world, *agent, before);
            }
            if let Some(outcome) = status.outcome() {
                world.trigger(TreeFinished {
                    entity: *agent,
                    outcome,
                });
            } else if *acted {
                due.push(*agent);
            }
            trace(world, *agent);
        }
    }
    // Ticked as many times as a run does: the rest next run.
    due.clear();
    if let Some(clock) = clock {
        for mut task in tasks.iter_mut(world) {
            // Time passing isn't a change a task system should react to.
            task.bypass_change_detection().advance(clock);
        }
    }
    world.flush();
}

/// `agent`'s root, out of its tree for a tick: none if it's paused, finished,
/// or gone.
fn take(world: &mut World, agent: Entity) -> Option<Ticked> {
    let mut entity = world.get_entity_mut(agent).ok()?;
    // Asked again, as commands before may have paused it.
    if entity.contains::<TreePaused>() {
        return None;
    }
    let mut tree = entity.get_mut::<BehaviorTree>()?;
    if tree.outcome.is_some() {
        return None;
    }
    let root = tree.root.take()?;
    Some(Ticked {
        agent,
        root,
        status: Status::Running,
        plan: Plan::default(),
        acted: false,
        queue: CommandQueue::default(),
    })
}

/// Puts `root` back in `agent`'s tree, with how its tick went.
fn put_back(world: &mut World, agent: Entity, root: Behavior, status: Status, plan: Plan) {
    // Only read while it was out, so it's still there.
    if let Some(mut tree) = world.get_mut::<BehaviorTree>(agent) {
        tree.root = Some(root);
        tree.outcome = status.outcome();
        tree.plan = plan;
        tree.fresh = false;
    }
}

/// Settles what `agent`'s tree watches, now its commands, applied from the
/// change tick `before`, are.
fn settle(world: &mut World, agent: Entity, before: Tick) {
    let Some(mut plan) = world
        .get_mut::<BehaviorTree>(agent)
        .map(|mut tree| std::mem::take(&mut tree.bypass_change_detection().plan))
    else {
        return;
    };
    plan.settle(world, before);
    if let Some(mut tree) = world.get_mut::<BehaviorTree>(agent) {
        tree.bypass_change_detection().plan = plan;
    }
}

/// Brings `agent`'s trace up to date, if it has one.
fn trace(world: &mut World, agent: Entity) {
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

/// Keeps the change ticks trees hold from getting too old, as a schedule does
/// for its own systems.
pub(crate) fn check_ticks(check: On<CheckChangeTicks>, mut trees: Query<&mut BehaviorTree>) {
    for mut tree in &mut trees {
        tree.bypass_change_detection().plan.check_ticks(*check);
    }
}
