use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use bevy::prelude::*;

use crate::behaviour::Systems;
use crate::tree::Plan;

/// What a context made outside a tree's tick reads for whether it's gone.
static STAYS: AtomicBool = AtomicBool::new(false);

/// What a node runs with: the world, and whose tree it's in.
pub struct Context<'a> {
    world: &'a mut World,
    agent: Entity,
    held: bool,
    interrupted: bool,
    /// Set once the tree being ticked is taken away or replaced.
    gone: &'a AtomicBool,
    /// The clock, read once for every tree ticked in a run.
    clock: Option<Duration>,
    /// The systems guards and acts run, out of the world while trees tick.
    systems: Option<&'a mut Systems>,
    /// What the tree waits on, as its nodes say.
    plan: Plan,
}

impl<'a> Context<'a> {
    /// A context for cancelling, outside a tree's tick.
    pub(crate) fn new(world: &'a mut World, agent: Entity) -> Self {
        Self::ticking(world, agent, &STAYS, None, None)
    }

    /// A context for ticking `agent`'s tree, `gone` set if it goes meanwhile:
    /// at `clock`, or with none, reading it when asked, and running `systems`,
    /// or with none, the world's.
    pub(crate) fn ticking(
        world: &'a mut World,
        agent: Entity,
        gone: &'a AtomicBool,
        clock: Option<Duration>,
        systems: Option<&'a mut Systems>,
    ) -> Self {
        Self {
            world,
            agent,
            held: false,
            interrupted: false,
            gone,
            clock,
            systems,
            plan: Plan::default(),
        }
    }

    /// What the tree waits on, as its nodes said.
    pub(crate) fn finish(self) -> Plan {
        self.plan
    }

    /// The world, and the systems guards and acts run, if they're out of it.
    pub(crate) fn world_and_systems(&mut self) -> (&mut World, Option<&mut Systems>) {
        (self.world, self.systems.as_deref_mut())
    }

    /// The entity whose tree this is.
    #[must_use]
    pub fn agent(&self) -> Entity {
        self.agent
    }

    /// The world, to read.
    #[must_use]
    pub fn world(&self) -> &World {
        self.world
    }

    /// The world, to change: spawn, run systems, write components.
    pub fn world_mut(&mut self) -> &mut World {
        self.world
    }

    /// The schedule's time since it began.
    #[must_use]
    pub fn now(&self) -> Duration {
        self.clock
            .unwrap_or_else(|| self.world.resource::<Time>().elapsed())
    }

    /// Asks for the tree to be ticked again once its clock gets to `at`: what a
    /// timer does, so the tree can sleep until then.
    pub fn wake_at(&mut self, at: Duration) {
        self.plan.wake_at(at);
    }

    /// Asks for the tree to be ticked again once `T` on `entity` changes, comes
    /// or goes: what a node waiting on a component does, so the tree can sleep
    /// until then.
    pub fn watch<T: Component>(&mut self, entity: Entity) {
        self.plan.watch::<T>(self.world, entity);
    }

    /// Asks for the tree to be ticked again next tick, whatever happens: what
    /// a node that has to look every tick does. One that doesn't say what it
    /// waits on does this (see
    /// [`Behave::polls`](crate::behaviour::Behave::polls)).
    pub fn poll(&mut self) {
        self.plan.poll();
    }

    /// Asks not to be stopped by a `Select` or `While`, for this tick.
    ///
    /// Call it on every tick you're asking; a child's hold is passed up on its
    /// own (see [`Behaviour::skip`](crate::behaviour::Behaviour::skip) for
    /// ticks a child isn't ticked). Only force gets past this: a `Timeout`, an
    /// `All`, `Any` or `Race` settling, anything above being cancelled, or the
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

    /// Whether the tree being ticked has been taken away or replaced.
    ///
    /// By something it ran: nothing more of it runs then.
    pub(crate) fn gone(&self) -> bool {
        self.gone.load(Ordering::Relaxed)
    }

    /// Runs `tick`, and says whether it held, keeping that from what's around
    /// it.
    pub(crate) fn holding<T>(&mut self, tick: impl FnOnce(&mut Self) -> T) -> (T, bool) {
        let outer = std::mem::take(&mut self.held);
        let result = tick(self);
        (result, std::mem::replace(&mut self.held, outer))
    }
}
