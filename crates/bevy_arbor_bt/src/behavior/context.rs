use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use bevy::prelude::*;

use crate::behavior::system::Systems;
use crate::tree::{NODE, Owner, Plan};

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
    /// The systems decorators and acts run, out of the world while trees tick.
    systems: Option<&'a mut Systems>,
    /// What the tree waits on, as its nodes say.
    plan: Plan,
    /// How many times anything has said what it waits on, this tick: whether
    /// a node or decorator said anything is whether this moved while it ran.
    said: u64,
    /// Who's being asked: a decorator part, or a node.
    owner: Owner,
    /// What woke the tree.
    woken: Woken<'a>,
}

/// What woke a tree: who in it its wait came for, what it waited on, and
/// what it knows of its decorator parts.
pub(crate) struct Woken<'a> {
    /// Who its wait came for.
    pub(crate) fired: &'a [Owner],
    /// What its last tick waited on: `None` if it's new or restarted, and
    /// asks everything.
    pub(crate) before: Option<&'a Plan>,
    /// How many times it's been ticked, this one included.
    pub(crate) tick: u64,
    /// The last id given a decorator part.
    pub(crate) ids: Owner,
}

impl Woken<'_> {
    /// A tree woken for everything: new, restarted, or ended outside a tick.
    pub(crate) const ALL: Woken<'static> = Woken {
        fired: &[],
        before: None,
        tick: 0,
        ids: NODE,
    };
}

impl<'a> Context<'a> {
    /// A context for canceling, outside a tree's tick.
    pub(crate) fn new(world: &'a mut World, agent: Entity) -> Self {
        Self::ticking(world, agent, &STAYS, None, None, Woken::ALL)
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
        woken: Woken<'a>,
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
            said: 0,
            owner: NODE,
            woken,
        }
    }

    /// What the tree waits on, as its nodes said, and the last id given a
    /// decorator part.
    pub(crate) fn finish(self) -> (Plan, Owner) {
        (self.plan, self.woken.ids)
    }

    /// An id for a decorator part, its own in its tree.
    pub(crate) fn new_id(&mut self) -> Owner {
        self.woken.ids += 1;
        self.woken.ids
    }

    /// How many times the tree has been ticked, this one included.
    pub(crate) fn tick_count(&self) -> u64 {
        self.woken.tick
    }

    /// Whether what `owner` waited on came: or the tree is new or restarted,
    /// and asks everything.
    pub(crate) fn woken(&self, owner: Owner) -> bool {
        self.woken.before.is_none() || self.woken.fired.contains(&owner)
    }

    /// Runs `ask` as `owner`: what it says it waits on is its own.
    pub(crate) fn asking<T>(&mut self, owner: Owner, ask: impl FnOnce(&mut Self) -> T) -> T {
        let outer = std::mem::replace(&mut self.owner, owner);
        let result = ask(self);
        self.owner = outer;
        result
    }

    /// Carries over what `owner` waited on last tick: it isn't asked again,
    /// its answer standing, so it waits on the same.
    pub(crate) fn keep(&mut self, owner: Owner) {
        self.said += 1;
        if let Some(before) = self.woken.before {
            self.plan.keep(before, owner);
        }
    }

    /// The world, and the systems decorators and acts run, if they're out of
    /// it.
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
    ///
    /// Zero without Bevy's `Time`, which `TimePlugin` adds: timers then never
    /// run out, and `TreePlugin` warns about it.
    pub fn now(&self) -> Duration {
        self.clock
            .or_else(|| self.world.get_resource::<Time>().map(Time::elapsed))
            .unwrap_or_default()
    }

    /// Asks for the tree to be ticked again once its clock gets to `at`: what a
    /// timer does, so the tree can sleep until then.
    pub fn wake_at(&mut self, at: Duration) {
        self.said += 1;
        self.plan.wake_at(self.owner, at);
    }

    /// Asks for the tree to be ticked again once `T` on `entity` changes, comes
    /// or goes: what a node waiting on a component does, so the tree can sleep
    /// until then.
    pub fn watch<T: Component>(&mut self, entity: Entity) {
        self.said += 1;
        self.plan.watch::<T>(self.owner, self.world, entity);
    }

    /// Asks for the tree to be ticked again next tick, whatever happens: what
    /// a node that has to look every tick does. A running node, or a decorator
    /// asked, that says nothing it waits on is asked again next tick anyway.
    pub fn poll(&mut self) {
        self.said += 1;
        self.plan.poll(self.owner);
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
