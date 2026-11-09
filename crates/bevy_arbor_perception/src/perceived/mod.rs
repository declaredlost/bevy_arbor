//! What an agent perceives and remembers by each sense, how it stands toward
//! it, and what's triggered as that changes.

mod attitude;
mod candidate;
mod commit;
mod events;
mod memory;
mod trace;

use std::fmt;
use std::marker::PhantomData;
use std::time::Duration;

use bevy::prelude::*;

pub use self::attitude::{Attitude, Team};
pub use self::candidate::{Candidate, Candidates};
pub(crate) use self::commit::{commit, settle};
pub use self::events::{Forgotten, Lost, Noticed};
pub(crate) use self::memory::{clear, forget};
pub use self::trace::{Trace, Traces};
use crate::sense::Sense;

/// On an agent: what it perceives and remembers by the sense `S`, from
/// itself and its sensors.
///
/// Changes only when something is noticed, lost or forgotten, or its attitude
/// changes: never as things move, or as what's remembered is sensed again. So
/// a tree whose decorators read it sleeps in between, and one reading
/// `Perceived<Sight>` isn't woken by a noise. When and where each thing was
/// sensed is in its [`Traces<S>`](Traces), which comes with it. While
/// something is active (in view, for sight), read its `Transform` for where
/// it is now.
///
/// It remembers for [`Sense::MEMORY`], unless made with
/// [`remembering`](Self::remembering): what's no longer active fades that
/// long after it was last sensed, and is then forgotten.
#[derive(Component, Reflect)]
#[reflect(Component, Default)]
#[require(Traces<S>)]
pub struct Perceived<S: Sense> {
    percepts: Vec<Percept>,
    memory: Duration,
    /// When something next fades: `None` to work it out again.
    #[reflect(ignore)]
    pub(crate) next_fade: Option<Duration>,
    /// A sensor stopped sensing for it: what's active is looked at again,
    /// even with no sensor looking.
    #[reflect(ignore)]
    pub(crate) recheck: bool,
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
            recheck: self.recheck,
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
            recheck: false,
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

/// Something an agent perceives or remembers by one sense. When and where it
/// was sensed is in the agent's [`Traces<S>`](Traces).
#[derive(Reflect, Debug, Clone, PartialEq)]
#[reflect(Debug, PartialEq)]
pub struct Percept {
    entity: Entity,
    attitude: Attitude,
    active: bool,
}

impl Percept {
    fn new(entity: Entity, attitude: Attitude) -> Self {
        Self {
            entity,
            attitude,
            active: false,
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

    pub(crate) fn set_attitude(&mut self, attitude: Attitude) -> bool {
        let changed = self.attitude != attitude;
        self.attitude = attitude;
        changed
    }

    /// Becomes active, or stops being: whether it changed.
    pub(crate) fn set_active(&mut self, active: bool) -> bool {
        let changed = self.active != active;
        self.active = active;
        changed
    }
}
