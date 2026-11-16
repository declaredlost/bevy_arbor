use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use bevy::prelude::*;

use crate::behavior::system::Systems;
use crate::tree::{NODE, Owner, Owners, Plan};

/// What a context made outside a tree's tick reads for whether it's gone.
static STAYS: AtomicBool = AtomicBool::new(false);

/// What a node runs with: the world, and whose tree it's in.
pub struct Context<'a> {
    world: &'a mut World,
    agent: Entity,
    /// The tree's clock, read once for every tree ticked in a run.
    now: Duration,
    /// The systems decorators and acts run, out of the world meanwhile.
    systems: &'a mut Systems,
    /// Set once the tree being ticked is taken away or replaced.
    gone: &'a AtomicBool,
    /// Whether what's ticking asked not to be stopped.
    held: bool,
    /// Whether what's above wants what's ticking stopped.
    interrupted: bool,
    /// What the tree waited on, and what it waits on now.
    waits: Waits<'a>,
}

/// What a tree's tick waited on, who's asked now, and what it waits on next.
///
/// A tree's tick keeps it for its nodes and decorator parts to say what they
/// wait on, each as its own.
pub(crate) struct Waits<'a> {
    /// What its last tick waited on: `None` if it's new or restarted, and
    /// asks everything.
    before: Option<&'a Plan>,
    /// Who its wait came for.
    fired: Owners,
    /// What it waits on now, as its nodes say.
    plan: Plan,
    /// The decorator parts whose answers stood, so wait on what they did.
    kept: Owners,
    /// Who's being asked: a decorator part, or a node.
    owner: Owner,
    /// How many times anything has said what it waits on, this tick.
    ///
    /// Whether a node or decorator said anything is whether this moved while it
    /// ran.
    said: u64,
    /// How many times the tree's been ticked, this one included.
    tick: u64,
    /// The last id given a decorator part.
    ids: Owner,
}

impl<'a> Waits<'a> {
    /// A tree's tick, having waited on `before`, its wait come for `fired`.
    ///
    /// It's the tree's `tick`th, its last part's id `ids`; what it waits on
    /// next goes into `plan`, an empty one whose room is reused.
    pub(crate) fn new(
        before: Option<&'a Plan>,
        fired: Owners,
        tick: u64,
        ids: Owner,
        plan: Plan,
    ) -> Self {
        Self {
            before,
            fired,
            plan,
            kept: Owners::EMPTY,
            owner: NODE,
            said: 0,
            tick,
            ids,
        }
    }

    /// Outside a tree's tick, as it's canceled: asks everything.
    fn none() -> Self {
        Self::new(None, Owners::EMPTY, 0, NODE, Plan::default())
    }
}

impl<'a> Context<'a> {
    /// A context for canceling, outside a tree's tick, running `systems`.
    pub(crate) fn new(world: &'a mut World, agent: Entity, systems: &'a mut Systems) -> Self {
        let now = world
            .get_resource::<Time>()
            .map(Time::elapsed)
            .unwrap_or_default();
        Self::ticking(world, agent, &STAYS, now, systems, Waits::none())
    }

    /// A context for ticking `agent`'s tree, `gone` set if it goes meanwhile.
    ///
    /// It's at `now`, and runs `systems`.
    pub(crate) fn ticking(
        world: &'a mut World,
        agent: Entity,
        gone: &'a AtomicBool,
        now: Duration,
        systems: &'a mut Systems,
        waits: Waits<'a>,
    ) -> Self {
        Self {
            world,
            agent,
            now,
            systems,
            gone,
            held: false,
            interrupted: false,
            waits,
        }
    }

    /// What the tree waits on, as its nodes said, and the last id given a
    /// decorator part.
    pub(crate) fn finish(self) -> (Plan, Owner) {
        let Waits {
            before,
            mut plan,
            kept,
            ids,
            ..
        } = self.waits;
        if let Some(before) = before {
            plan.keep(before, kept);
        }
        (plan, ids)
    }

    /// An id for a decorator part, its own in its tree.
    pub(crate) fn new_id(&mut self) -> Owner {
        self.waits.ids += 1;
        self.waits.ids
    }

    /// How many times the tree has been ticked, this one included.
    pub(crate) fn tick_count(&self) -> u64 {
        self.waits.tick
    }

    /// Whether what `owner` waited on came, or the tree asks everything.
    ///
    /// A tree asks everything when it's new or restarted.
    pub(crate) fn woken(&self, owner: Owner) -> bool {
        self.waits.before.is_none() || self.waits.fired.contains(owner)
    }

    /// Runs `ask` as `owner`: what it says it waits on is its own.
    pub(crate) fn asking<T>(&mut self, owner: Owner, ask: impl FnOnce(&mut Self) -> T) -> T {
        let outer = std::mem::replace(&mut self.waits.owner, owner);
        let result = ask(self);
        self.waits.owner = outer;
        result
    }

    /// Carries over what `owner` waited on last tick, so it waits on the same.
    ///
    /// It isn't asked again, its answer standing.
    pub(crate) fn keep(&mut self, owner: Owner) {
        self.waits.said += 1;
        self.waits.kept.insert(owner);
    }

    /// The world, and the systems decorators and acts run, out of it.
    pub(crate) fn world_and_systems(&mut self) -> (&mut World, &mut Systems) {
        (self.world, self.systems)
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
    /// run out, and `BehaviorTreePlugin` warns about it.
    pub fn now(&self) -> Duration {
        self.now
    }

    /// Asks for the tree to be ticked again once its clock gets to `at`.
    ///
    /// What a timer does, so the tree can sleep until then.
    pub fn wake_at(&mut self, at: Duration) {
        self.waits.said += 1;
        self.waits.plan.wake_at(self.waits.owner, at);
    }

    /// Asks for the tree to be ticked again once `T` on `entity` changes, comes
    /// or goes.
    ///
    /// What a node waiting on a component does, so the tree can sleep until
    /// then.
    pub fn watch<T: Component>(&mut self, entity: Entity) {
        self.waits.said += 1;
        self.waits
            .plan
            .watch::<T>(self.waits.owner, self.world, entity);
    }

    /// Asks for the tree to be ticked again next tick, whatever happens.
    ///
    /// What a node that has to look every tick does. A running node, or a
    /// decorator asked, that says nothing it waits on is asked again next tick
    /// anyway.
    pub fn poll(&mut self) {
        self.waits.said += 1;
        self.waits.plan.poll(self.waits.owner);
    }

    /// Says nothing needs waking for it, as nothing it reads changes by itself.
    ///
    /// Its own branch starting or ending is all that changes what it reads. A
    /// decorator saying nothing else is otherwise asked every tick; one at
    /// rest, as an idle cooldown, says this.
    pub fn wait_on_nothing(&mut self) {
        self.waits.said += 1;
    }

    /// How many times anything has said what it waits on so far.
    pub(crate) fn said(&self) -> u64 {
        self.waits.said
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

    /// Whether what's above wants this stopped, while something in it holds.
    ///
    /// It's only still ticking because of that hold. Wind down, and stop
    /// holding.
    ///
    /// Don't start anything new meanwhile: a node over children goes on with
    /// the one running, and once it ends, ends too, as the built-in ones do.
    #[must_use]
    pub fn is_interrupted(&self) -> bool {
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

    /// Runs `tick` with [`is_interrupted`](Self::is_interrupted) as
    /// `interrupted`.
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
