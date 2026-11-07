use std::time::Duration;

use bevy::prelude::*;

use super::{Candidates, Forgotten, Perceived};
use crate::sense::Sense;

/// Each agent whose memory of something by `S` is due to fade lets it, and
/// forgets what nothing remembers: triggering [`Forgotten`].
pub(crate) fn forget<S: Sense>(
    time: Res<Time>,
    commands: ParallelCommands,
    mut agents: Query<(Entity, &mut Perceived<S>)>,
) {
    let now = time.elapsed();
    agents.par_iter_mut().for_each(|(agent, mut perceived)| {
        if perceived.next_fade.is_some_and(|next| now < next) {
            return;
        }
        let percepts = perceived.bypass_change_detection();
        let memory = percepts.memory();
        let mut changed = false;
        for percept in percepts.iter_mut() {
            changed |= percept.fade(memory, now);
        }
        let mut forgotten = Vec::new();
        percepts.retain(|percept| {
            let kept = percept.remembered();
            if !kept {
                forgotten.push(percept.entity());
            }
            kept
        });
        percepts.next_fade = percepts
            .iter()
            .filter_map(|percept| percept.fades_at(memory))
            .min()
            // Nothing to fade: looked at again once something changes.
            .or(Some(Duration::MAX));
        if changed {
            perceived.set_changed();
        }
        if !forgotten.is_empty() {
            commands.command_scope(|mut commands| {
                for what in forgotten {
                    commands.trigger(Forgotten::<S>::new(agent, what));
                }
            });
        }
    });
}

/// Empties every sensor's candidates for `S`, ready for the next tick.
pub(crate) fn clear<S: Sense>(mut sensors: Query<&mut Candidates<S>>) {
    sensors.par_iter_mut().for_each(|mut candidates| {
        if candidates.looked || !candidates.list.is_empty() {
            let candidates = candidates.as_mut();
            candidates.list.clear();
            candidates.looked = false;
        }
    });
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use crate::prelude::*;
    use crate::testing::*;

    #[test]
    fn what_went_out_of_view_is_forgotten_once_its_memory_fades() {
        for space in SPACES {
            let mut app = logged(space);
            let agent = remembering(&mut app, 1.0, 1.0);
            let target = thing(&mut app, ahead(space, 5.0));
            app.tick();
            app.entity_mut(target)
                .insert(GlobalTransform::from_translation(ahead(space, 50.0)));
            app.tick();
            let changes = app.world().resource::<Changes>().0;
            app.secs(0.5);
            assert!(
                perceived::<Sight>(&app, agent).get(target).is_some(),
                "{space:?}: remembered"
            );
            assert_eq!(
                app.world().resource::<Changes>().0,
                changes,
                "{space:?}: untouched while it lasts"
            );
            app.secs(0.6);
            assert!(
                perceived::<Sight>(&app, agent).get(target).is_none(),
                "{space:?}: forgotten"
            );
            assert_eq!(
                app.world().resource::<Changes>().0,
                changes + 1,
                "{space:?}"
            );
            assert_eq!(
                log(&app).last(),
                Some(&("forgotten", "sight", target)),
                "{space:?}"
            );
        }
    }

    #[test]
    fn what_is_in_view_is_never_forgotten() {
        for space in SPACES {
            let mut app = logged(space);
            let agent = remembering(&mut app, 0.1, 0.1);
            let target = thing(&mut app, ahead(space, 5.0));
            // Something else going on, so its memory is gone through meanwhile.
            let noisy = app.spawn(GlobalTransform::IDENTITY);
            for _ in 0..64 {
                app.write(Noise::new(noisy, ahead(space, 2.0)));
                app.tick();
            }
            assert!(sees(&app, agent, target), "{space:?}");
            let percept = perceived::<Sight>(&app, agent).get(target).unwrap();
            assert!(
                percept.trace().is_some(),
                "{space:?}: when it came into view, kept"
            );
        }
    }

    #[test]
    fn one_sense_forgetting_leaves_what_the_others_perceive() {
        for space in SPACES {
            let mut app = logged(space);
            let agent = remembering(&mut app, 10.0, 0.5);
            let target = thing(&mut app, ahead(space, 5.0));
            app.write(Noise::new(target, ahead(space, 5.0)));
            app.tick();
            assert!(
                perceived::<Hearing>(&app, agent).get(target).is_some(),
                "{space:?}: heard"
            );
            app.secs(0.6);
            assert!(
                perceived::<Hearing>(&app, agent).get(target).is_none(),
                "{space:?}: hearing forgot it"
            );
            assert!(sees(&app, agent, target), "{space:?}: still seen");
            assert!(
                log(&app).contains(&("forgotten", "hearing", target)),
                "{space:?}"
            );
        }
    }

    #[test]
    fn remembering_forever_never_forgets() {
        for space in SPACES {
            let mut app = logged(space);
            let agent = remembering(&mut app, f32::INFINITY, f32::INFINITY);
            let target = thing(&mut app, ahead(space, 5.0));
            app.tick();
            app.entity_mut(target)
                .insert(GlobalTransform::from_translation(ahead(space, 50.0)));
            app.secs(30.0);
            assert!(
                perceived::<Sight>(&app, agent).get(target).is_some(),
                "{space:?}"
            );
        }
    }
}
