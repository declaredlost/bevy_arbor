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

/// The fewest trees ticked on one thread: a tree's tick is around a tenth of a
/// microsecond, so fewer aren't worth handing out.
const CHUNK: usize = 256;

/// A tree out of its component for a tick, and what the tick left.
pub(crate) struct Ticked {
    agent: Entity,
    root: Option<Behavior>,
    status: Status,
    plan: Plan,
    acted: bool,
    /// Whether it has a [`TreeTrace`] to bring up to date.
    traced: bool,
}

impl Ticked {
    /// Ticks it, reading `world`, at `clock`, in `run`, its commands going in
    /// `queue`.
    fn tick(&mut self, world: &World, clock: Option<Duration>, run: u64, queue: &mut CommandQueue) {
        let Some(root) = &mut self.root else {
            return;
        };
        let mut cx = Context::new(world, self.agent, clock, queue).in_run(run);
        self.status = root.tick(&mut cx);
        (self.plan, self.acted) = cx.finish();
    }
}

/// What a tree's tick left, once its root is back: for after its commands are
/// applied.
pub(crate) struct Left {
    agent: Entity,
    status: Status,
    acted: bool,
    unborn: bool,
    traced: bool,
}

/// Ticks every tree that's woken, side by side.
///
/// Each one's root comes out of its component for the tick, and reads the
/// world: what it changes, it queues as commands. Trees are ticked in chunks,
/// one to a thread, each chunk's commands in one queue, in order. Once every
/// tree has ticked, the roots go back, and the chunks' commands are applied in
/// turn, so every tree's are, in the order the trees were found. A tree whose
/// acts ran with them is ticked again, to see what they said.
///
/// What each tick waits on is kept on the tree, and it sleeps until that
/// comes. Then tasks are told how long they've run.
pub(crate) fn tick(
    world: &mut World,
    trees: &mut QueryState<(Entity, &BehaviorTree), Without<TreePaused>>,
    roots: &mut QueryState<(&mut BehaviorTree, Has<TreeTrace>), Without<TreePaused>>,
    tasks: &mut QueryState<&mut Task>,
    mut due: Local<Vec<Entity>>,
    mut ticked: Local<Vec<Ticked>>,
    mut left: Local<Vec<Left>>,
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
        ticked.extend(due.drain(..).filter_map(|agent| take(world, roots, agent)));
        let shared: &World = world;
        let (chunk, mut queues) = if ticked.len() <= CHUNK {
            let mut queue = CommandQueue::default();
            for tree in &mut *ticked {
                tree.tick(shared, clock, run, &mut queue);
            }
            (ticked.len().max(1), vec![queue])
        } else {
            let pool = ComputeTaskPool::get_or_init(TaskPool::default);
            let chunk = ticked.len().div_ceil(pool.thread_num().max(1)).max(CHUNK);
            let queues = ticked.par_chunk_map_mut(pool, chunk, |_, trees| {
                let mut queue = CommandQueue::default();
                for tree in trees {
                    tree.tick(shared, clock, run, &mut queue);
                }
                queue
            });
            (chunk, queues)
        };
        // Every root back first: a command may take any tree away, which
        // cancels what it runs.
        left.extend(ticked.drain(..).map(|tree| {
            let unborn = tree.plan.unborn();
            if let Ok((mut back, _)) = roots.get_mut(world, tree.agent) {
                back.root = tree.root;
                back.outcome = tree.status.outcome();
                back.plan = tree.plan;
                back.fresh = false;
            }
            Left {
                agent: tree.agent,
                status: tree.status,
                acted: tree.acted,
                unborn,
                traced: tree.traced,
            }
        }));
        for (queue, trees) in queues.iter_mut().zip(left.chunks(chunk)) {
            let before = world.change_tick();
            queue.apply(world);
            for tree in trees {
                if tree.unborn {
                    settle(world, tree.agent, before);
                }
                if let Some(outcome) = tree.status.outcome() {
                    world.trigger(TreeFinished {
                        entity: tree.agent,
                        outcome,
                    });
                } else if tree.acted {
                    due.push(tree.agent);
                }
                if tree.traced {
                    trace(world, tree.agent);
                }
            }
        }
        left.clear();
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

/// `agent`'s root, out of its tree for a tick: none if it's paused (asked
/// again, as commands before may have paused it), finished, or gone.
fn take(
    world: &mut World,
    roots: &mut QueryState<(&mut BehaviorTree, Has<TreeTrace>), Without<TreePaused>>,
    agent: Entity,
) -> Option<Ticked> {
    let (mut tree, traced) = roots.get_mut(world, agent).ok()?;
    if tree.outcome.is_some() {
        return None;
    }
    let root = Some(tree.root.take()?);
    Some(Ticked {
        agent,
        root,
        status: Status::Running,
        plan: Plan::default(),
        acted: false,
        traced,
    })
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

/// Brings `agent`'s trace up to date.
fn trace(world: &mut World, agent: Entity) {
    if let Some(traced) = world
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

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use crate::behavior::{Behave, Context, Outcome, Status};
    use crate::prelude::*;
    use crate::testing::*;

    /// The agents whose commands were applied, in order.
    #[derive(Resource, Default)]
    struct Applied(Vec<Entity>);

    /// Queues its agent, once.
    struct Note;

    impl Behave for Note {
        fn tick(&mut self, cx: &mut Context) -> Status {
            let agent = cx.agent();
            cx.commands()
                .queue(move |world: &mut World| world.resource_mut::<Applied>().0.push(agent));
            Status::Success
        }

        fn end(&mut self, _cx: &mut Context, _outcome: Option<Outcome>) {}
    }

    #[test]
    fn commands_are_applied_in_the_order_trees_were_found_across_chunks() {
        let mut app = app();
        app.app_mut().init_resource::<Applied>();
        for _ in 0..500 {
            agent(&mut app, behavior_tree! { @ Note });
        }
        let found: Vec<Entity> = app
            .world_mut()
            .query_filtered::<Entity, With<BehaviorTree>>()
            .iter(app.world())
            .collect();
        app.tick();
        assert_eq!(app.world().resource::<Applied>().0, found);
    }
}
