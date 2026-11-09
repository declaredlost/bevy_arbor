use std::time::Duration;

use bevy::prelude::*;

use super::{Sense, SensorOf, agent};
use crate::perceived::Candidates;
use crate::space::Space;

/// On an agent or a sensor: what it can hear.
///
/// It hears a [`Noise`] within `range` times the noise's loudness.
#[derive(Component, Reflect, Debug, Clone, Copy, PartialEq)]
#[reflect(Component, Debug, Default, PartialEq)]
pub struct Hearing {
    /// How far it hears a noise of loudness 1.
    pub range: f32,
}

impl Sense for Hearing {
    /// Five seconds after it was heard.
    const MEMORY: Duration = Duration::from_secs(5);
}

impl Default for Hearing {
    /// Ten units.
    fn default() -> Self {
        Self::new(10.0)
    }
}

impl Hearing {
    /// Hearing a noise of loudness 1 as far as `range`.
    #[must_use]
    pub fn new(range: f32) -> Self {
        Self { range }
    }
}

/// A sound `source` made at `at`, for what has [`Hearing`] to hear: write it
/// as a message.
///
/// What makes it is what's heard, so an explosion is its own entity.
/// Something making a noise every tick wakes trees every tick: make noises at
/// the rate they matter, a footstep, not a frame.
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct Noise {
    /// What made it.
    pub source: Entity,
    /// Where.
    pub at: Vec3,
    /// How far it carries: 1 is as far as [`Hearing::range`], 2 twice that.
    pub loudness: f32,
}

impl Noise {
    /// A noise of loudness 1, made by `source` at `at`.
    #[must_use]
    pub fn new(source: Entity, at: Vec3) -> Self {
        Self {
            source,
            at,
            loudness: 1.0,
        }
    }

    /// Carrying `loudness` times as far.
    #[must_use]
    pub fn with_loudness(mut self, loudness: f32) -> Self {
        self.loudness = loudness;
        self
    }
}

