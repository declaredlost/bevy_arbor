use std::fmt;
use std::marker::PhantomData;
use std::time::Duration;

use bevy::prelude::*;

use crate::sense::Sense;

/// On an agent, beside its [`Perceived<S>`](super::Perceived): when and where
/// it sensed each thing.
///
/// That's each thing it perceives or remembers by `S`.
///
/// Apart from `Perceived<S>` so that something sensed again, a footstep from
/// what the agent already heard, changes only this: a tree whose decorators
/// read `Perceived<S>` sleeps on, and one reading `Stimuli<S>` wakes. For a
/// sustained sense it changes only as `Perceived<S>` does, as something comes
/// into view or is lost.
///
/// Everything in `Perceived<S>` has a stimulus here. Kept by perception: added
/// and taken off with `Perceived<S>`.
#[derive(Component, Reflect)]
#[reflect(Component, Default)]
pub struct Stimuli<S: Sense> {
    stimuli: Vec<(Entity, Stimulus)>,
    #[reflect(ignore)]
    sense: PhantomData<fn() -> S>,
}

impl<S: Sense> Default for Stimuli<S> {
    fn default() -> Self {
        Self {
            stimuli: Vec::new(),
            sense: PhantomData,
        }
    }
}

impl<S: Sense> Clone for Stimuli<S> {
    fn clone(&self) -> Self {
        Self {
            stimuli: self.stimuli.clone(),
            sense: PhantomData,
        }
    }
}

impl<S: Sense> PartialEq for Stimuli<S> {
    fn eq(&self, other: &Self) -> bool {
        self.stimuli == other.stimuli
    }
}

impl<S: Sense> fmt::Debug for Stimuli<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(&format!("Stimuli<{}>", S::short_type_path()))
            .field("stimuli", &self.stimuli)
            .finish()
    }
}

impl<S: Sense> Stimuli<S> {
    /// When and where `entity` was sensed: last, unless `S` is a sustained
    /// sense.
    ///
    /// For a sustained sense, as it became active, or as it stopped being.
    #[must_use]
    pub fn get(&self, entity: Entity) -> Option<Stimulus> {
        self.stimuli
            .iter()
            .find(|(recorded, _)| *recorded == entity)
            .map(|(_, stimulus)| *stimulus)
    }

    /// Each thing perceived or remembered, and its stimulus.
    pub fn iter(&self) -> impl Iterator<Item = (Entity, Stimulus)> + '_ {
        self.stimuli.iter().copied()
    }

    /// Whether there's no stimulus of anything.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.stimuli.is_empty()
    }

    /// `entity` sensed at `stimulus`: whether that changed it.
    pub(crate) fn set(&mut self, entity: Entity, stimulus: Stimulus) -> bool {
        match self
            .stimuli
            .iter_mut()
            .find(|(recorded, _)| *recorded == entity)
        {
            Some((_, known)) if *known == stimulus => false,
            Some((_, known)) => {
                *known = stimulus;
                true
            }
            None => {
                self.stimuli.push((entity, stimulus));
                true
            }
        }
    }

    /// `entity` sensed at `stimulus`, a moment: kept if it's newer, or nothing
    /// was known.
    ///
    /// Returns whether that changed it.
    pub(crate) fn renew(&mut self, entity: Entity, stimulus: Stimulus) -> bool {
        if self.get(entity).is_some_and(|known| known.at > stimulus.at) {
            return false;
        }
        self.set(entity, stimulus)
    }

    pub(crate) fn retain(&mut self, mut keep: impl FnMut(Entity) -> bool) {
        self.stimuli.retain(|(entity, _)| keep(*entity));
    }
}

/// When and where something was sensed, by the clock perception runs on.
///
/// Where is in perception's [`Space`](crate::Space).
#[derive(Reflect, Debug, Clone, Copy, PartialEq)]
#[reflect(Debug, PartialEq)]
pub struct Stimulus {
    /// When.
    pub at: Duration,
    /// Where.
    pub position: Vec3,
}

impl Stimulus {
    /// When `memory` lets it fade.
    pub(crate) fn fades_at(self, memory: Duration) -> Duration {
        self.at.saturating_add(memory)
    }
}
