use std::time::Duration;

use bevy::prelude::*;

use super::Sense;
use crate::perceived::{Attitude, Candidates, Perceived, Team};
use crate::space::Space;

/// On an agent: it takes what others [`Share`] with it, as
/// [`Perceived<Shared>`](Perceived).
#[derive(Component, Reflect, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[reflect(Component, Debug, Default, PartialEq)]
pub struct Shared;

impl Sense for Shared {
    /// Ten seconds after the teller sensed it.
    const MEMORY: Duration = Duration::from_secs(10);
}

/// `from` tells others what it perceives and remembers, by every sense: a
/// whole squad reacts when one spots you. Write it as a message.
///
/// What's active for it now (in its view) is passed on where it is now; what
/// it remembers, from when and where it last sensed it. Each listener, an
/// agent with [`Shared`], takes it as [`Perceived<Shared>`](Perceived), gives
/// it its own attitude, and keeps what it already knew if that's newer.
#[derive(Message, Debug, Clone, PartialEq)]
pub struct Share {
    /// Who tells.
    pub from: Entity,
    /// Who's told.
    pub to: Listeners,
    /// What about: everything it perceives, if `None`.
    pub about: Option<Entity>,
}

/// Who's told what's shared.
#[derive(Debug, Clone, PartialEq)]
pub enum Listeners {
    /// Agents on the teller's [`Team`] within `range` of it.
    Allies {
        /// How far it's heard.
        range: f32,
    },
    /// These agents, wherever they are.
    Only(Vec<Entity>),
}

impl Share {
    /// `from` tells the allies within `range` everything it perceives.
    #[must_use]
    pub fn allies(from: Entity, range: f32) -> Self {
        Self {
            from,
            to: Listeners::Allies { range },
            about: None,
        }
    }

    /// `from` tells `to` everything it perceives.
    #[must_use]
    pub fn to(from: Entity, to: impl IntoIterator<Item = Entity>) -> Self {
        Self {
            from,
            to: Listeners::Only(to.into_iter().collect()),
            about: None,
        }
    }

    /// Only about `about`.
    #[must_use]
    pub fn about(mut self, about: Entity) -> Self {
        self.about = Some(about);
        self
    }
}