/// Each sensor hears the noises within its range of them, but not its own
/// agent's.
pub(crate) fn hear(
    space: Res<Space>,
    mut noises: MessageReader<Noise>,
    mut sensors: Query<(
        Entity,
        &Hearing,
        &GlobalTransform,
        Option<&SensorOf>,
        &mut Candidates<Hearing>,
    )>,
) {
    let space = *space;
    let noises: Vec<Noise> = noises.read().copied().collect();
    if noises.is_empty() {
        return;
    }
    sensors
        .par_iter_mut()
        .for_each(|(sensor, hearing, transform, of, mut candidates)| {
            let ear = space.flatten(transform.translation());
            let agent = agent(sensor, of);
            for noise in &noises {
                let at = space.flatten(noise.at);
                let reach = hearing.range * noise.loudness;
                let own = noise.source == agent || noise.source == sensor;
                // A negative reach is none, not one squared positive.
                if !own && reach > 0.0 && ear.distance_squared(at) <= reach * reach {
                    candidates.sense(noise.source, at);
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use bevy_harness::TestApp;

    use crate::prelude::*;
    use crate::testing::*;

    /// Something that can make a noise: an entity, nothing more.
    fn source(app: &mut TestApp) -> Entity {
        app.spawn(GlobalTransform::IDENTITY)
    }

    #[test]
    fn it_hears_within_its_range_times_the_loudness() {
        for space in SPACES {
            let mut app = app(space);
            let agent = app.spawn((Hearing::new(10.0), GlobalTransform::IDENTITY));
            let (near, far, loud) = (source(&mut app), source(&mut app), source(&mut app));
            app.write(Noise::new(near, ahead(space, 8.0)));
            app.write(Noise::new(far, behind(space, 15.0)));
            app.write(Noise::new(loud, behind(space, 15.0)).with_loudness(2.0));
            app.tick();
            let perceived = perceived::<Hearing>(&app, agent);
            let heard = traces::<Hearing>(&app, agent).get(near).unwrap();
            assert_eq!(
                heard.position,
                space.flatten(ahead(space, 8.0)),
                "{space:?}"
            );
            assert!(
                !perceived.get(near).unwrap().is_active(),
                "{space:?}: heard, not seen"
            );
            assert!(perceived.get(far).is_none(), "{space:?}: too far");
            assert!(perceived.get(loud).is_some(), "{space:?}: loud enough");
        }
    }

    #[test]
    fn an_agent_without_hearing_or_making_the_noise_hears_nothing() {
        for space in SPACES {
            let mut app = app(space);
            let deaf = looker(&mut app, Sight::new(1.0));
            let noisy = app.spawn((Hearing::new(10.0), GlobalTransform::IDENTITY));
            app.write(Noise::new(noisy, ahead(space, 1.0)));
            app.tick();
            assert!(
                app.world().get::<Perceived<Hearing>>(deaf).is_none(),
                "{space:?}: no hearing"
            );
            assert!(
                perceived::<Hearing>(&app, noisy).is_empty(),
                "{space:?}: its own noise"
            );
        }
    }

    #[test]
    fn a_later_noise_moves_what_was_heard() {
        for space in SPACES {
            let mut app = app(space);
            let agent = app.spawn((Hearing::new(10.0), GlobalTransform::IDENTITY));
            let steps = source(&mut app);
            app.write(Noise::new(steps, ahead(space, 8.0)));
            app.tick();
            let first = traces::<Hearing>(&app, agent).get(steps).unwrap();
            app.tick();
            app.write(Noise::new(steps, ahead(space, 4.0)));
            app.tick();
            let second = traces::<Hearing>(&app, agent).get(steps).unwrap();
            assert!(second.at > first.at, "{space:?}");
            assert_eq!(
                second.position,
                space.flatten(ahead(space, 4.0)),
                "{space:?}"
            );
        }
    }

    #[test]
    fn a_negative_loudness_is_not_heard() {
        for space in SPACES {
            let mut app = app(space);
            let agent = app.spawn((Hearing::new(10.0), GlobalTransform::IDENTITY));
            let source = app.spawn(GlobalTransform::IDENTITY);
            app.write(Noise::new(source, ahead(space, 5.0)).with_loudness(-1.0));
            app.tick();
            assert!(perceived::<Hearing>(&app, agent).is_empty(), "{space:?}");
        }
    }

    /// How often the agents' `Perceived<Hearing>` and `Traces<Hearing>`
    /// changed.
    #[derive(Resource, Default)]
    struct HearingChanges {
        perceived: usize,
        traces: usize,
    }

    fn count(
        perceived: Query<(), Changed<Perceived<Hearing>>>,
        traces: Query<(), Changed<Traces<Hearing>>>,
        mut changes: ResMut<HearingChanges>,
    ) {
        changes.perceived += perceived.iter().count();
        changes.traces += traces.iter().count();
    }

    #[test]
    fn hearing_again_what_it_remembers_changes_only_its_traces() {
        for space in SPACES {
            let mut app = app(space);
            app.app_mut()
                .init_resource::<HearingChanges>()
                .add_systems(FixedUpdate, count.after(PerceptionSystems::Commit));
            let agent = app.spawn((Hearing::new(10.0), GlobalTransform::IDENTITY));
            let steps = source(&mut app);
            app.write(Noise::new(steps, ahead(space, 8.0)));
            app.tick();
            let heard = app.world().resource::<HearingChanges>();
            let (changed, traced) = (heard.perceived, heard.traces);
            for step in 1..=3 {
                let step = f32::from(u8::try_from(step).unwrap());
                app.write(Noise::new(steps, ahead(space, 8.0 - step)));
                app.tick();
            }
            let heard = app.world().resource::<HearingChanges>();
            assert_eq!(heard.perceived, changed, "{space:?}: a tree sleeps on");
            assert_eq!(heard.traces, traced + 3, "{space:?}: each step a trace");
            assert_eq!(
                traces::<Hearing>(&app, agent).get(steps).unwrap().position,
                space.flatten(ahead(space, 5.0)),
                "{space:?}"
            );
        }
    }
}
