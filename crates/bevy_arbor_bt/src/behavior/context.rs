use std::time::Duration;

use bevy::ecs::world::CommandQueue;
use bevy::prelude::*;

use crate::tree::Plan;

/// What a node runs with: the world to read, whose tree it's in, and commands
/// for what it changes.
///
/// Trees tick side by side, so the world is only read while they do. What a
/// node changes it queues as [`commands`](Self::commands), applied in the
/// order they were queued once its tree's tick is done.
pub struct Context<'a> {
    world: &'a World,
    agent: Entity,
    held: bool,
    interrupted: bool,
    /// The clock, read once for every tree ticked in a run.
    clock: Option<Duration>,
    /// What the tree changes, applied once its tick is done.
    queue: &'a mut CommandQueue,
    /// What the tree waits on, as its nodes say.
    plan: Plan,
    /// How many times anything has said what it waits on, this tick: whether
    /// a node or decorator said anything is whether this moved while it ran.
    said: u64,
    /// That something in it runs a system with the commands, as an `act`
    /// does: the tree is ticked again once they're applied, to see what it
    /// said.
    acted: bool,
    /// Which run of the trees this is: the same for each pass of one.
    run: u64,
}

impl<'a> Context<'a> {
    /// A context for `agent`'s tree, at `clock`, or with none, reading it when
    /// asked: what it changes goes in `queue`, for whoever made it to apply.
    pub(crate) fn new(
        world: &'a World,
        agent: Entity,
        clock: Option<Duration>,
        queue: &'a mut CommandQueue,
    ) -> Self {
        Self {
            world,
            agent,
            held: false,
            interrupted: false,
            clock,
            queue,
            plan: Plan::default(),
            said: 0,
            acted: false,
            run: 0,
        }
    }

    /// The same, in run `run` of the trees.
    pub(crate) fn in_run(mut self, run: u64) -> Self {
        self.run = run;
        self
    }

    /// Which run of the trees this is: each pass of one, to hear what its acts
    /// said, is the same run.
    pub(crate) fn run(&self) -> u64 {
        self.run
    }

    /// What the tree waits on, as its nodes said, and whether it's to be ticked
    /// again once its commands are applied.
    pub(crate) fn finish(self) -> (Plan, bool) {
        (self.plan, self.acted)
    }

    /// Asks for the tree to be ticked again once its commands are applied,
    /// within the same run: for a node whose answer comes from them.
    pub(crate) fn acted(&mut self) {
        self.acted = true;
    }

    /// The entity whose tree this is.
    #[must_use]
    pub fn agent(&self) -> Entity {
        self.agent
    }

    /// The world, to read.
    #[must_use]
    pub fn world(&self) -> &'a World {
        self.world
    }

    /// Commands for what the node changes: spawning, inserting, removing.
    /// They're applied once the tree's tick is done, in the order queued, so
    /// what they do isn't seen until its next tick.
    pub fn commands(&mut self) -> Commands<'_, '_> {
        Commands::new(self.queue, self.world)
    }

    /// The schedule's time since it began.
    ///
    /// Zero without Bevy's `Time`, which `TimePlugin` adds: timers then never
    /// run out, and `TreePlugin` warns about it.
    #[must_use]
    pub fn now(&self) -> Duration {
        self.clock
            .or_else(|| self.world.get_resource::<Time>().map(Time::elapsed))
            .unwrap_or_default()
    }

    /// Asks for the tree to be ticked again once its clock gets to `at`: what a
    /// timer does, so the tree can sleep until then.
    pub fn wake_at(&mut self, at: Duration) {
        self.said += 1;
        self.plan.wake_at(at);
    }

    /// Asks for the tree to be ticked again once `T` on `entity` changes, comes
    /// or goes: what a node waiting on a component does, so the tree can sleep
    /// until then.
    pub fn watch<T: Component>(&mut self, entity: Entity) {
        self.said += 1;
        self.plan.watch::<T>(self.world, entity);
    }

    /// Asks for the tree to be ticked again next tick, whatever happens: what
    /// a node that has to look every tick does. A running node, or a decorator
    /// asked, that says nothing it waits on is asked again next tick anyway.
    pub fn poll(&mut self) {
        self.said += 1;
        self.plan.poll();
    }

    /// Says nothing needs waking for it: nothing it reads changes but by its
    /// own branch starting or ending. A decorator saying nothing else is
    /// otherwise asked every tick; one at rest, as an idle cooldown, says this.
    pub fn wait_on_nothing(&mut self) {
        self.said += 1;
    }

    /// How many times anything has said what it waits on so far.
    pub(crate) fn said(&self) -> u64 {
        self.said
    }

    /// Asks not to be stopped by a `Select` or `RepeatWhile`, for this tick.
    ///
    /// Call it on every tick you're asking; a child's hold is passed up on its
    /// own (see [`Behavior::skip`](crate::behavior::Behavior::skip) for
    /// ticks a child isn't ticked). Only force gets past this: a `Timeout`, an
    /// `All`, `Any` or `Race` settling, anything above being canceled, or the
    /// tree going.
    pub fn hold(&mut self) {
        self.held = true;
    }

    /// Whether what's above wants this stopped, and it's only still ticking
    /// because something in it holds. Wind down, and stop holding.
    ///
    /// Don't start anything new meanwhile: a node over children goes on with
    /// the one running, and once it ends, ends too, as the built-in ones do.
    #[must_use]
    pub fn interrupted(&self) -> bool {
        self.interrupted
    }

    /// Runs `tick` asking it to stop.
    pub(crate) fn interrupting<T>(&mut self, tick: impl FnOnce(&mut Self) -> T) -> T {
        self.interrupted_as(true, tick)
    }

    /// Runs `tick` not asking it to stop, whatever's above: what `Commit` does.
    pub(crate) fn sheltering<T>(&mut self, tick: impl FnOnce(&mut Self) -> T) -> T {
        self.interrupted_as(false, tick)
    }

    /// Runs `tick` with [`interrupted`](Self::interrupted) as `interrupted`.
    fn interrupted_as<T>(&mut self, interrupted: bool, tick: impl FnOnce(&mut Self) -> T) -> T {
        let outer = std::mem::replace(&mut self.interrupted, interrupted);
        let result = tick(self);
        self.interrupted = outer;
        result
    }

    /// Runs `tick`, and says whether it held, keeping that from what's around
    /// it.
    pub(crate) fn holding<T>(&mut self, tick: impl FnOnce(&mut Self) -> T) -> (T, bool) {
        let outer = std::mem::take(&mut self.held);
        let result = tick(self);
        (result, std::mem::replace(&mut self.held, outer))
    }
}
