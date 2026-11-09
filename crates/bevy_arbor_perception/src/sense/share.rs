use std::time::Duration;

use bevy::prelude::*;

use super::Sense;
use crate::perceived::{Attitude, Candidates, Perceived, Team, Trace, Traces};
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

/// What tellers pass on, by every sense, for [`tell`] to hand their
/// listeners: each with the order of the [`Share`] it's for.
#[derive(Resource, Default)]
pub(crate) struct Told(Vec<Passed>);

/// One thing passed on.
struct Passed {
    /// The order of the share it's for.
    share: usize,
    what: Entity,
    trace: Trace,
}

/// What each teller perceives by `S` is gathered to be passed on: what's
/// active for it, where it is now; else when and where it was last sensed.
/// One of these runs for each sense, before [`tell`].
pub(crate) fn told<S: Sense>(
    time: Res<Time>,
    space: Res<Space>,
    mut shares: MessageReader<Share>,
    tellers: Query<(&Perceived<S>, &Traces<S>)>,
    places: Query<&GlobalTransform>,
    mut told: ResMut<Told>,
) {
    let (now, space) = (time.elapsed(), *space);
    for (share, id) in shares.read_with_id() {
        let Ok((perceived, traces)) = tellers.get(share.from) else {
            continue;
        };
        let about = perceived
            .iter()
            .filter(|percept| share.about.is_none_or(|about| percept.entity() == about));
        for percept in about {
            let entity = percept.entity();
            let live = percept
                .is_active()
                .then(|| places.get(entity).ok())
                .flatten()
                .map(|transform| Trace {
                    at: now,
                    position: space.flatten(transform.translation()),
                });
            if let Some(trace) = live.or_else(|| traces.get(entity)) {
                told.0.push(Passed {
                    share: id.id,
                    what: entity,
                    trace,
                });
            }
        }
    }
}

/// Each [`Share`]'s listeners are found once, and handed what its teller
/// passes on by every sense, as candidates for [`Shared`].
pub(crate) fn tell(
    space: Res<Space>,
    mut shares: MessageReader<Share>,
    tellers: Query<(&GlobalTransform, Option<&Team>)>,
    mut listeners: Query<(
        Entity,
        &GlobalTransform,
        Option<&Team>,
        &mut Candidates<Shared>,
    )>,
    mut told: ResMut<Told>,
) {
    if told.0.is_empty() {
        shares.clear();
        return;
    }
    let space = *space;
    // Gathered sense by sense: each share's together, in the order written.
    told.0.sort_by_key(|passed| passed.share);
    for (share, id) in shares.read_with_id() {
        let from = told.0.partition_point(|passed| passed.share < id.id);
        let to = told.0.partition_point(|passed| passed.share <= id.id);
        let told = &told.0[from..to];
        if told.is_empty() {
            continue;
        }
        let tell = |listener: Entity, mut candidates: Mut<Candidates<Shared>>| {
            for passed in told.iter().filter(|passed| passed.what != listener) {
                candidates.sense_at(passed.what, passed.trace.position, passed.trace.at);
            }
        };
        match &share.to {
            Listeners::Allies { range } => {
                let Ok((teller, team)) = tellers.get(share.from) else {
                    continue;
                };
                let at = space.flatten(teller.translation());
                for (listener, transform, theirs, candidates) in &mut listeners {
                    let near = *range > 0.0
                        && space.flatten(transform.translation()).distance_squared(at)
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
    told.0.clear();
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
                traces::<Shared>(&app, ally).get(target).unwrap().position,
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
            let lost = traces::<Sight>(&app, teller).get(target).unwrap();
            app.ticks(5);
            app.write(Share::to(teller, [ally]));
            app.tick();
            let told = traces::<Shared>(&app, ally).get(target).unwrap();
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

    /// How often something was noticed by what was shared.
    #[derive(Resource, Default)]
    struct NoticedShared(u32);

    #[test]
    fn a_memory_older_than_the_listener_remembers_is_not_noticed() {
        for space in SPACES {
            let mut app = app(space);
            let teller = teller(&mut app);
            app.entity_mut(teller)
                .insert(Perceived::<Sight>::remembering(Duration::MAX));
            let target = enemy(&mut app, ahead(space, 5.0));
            let ally = listener(&mut app, 1, behind(space, 3.0));
            app.entity_mut(ally)
                .insert(Perceived::<Shared>::remembering(Duration::from_millis(1)));
            app.app_mut().init_resource::<NoticedShared>().add_observer(
                |_: On<crate::perceived::Noticed<Shared>>, mut n: ResMut<NoticedShared>| {
                    n.0 += 1;
                },
            );
            app.tick();
            app.entity_mut(target)
                .insert(GlobalTransform::from_translation(ahead(space, 50.0)));
            app.ticks(5);
            app.write(Share::to(teller, [ally]));
            app.tick();
            assert!(
                perceived::<Shared>(&app, ally).get(target).is_none(),
                "{space:?}: too old for it"
            );
            assert_eq!(
                app.world().resource::<NoticedShared>().0,
                0,
                "{space:?}: not noticed, then forgotten"
            );
        }
    }

    #[test]
    fn a_negative_range_tells_no_one() {
        for space in SPACES {
            let mut app = app(space);
            let teller = teller(&mut app);
            enemy(&mut app, ahead(space, 5.0));
            let ally = listener(&mut app, 1, behind(space, 3.0));
            app.tick();
            app.write(Share::allies(teller, -10.0));
            app.tick();
            assert!(perceived::<Shared>(&app, ally).is_empty(), "{space:?}");
        }
    }

    #[test]
    fn what_it_perceives_by_every_sense_is_told_once_to_each_listener() {
        for space in SPACES {
            let mut app = app(space);
            let teller = teller(&mut app);
            app.entity_mut(teller).insert(Hearing::new(10.0));
            let seen = enemy(&mut app, ahead(space, 5.0));
            let heard = app.spawn(GlobalTransform::IDENTITY);
            let ally = listener(&mut app, 1, behind(space, 3.0));
            app.write(Noise::new(heard, behind(space, 4.0)));
            app.tick();
            app.write(Share::allies(teller, 10.0));
            app.tick();
            let told = perceived::<Shared>(&app, ally);
            assert!(told.get(seen).is_some(), "{space:?}: what it sees");
            assert!(told.get(heard).is_some(), "{space:?}: what it heard");
            assert_eq!(told.iter().count(), 2, "{space:?}: once each");
        }
    }

    #[test]
    fn shares_in_one_tick_each_tell_their_own() {
        for space in SPACES {
            let mut app = app(space);
            let teller = teller(&mut app);
            let first = enemy(&mut app, ahead(space, 5.0));
            let second = enemy(&mut app, behind(space, 5.0));
            let one = listener(&mut app, 1, behind(space, 3.0));
            let other = listener(&mut app, 1, behind(space, 3.0));
            app.tick();
            app.write(Share::to(teller, [one]).about(first));
            app.write(Share::to(teller, [other]).about(second));
            app.tick();
            let (one, other) = (
                perceived::<Shared>(&app, one),
                perceived::<Shared>(&app, other),
            );
            assert!(
                one.get(first).is_some() && one.get(second).is_none(),
                "{space:?}"
            );
            assert!(
                other.get(second).is_some() && other.get(first).is_none(),
                "{space:?}"
            );
        }
    }
}
