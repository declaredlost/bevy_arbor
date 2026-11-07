use std::marker::PhantomData;
use std::time::Duration;

use bevy::prelude::*;

use super::Attitude;
use crate::sense::Sense;

/// On a sensor: what it might be sensing by `S` this tick, on its way to its
/// agent's [`Perceived<S>`](super::Perceived).
///
/// Filled in [`Gather`](crate::PerceptionSystems::Gather) (by a system of your
/// own, for a sense of yours), then a system can block candidates in
/// [`Occlude`](crate::PerceptionSystems::Occlude) (a raycast, say) or give
/// their attitude in [`Classify`](crate::PerceptionSystems::Classify), and
/// [`Commit`](crate::PerceptionSystems::Commit) writes what's left. Emptied
/// each tick.
#[derive(Component)]
pub struct Candidates<S: Sense> {
    pub(crate) eye: Vec3,
    pub(crate) looked: bool,
    pub(crate) next_look: Option<Duration>,
    /// What it sensed at its last look, for a sustained sense.
    pub(crate) sensing: Vec<Entity>,
    pub(crate) list: Vec<Candidate>,
    sense: PhantomData<fn() -> S>,
}

impl<S: Sense> Default for Candidates<S> {
    fn default() -> Self {
        Self {
            eye: Vec3::ZERO,
            looked: false,
            next_look: None,
            sensing: Vec::new(),
            list: Vec::new(),
            sense: PhantomData,
        }
    }
}

impl<S: Sense> Candidates<S> {
    /// Where it looked from this tick, if it looked.
    #[must_use]
    pub fn eye(&self) -> Option<Vec3> {
        self.looked.then_some(self.eye)
    }

    /// The candidates.
    pub fn iter(&self) -> impl Iterator<Item = &Candidate> {
        self.list.iter()
    }

    /// The candidates, to block or classify.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Candidate> {
        self.list.iter_mut()
    }

    /// Looks this tick, from `eye`: for a sustained sense, what it doesn't
    /// sense this look it no longer senses.
    pub fn look_from(&mut self, eye: Vec3) {
        self.looked = true;
        self.eye = eye;
    }

    /// Senses `entity` at `position` now, in the plugin's
    /// [`Space`](crate::space::Space).
    pub fn sense(&mut self, entity: Entity, position: Vec3) {
        self.list.push(Candidate::new(entity, position));
    }

    /// Senses `entity` as it was at `at`, at `position`: what's passed on.
    pub fn sense_at(&mut self, entity: Entity, position: Vec3, at: Duration) {
        let mut candidate = Candidate::new(entity, position);
        candidate.at = Some(at);
        self.list.push(candidate);
    }
}

/// Something a sensor might be sensing.
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    entity: Entity,
    position: Vec3,
    at: Option<Duration>,
    blocked: bool,
    attitude: Option<Attitude>,
}

impl Candidate {
    fn new(entity: Entity, position: Vec3) -> Self {
        Self {
            entity,
            position,
            at: None,
            blocked: false,
            attitude: None,
        }
    }

    /// What it is.
    #[must_use]
    pub fn entity(&self) -> Entity {
        self.entity
    }

    /// Where it is, in the plugin's [`Space`](crate::space::Space).
    #[must_use]
    pub fn position(&self) -> Vec3 {
        self.position
    }

    /// When it was sensed, if not now: what an ally passed on.
    #[must_use]
    pub fn at(&self) -> Option<Duration> {
        self.at
    }

    /// Stops it being sensed this tick: something's in the way.
    pub fn block(&mut self) {
        self.blocked = true;
    }

    /// Whether something blocked it.
    #[must_use]
    pub fn blocked(&self) -> bool {
        self.blocked
    }

    /// How the agent stands towards it, if a system has said.
    #[must_use]
    pub fn attitude(&self) -> Option<Attitude> {
        self.attitude
    }

    /// Says how the agent stands towards it, over the
    /// [`Team`](super::Team) fallback.
    pub fn set_attitude(&mut self, attitude: Attitude) {
        self.attitude = Some(attitude);
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::prelude::*;

    use crate::PerceptionSystems;
    use crate::perceived::Candidates;
    use crate::prelude::*;
    use crate::testing::*;

    /// Something of the game's own: what's behind a wall can't be seen.
    #[derive(Component)]
    struct BehindWall;

    fn walls(mut agents: Query<&mut Candidates<Sight>>, hidden: Query<(), With<BehindWall>>) {
        for mut candidates in &mut agents {
            for candidate in candidates.iter_mut() {
                if hidden.contains(candidate.entity()) {
                    candidate.block();
                }
            }
        }
    }

    /// Something of the game's own: anything marked so is hostile, whatever its
    /// team.
    #[derive(Component)]
    struct Cursed;

    fn curses(mut agents: Query<&mut Candidates<Sight>>, cursed: Query<(), With<Cursed>>) {
        for mut candidates in &mut agents {
            for candidate in candidates.iter_mut() {
                if cursed.contains(candidate.entity()) {
                    candidate.set_attitude(Attitude::Hostile);
                }
            }
        }
    }

    #[test]
    fn systems_of_your_own_block_and_classify_in_their_stages() {
        for space in SPACES {
            let mut app = app(space);
            app.app_mut().add_systems(
                FixedUpdate,
                (
                    walls.in_set(PerceptionSystems::Occlude),
                    curses.in_set(PerceptionSystems::Classify),
                ),
            );
            let agent = app.spawn((
                Sight::new(10.0).with_interval(Duration::ZERO),
                Team(1),
                GlobalTransform::IDENTITY,
            ));
            let hidden = app.spawn((
                Perceivable,
                BehindWall,
                GlobalTransform::from_translation(ahead(space, 3.0)),
            ));
            let cursed_ally = app.spawn((
                Perceivable,
                Cursed,
                Team(1),
                GlobalTransform::from_translation(ahead(space, 4.0)),
            ));
            app.tick();
            assert!(!sees(&app, agent, hidden), "{space:?}: blocked");
            assert!(
                perceived::<Sight>(&app, agent)
                    .get(cursed_ally)
                    .unwrap()
                    .is_hostile(),
                "{space:?}: over the team fallback"
            );
        }
    }
}