/// What a teller perceived by `S` is passed on to its listeners, as
/// candidates for [`Shared`]: one of these runs for each sense.
pub(crate) fn told<S: Sense>(
    time: Res<Time>,
    space: Res<Space>,
    mut shares: MessageReader<Share>,
    tellers: Query<(&Perceived<S>, &GlobalTransform, Option<&Team>)>,
    places: Query<&GlobalTransform>,
    mut listeners: Query<(
        Entity,
        &GlobalTransform,
        Option<&Team>,
        &mut Candidates<Shared>,
    )>,
) {
    let (now, space) = (time.elapsed(), *space);
    for share in shares.read() {
        let Ok((perceived, teller, team)) = tellers.get(share.from) else {
            continue;
        };
        // In view, where it is now; else when and where it was last sensed.
        let told: Vec<(Entity, Vec3, Duration)> = perceived
            .iter()
            .filter(|percept| share.about.is_none_or(|about| percept.entity() == about))
            .filter_map(|percept| {
                let live = percept
                    .is_active()
                    .then(|| places.get(percept.entity()).ok())
                    .flatten()
                    .map(|transform| (space.flatten(transform.translation()), now));
                let (position, at) =
                    live.or_else(|| percept.trace().map(|trace| (trace.position, trace.at)))?;
                Some((percept.entity(), position, at))
            })
            .collect();
        if told.is_empty() {
            continue;
        }
        let tell = |listener: Entity, mut candidates: Mut<Candidates<Shared>>| {
            for &(entity, position, at) in told.iter().filter(|(e, ..)| *e != listener) {
                candidates.sense_at(entity, position, at);
            }
        };
        match &share.to {
            Listeners::Allies { range } => {
                let at = space.flatten(teller.translation());
                for (listener, transform, theirs, candidates) in &mut listeners {
                    let near = space.flatten(transform.translation()).distance_squared(at)
                        <= range * range;
                    let allied =
                        Team::attitude(team.copied(), theirs.copied()) == Attitude::Friendly;
                    if listener != share.from && near && allied {
                        tell(listener, candidates);
                    }
                }
            }
            Listeners::Only(to) => {
                for &listener in to {
                    if listener == share.from {
                        continue;
                    }
                    if let Ok((_, _, _, candidates)) = listeners.get_mut(listener) {
                        tell(listener, candidates);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::PI;
    use std::time::Duration;

    use bevy::prelude::*;
    use bevy_harness::TestApp;

    use crate::prelude::*;
    use crate::testing::*;

    /// An agent on `team` at `at` that perceives only what it's told.
    fn listener(app: &mut TestApp, team: u8, at: Vec3) -> Entity {
        app.spawn((Shared, Team(team), GlobalTransform::from_translation(at)))
    }

    /// An agent on team 1 at the origin that sees all round, every tick.
    fn teller(app: &mut TestApp) -> Entity {
        app.spawn((
            Sight::new(10.0)
                .with_half_angle(PI)
                .with_interval(Duration::ZERO),
            Team(1),
            GlobalTransform::IDENTITY,
        ))
    }

    fn enemy(app: &mut TestApp, at: Vec3) -> Entity {
        app.spawn((Perceivable, Team(2), GlobalTransform::from_translation(at)))
    }

    #[test]
    fn allies_in_range_are_told_where_what_it_sees_is_now() {
        for space in SPACES {
            let mut app = app(space);
            let teller = teller(&mut app);
            let target = enemy(&mut app, ahead(space, 5.0));
            let ally = listener(&mut app, 1, behind(space, 3.0));
            let far = listener(&mut app, 1, behind(space, 50.0));
            let other = listener(&mut app, 2, behind(space, 3.0));
            app.tick();
            app.entity_mut(target)
                .insert(GlobalTransform::from_translation(ahead(space, 6.0)));
            app.write(Share::allies(teller, 10.0));
            app.tick();
            let told = perceived::<Shared>(&app, ally).get(target).unwrap();
            assert_eq!(
                told.trace().unwrap().position,
                space.flatten(ahead(space, 6.0)),
                "{space:?}: where it is now"
            );
            assert!(told.is_hostile(), "{space:?}: its own attitude");
            assert!(
                perceived::<Shared>(&app, far).is_empty(),
                "{space:?}: out of range"
            );
            assert!(
                perceived::<Shared>(&app, other).is_empty(),
                "{space:?}: not an ally"
            );
        }
    }

    #[test]
    fn what_it_remembers_is_told_from_when_and_where_it_was_sensed() {
        for space in SPACES {
            let mut app = app(space);
            let teller = teller(&mut app);
            let target = enemy(&mut app, ahead(space, 5.0));
            let ally = listener(&mut app, 1, behind(space, 3.0));
            app.tick();
            app.entity_mut(target)
                .insert(GlobalTransform::from_translation(ahead(space, 50.0)));
            app.tick();
            let lost = perceived::<Sight>(&app, teller)
                .get(target)
                .unwrap()
                .trace()
                .unwrap();
            app.ticks(5);
            app.write(Share::to(teller, [ally]));
            app.tick();
            let told = perceived::<Shared>(&app, ally)
                .get(target)
                .and_then(Percept::trace)
                .unwrap();
            assert_eq!(told, lost, "{space:?}: when and where it was lost");
        }
    }

    #[test]
    fn it_can_tell_about_one_thing_and_never_tells_a_listener_about_itself() {
        for space in SPACES {
            let mut app = app(space);
            let teller = teller(&mut app);
            let first = enemy(&mut app, ahead(space, 5.0));
            let second = enemy(&mut app, behind(space, 5.0));
            let ally = app.spawn((
                Shared,
                Perceivable,
                Team(1),
                GlobalTransform::from_translation(ahead(space, 2.0)),
            ));
            app.tick();
            assert!(sees(&app, teller, ally), "{space:?}: it sees the ally too");
            app.write(Share::to(teller, [ally]).about(first));
            app.tick();
            let known = perceived::<Shared>(&app, ally);
            assert!(known.get(first).is_some(), "{space:?}");
            assert!(known.get(second).is_none(), "{space:?}: not asked about");
            app.write(Share::to(teller, [ally]));
            app.tick();
            assert!(
                perceived::<Shared>(&app, ally).get(ally).is_none(),
                "{space:?}: not about itself"
            );
        }
    }
}
