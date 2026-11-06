use std::time::Duration;

use bevy::prelude::*;

/// What a node runs with: the world, and whose tree it's in.
pub struct Context<'a> {
    world: &'a mut World,
    agent: Entity,
    held: bool,
    interrupted: bool,
}

impl<'a> Context<'a> {
    pub(crate) fn new(world: &'a mut World, agent: Entity) -> Self {
        Self {
            world,
            agent,
            held: false,
            interrupted: false,
        }
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
        self.world.resource::<Time>().elapsed()
    }

    /// Asks not to be stopped by a `Select` or `While`, for this tick. Call it
    /// on every tick you're asking; a child's hold is passed up on its own (see
    /// [`Behaviour::skip`](crate::behaviour::Behaviour::skip) for ticks a child
    /// isn't ticked). Only force gets past this: a `Timeout`, or the tree
    /// going.
    pub fn hold(&mut self) {
        self.held = true;
    }

    /// Whether what's above wants this stopped, and it's only still ticking
    /// because something in it holds. Wind down, and stop holding.
    #[must_use]
    pub fn interrupted(&self) -> bool {
        self.interrupted
    }

    /// Runs `tick` asking it to stop.
    pub(crate) fn interrupting<T>(&mut self, tick: impl FnOnce(&mut Self) -> T) -> T {
        let outer = std::mem::replace(&mut self.interrupted, true);
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
