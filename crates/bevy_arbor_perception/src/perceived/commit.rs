use bevy::ecs::entity::EntityHashMap;
use bevy::prelude::*;

use super::{Attitude, Candidates, Lost, Noticed, Perceived, Team, Trace};
use crate::sense::{Sense, Sensors};
use crate::space::Space;

/// Each sensor of a sustained sense that looked this tick settles what it
/// senses now: what it wasn't blocked from.
pub(crate) fn settle<S: Sense>(mut sensors: Query<&mut Candidates<S>>) {
    if !S::SUSTAINED {
        return;
    }
    sensors.par_iter_mut().for_each(|mut candidates| {
        if !candidates.looked {
            return;
        }
        let candidates = candidates.as_mut();
        candidates.sensing.clear();
        for candidate in candidates.list.iter().filter(|c| !c.blocked()) {
            if !candidates.sensing.contains(&candidate.entity()) {
                candidates.sensing.push(candidate.entity());
            }
        }
    });
}

/// Writes what an agent and its sensors sensed by `S` to its `Perceived<S>`,
/// changing it only when something is noticed, lost, sensed anew, or its
/// attitude changes; and triggers [`Noticed`] and [`Lost`].
pub(crate) fn commit<S: Sense>(
    time: Res<Time>,
    space: Res<Space>,
    commands: ParallelCommands,
    teams: Query<&Team>,
    places: Query<&GlobalTransform>,
    readings: Query<&Candidates<S>>,
    mut agents: Query<(Entity, &mut Perceived<S>, Option<&Sensors>)>,
) {
    let (now, space) = (time.elapsed(), *space);
    agents
        .par_iter_mut()
        .for_each(|(agent, mut perceived, sensors)| {
            let sources: Vec<&Candidates<S>> = std::iter::once(agent)
                .chain(sensors.into_iter().flat_map(Sensors::iter))
                .filter_map(|sensor| readings.get(sensor).ok())
                .collect();
            // A sensor gone or detached: what it saw is looked at again.
            let looked = sources.iter().any(|source| source.looked) || perceived.recheck;
            if !looked && sources.iter().all(|source| source.list.is_empty()) {
                return;
            }
            let mine = teams.get(agent).ok().copied();
            let (mut changed, mut noticed, mut lost) = (false, Vec::new(), Vec::new());
            let percepts = perceived.bypass_change_detection();
            percepts.recheck = false;
            let memory = percepts.memory();
            let sensed = || {
                sources
                    .iter()
                    .flat_map(|source| source.list.iter())
                    .filter(|candidate| !candidate.blocked() && candidate.entity() != agent)
            };
            // One attitude for each, however many sensors sensed it: one a
            // system set beats the team's.
            let mut attitudes = EntityHashMap::<(Attitude, bool)>::default();
            for candidate in sensed() {
                let set = candidate.attitude();
                let slot = attitudes.entry(candidate.entity()).or_insert_with(|| {
                    let team = Team::attitude(mine, teams.get(candidate.entity()).ok().copied());
                    (set.unwrap_or(team), set.is_some())
                });
                if let Some(set) = set
                    && !slot.1
                {
                    *slot = (set, true);
                }
            }
            for candidate in sensed() {
                // Sensed so long ago it'd be forgotten at once, as an old
                // memory shared: not noticed at all.
                let stale = candidate
                    .at()
                    .is_some_and(|at| at.saturating_add(memory) <= now);
                if stale && percepts.get(candidate.entity()).is_none() {
                    continue;
                }
                let (attitude, _) = attitudes[&candidate.entity()];
                let trace = Trace {
                    at: candidate.at().unwrap_or(now),
                    position: candidate.position(),
                };
                let (percept, added) = percepts.entry(candidate.entity(), attitude);
                changed |= percept.set_attitude(attitude);
                let sensed = if S::SUSTAINED {
                    percept.activate(trace)
                } else {
                    percept.sense(trace)
                };
                changed |= added || sensed;
                // New to it, or active again.
                if added || (S::SUSTAINED && sensed) {
                    noticed.push(candidate.entity());
                }
            }
            // What none of its sensors senses any more is lost.
            if S::SUSTAINED && looked {
                for percept in percepts.iter_mut().filter(|percept| percept.is_active()) {
                    let still = sources
                        .iter()
                        .any(|source| source.sensing.contains(&percept.entity()));
                    if still {
                        continue;
                    }
                    // Where it was as it was lost; gone altogether, where it
                    // was sensed last.
                    let position = places.get(percept.entity()).map_or_else(
                        |_| percept.trace().map_or(Vec3::ZERO, |trace| trace.position),
                        |transform| space.flatten(transform.translation()),
                    );
                    if percept.deactivate(Trace { at: now, position }) {
                        lost.push(percept.entity());
                        changed = true;
                    }
                }
            }
            if changed {
                perceived.set_changed();
                // What it knows changed, so when it next forgets may have too.
                perceived.bypass_change_detection().next_fade = None;
            }
            if noticed.is_empty() && lost.is_empty() {
                return;
            }
            commands.command_scope(|mut commands| {
                for what in noticed {
                    commands.trigger(Noticed::<S>::new(agent, what));
                }
                for what in lost {
                    commands.trigger(Lost::<S>::new(agent, what));
                }
            });
        });
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::prelude::*;

    use crate::prelude::*;
    use crate::testing::*;

    #[test]
    fn perceived_changes_only_when_something_does() {
        for space in SPACES {
            let mut app = app(space);
            let agent = looker(&mut app, Sight::new(10.0));
            let target = thing(&mut app, ahead(space, 5.0));
            app.tick();
            assert!(sees(&app, agent, target));
            let before = app.world().resource::<Changes>().0;
            // It moves about in view, and is looked at every tick.
            for step in 1..=5 {
                let step = f32::from(u8::try_from(step).unwrap());
                app.entity_mut(target)
                    .insert(GlobalTransform::from_translation(ahead(
                        space,
                        5.0 + step / 2.0,
                    )));
                app.tick();
            }
            assert_eq!(
                app.world().resource::<Changes>().0,
                before,
                "{space:?}: still in view, nothing to change"
            );
            app.entity_mut(target)
                .insert(GlobalTransform::from_translation(ahead(space, 50.0)));
            app.tick();
            assert_eq!(
                app.world().resource::<Changes>().0,
                before + 1,
                "{space:?}: lost"
            );
        }
    }

    #[test]
    fn a_noise_leaves_what_is_seen_untouched() {
        for space in SPACES {
            let mut app = app(space);
            let agent = app.spawn((
                Sight::new(10.0).with_interval(Duration::ZERO),
                Hearing::new(10.0),
                GlobalTransform::IDENTITY,
            ));
            let target = thing(&mut app, ahead(space, 5.0));
            app.tick();
            let seen = app.world().resource::<Changes>().0;
            app.write(Noise::new(target, ahead(space, 5.0)));
            app.tick();
            assert!(
                perceived::<Hearing>(&app, agent).get(target).is_some(),
                "{space:?}"
            );
            assert_eq!(
                app.world().resource::<Changes>().0,
                seen,
                "{space:?}: a tree reading sight sleeps on"
            );
        }
    }

    /// What the agent's own looking finds, a system sets hostile; what its
    /// sensors find, it leaves to teams.
    fn hostile_to_the_agent_only(mut own: Query<&mut Candidates<Sight>, Without<SensorOf>>) {
        for mut candidates in &mut own {
            for candidate in candidates.iter_mut() {
                candidate.set_attitude(Attitude::Hostile);
            }
        }
    }

    #[test]
    fn sensors_disagreeing_settle_on_what_a_system_set_and_stay_settled() {
        for space in SPACES {
            let mut app = app(space);
            app.app_mut().add_systems(
                FixedUpdate,
                hostile_to_the_agent_only.in_set(PerceptionSystems::Classify),
            );
            let agent = looker(
                &mut app,
                Sight::new(10.0).with_half_angle(std::f32::consts::PI),
            );
            app.spawn((
                Sight::new(10.0)
                    .with_half_angle(std::f32::consts::PI)
                    .with_interval(Duration::ZERO),
                SensorOf(agent),
                GlobalTransform::IDENTITY,
            ));
            let target = thing(&mut app, ahead(space, 5.0));
            app.ticks(2);
            let attitude = perceived::<Sight>(&app, agent)
                .get(target)
                .unwrap()
                .attitude();
            assert_eq!(attitude, Attitude::Hostile, "{space:?}: the system's");
            let changes = app.world().resource::<Changes>().0;
            app.ticks(5);
            assert_eq!(
                app.world().resource::<Changes>().0,
                changes,
                "{space:?}: nothing changing, nothing woken"
            );
        }
    }
}
