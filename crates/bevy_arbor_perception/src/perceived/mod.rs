//! What an agent perceives and remembers by each sense, how it stands toward
//! it, and what's triggered as that changes.

mod attitude;
mod candidate;
mod commit;
mod events;
mod memory;

use std::fmt;
use std::marker::PhantomData;
use std::time::Duration;

use bevy::prelude::*;

pub use self::attitude::{Attitude, Team};
pub use self::candidate::{Candidate, Candidates};
pub(crate) use self::commit::{commit, settle};
pub use self::events::{Forgotten, Lost, Noticed};
pub(crate) use self::memory::{clear, forget};
use crate::sense::Sense;

/// On an agent: what it perceives and remembers by the sense `S`, from
/// itself and its sensors.
///
/// Changes only when something is noticed, lost or forgotten, or its attitude
/// changes, never as things move: so a tree whose decorators read it sleeps in
/// between, and one reading `Perceived<Sight>` isn't woken by a noise. While
/// something is active (in view, for sight), read its `Transform` for where
/// it is now.
///
/// It remembers for [`Sense::MEMORY`], unless made with
/// [`remembering`](Self::remembering): what's no longer active fades that
/// long after it was last sensed, and is then forgotten.
#[derive(Component, Reflect)]
#[reflect(Component, Default)]
pub struct Perceived<S: Sense> {
    percepts: Vec<Percept>,
    memory: Duration,
    /// When something next fades: `None` to work it out again.
    #[reflect(ignore)]
    pub(crate) next_fade: Option<Duration>,
    #[reflect(ignore)]
    sense: PhantomData<fn() -> S>,
}

impl<S: Sense> Default for Perceived<S> {
    fn default() -> Self {
        Self::remembering(S::MEMORY)
    }
}

impl<S: Sense> Clone for Perceived<S> {
    fn clone(&self) -> Self {
        Self {
            percepts: self.percepts.clone(),
            memory: self.memory,
            next_fade: self.next_fade,
            sense: PhantomData,
        }
    }
}

impl<S: Sense> PartialEq for Perceived<S> {
    fn eq(&self, other: &Self) -> bool {
        self.percepts == other.percepts && self.memory == other.memory
    }
}

impl<S: Sense> fmt::Debug for Perceived<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(&format!("Perceived<{}>", S::short_type_path()))
            .field("percepts", &self.percepts)
            .field("memory", &self.memory)
            .finish_non_exhaustive()
    }
}

impl<S: Sense> Perceived<S> {
    /// Nothing perceived yet, remembering for `memory`: `Duration::MAX` is for
    /// good.
    #[must_use]
    pub fn remembering(memory: Duration) -> Self {
        Self {
            percepts: Vec::new(),
            memory,
            next_fade: None,
            sense: PhantomData,
        }
    }

    /// How long it remembers.
    #[must_use]
    pub fn memory(&self) -> Duration {
        self.memory
    }

    /// Everything perceived or remembered.
    pub fn iter(&self) -> impl Iterator<Item = &Percept> {
        self.percepts.iter()
    }

    /// What's active now: in view, for sight.
    pub fn active(&self) -> impl Iterator<Item = &Percept> {
        self.percepts.iter().filter(|percept| percept.active)
    }

    /// `entity`, if it's perceived or remembered.
    #[must_use]
    pub fn get(&self, entity: Entity) -> Option<&Percept> {
        self.percepts
            .iter()
            .find(|percept| percept.entity == entity)
    }

    /// Whether nothing is perceived or remembered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.percepts.is_empty()
    }

    /// `entity`, added if it isn't there, and whether it was added.
    pub(crate) fn entry(&mut self, entity: Entity, attitude: Attitude) -> (&mut Percept, bool) {
        let found = self
            .percepts
            .iter()
            .position(|percept| percept.entity == entity);
        let (index, added) = if let Some(index) = found {
            (index, false)
        } else {
            self.percepts.push(Percept::new(entity, attitude));
            (self.percepts.len() - 1, true)
        };
        (&mut self.percepts[index], added)
    }

    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = &mut Percept> {
        self.percepts.iter_mut()
    }

    pub(crate) fn retain(&mut self, keep: impl FnMut(&Percept) -> bool) {
        self.percepts.retain(keep);
    }
}

/// Something an agent perceives or remembers by one sense.
#[derive(Reflect, Debug, Clone, PartialEq)]
#[reflect(Debug, PartialEq)]
pub struct Percept {
    entity: Entity,
    attitude: Attitude,
    active: bool,
    trace: Option<Trace>,
}

impl Percept {
    fn new(entity: Entity, attitude: Attitude) -> Self {
        Self {
            entity,
            attitude,
            active: false,
            trace: None,
        }
    }

    /// What it is.
    #[must_use]
    pub fn entity(&self) -> Entity {
        self.entity
    }

    /// How the agent stands toward it.
    #[must_use]
    pub fn attitude(&self) -> Attitude {
        self.attitude
    }

    /// Whether the agent stands against it.
    #[must_use]
    pub fn is_hostile(&self) -> bool {
        self.attitude == Attitude::Hostile
    }

    /// Whether the agent stands with it.
    #[must_use]
    pub fn is_friendly(&self) -> bool {
        self.attitude == Attitude::Friendly
    }

    /// Whether it's sensed now, for a sustained sense: in view, for sight.
    /// Always `false` for a sense of moments.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// When and where it was sensed: for a sustained sense, as it became
    /// active, or as it stopped being; else, last.
    #[must_use]
    pub fn trace(&self) -> Option<Trace> {
        self.trace
    }

    pub(crate) fn set_attitude(&mut self, attitude: Attitude) -> bool {
        let changed = self.attitude != attitude;
        self.attitude = attitude;
        changed
    }

    /// Sensed at `trace`, a moment: kept if it's newer. Whether it changed.
    pub(crate) fn sense(&mut self, trace: Trace) -> bool {
        if self
            .trace
            .is_some_and(|known| known.at > trace.at || known == trace)
        {
            return false;
        }
        self.trace = Some(trace);
        true
    }

    /// Becomes active at `trace`, if it isn't: whether it changed.
    pub(crate) fn activate(&mut self, trace: Trace) -> bool {
        if self.active {
            return false;
        }
        self.active = true;
        self.trace = Some(trace);
        true
    }

    /// Stops being active at `trace`, if it is: whether it changed.
    pub(crate) fn deactivate(&mut self, trace: Trace) -> bool {
        if !self.active {
            return false;
        }
        self.active = false;
        self.trace = Some(trace);
        true
    }

    /// Lets it fade, if `memory` no longer holds it at `now`: whether it did.
    /// What's active never fades.
    pub(crate) fn fade(&mut self, memory: Duration, now: Duration) -> bool {
        if self.active || self.fades_at(memory).is_none_or(|at| now < at) {
            return false;
        }
        self.trace = None;
        true
    }

    /// When `memory` lets it fade, if it can.
    pub(crate) fn fades_at(&self, memory: Duration) -> Option<Duration> {
        if self.active {
            return None;
        }
        self.trace.map(|trace| trace.at.saturating_add(memory))
    }

    /// Whether anything still remembers it.
    pub(crate) fn remembered(&self) -> bool {
        self.active || self.trace.is_some()
    }
}

/// When and where something was sensed, by the clock perception runs on, and in
/// its [`Space`](crate::Space).
#[derive(Reflect, Debug, Clone, Copy, PartialEq)]
#[reflect(Debug, PartialEq)]
pub struct Trace {
    /// When.
    pub at: Duration,
    /// Where.
    pub position: Vec3,
}
