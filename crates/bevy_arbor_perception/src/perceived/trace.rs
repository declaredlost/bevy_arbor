use std::fmt;
use std::marker::PhantomData;
use std::time::Duration;

use bevy::prelude::*;

use crate::sense::Sense;

/// On an agent, beside its [`Perceived<S>`](super::Perceived): when and where
/// it sensed each thing it perceives or remembers by `S`.
///
/// Apart from `Perceived<S>` so that something sensed again, a footstep from
/// what the agent already heard, changes only this: a tree whose decorators
/// read `Perceived<S>` sleeps on, and one reading `Traces<S>` wakes. For a
/// sustained sense it changes only as `Perceived<S>` does, as something comes
/// into view or is lost.
///
/// Everything in `Perceived<S>` has a trace here. Kept by perception: added and
/// taken off with `Perceived<S>`.
#[derive(Component, Reflect)]
#[reflect(Component, Default)]
pub struct Traces<S: Sense> {
    traces: Vec<(Entity, Trace)>,
    #[reflect(ignore)]
    sense: PhantomData<fn() -> S>,
}

impl<S: Sense> Default for Traces<S> {
    fn default() -> Self {
        Self {
            traces: Vec::new(),
            sense: PhantomData,
        }
    }
}

impl<S: Sense> Clone for Traces<S> {
    fn clone(&self) -> Self {
        Self {
            traces: self.traces.clone(),
            sense: PhantomData,
        }
    }
}

impl<S: Sense> PartialEq for Traces<S> {
    fn eq(&self, other: &Self) -> bool {
        self.traces == other.traces
    }
}

impl<S: Sense> fmt::Debug for Traces<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(&format!("Traces<{}>", S::short_type_path()))
            .field("traces", &self.traces)
            .finish()
    }
}

impl<S: Sense> Traces<S> {
    /// When and where `entity` was sensed: for a sustained sense, as it became
    /// active, or as it stopped being; else, last.
    #[must_use]
    pub fn get(&self, entity: Entity) -> Option<Trace> {
        self.traces
            .iter()
            .find(|(traced, _)| *traced == entity)
            .map(|(_, trace)| *trace)
    }

    /// Each thing perceived or remembered, and its trace.
    pub fn iter(&self) -> impl Iterator<Item = (Entity, Trace)> + '_ {
        self.traces.iter().copied()
    }

    /// Whether there's no trace of anything.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.traces.is_empty()
    }

    /// `entity` sensed at `trace`: whether that changed it.
    pub(crate) fn set(&mut self, entity: Entity, trace: Trace) -> bool {
        match self.traces.iter_mut().find(|(traced, _)| *traced == entity) {
            Some((_, known)) if *known == trace => false,
            Some((_, known)) => {
                *known = trace;
                true
            }
            None => {
                self.traces.push((entity, trace));
                true
            }
        }
    }

    /// `entity` sensed at `trace`, a moment: kept if it's newer, or nothing
    /// was known. Whether that changed it.
    pub(crate) fn renew(&mut self, entity: Entity, trace: Trace) -> bool {
        if self.get(entity).is_some_and(|known| known.at > trace.at) {
            return false;
        }
        self.set(entity, trace)
    }

    pub(crate) fn retain(&mut self, mut keep: impl FnMut(Entity) -> bool) {
        self.traces.retain(|(entity, _)| keep(*entity));
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

impl Trace {
    /// When `memory` lets it fade.
    pub(crate) fn fades_at(self, memory: Duration) -> Duration {
        self.at.saturating_add(memory)
    }
}
