//! The senses: what agents sense with, what they sense from, and how a sense
//! of your own joins them.
//!
//! A sense is a component, its own configuration: [`Sight`], [`Hearing`],
//! [`Touch`], [`Shared`], or yours. On an agent, the agent senses; on a sensor
//! entity, [`SensorOf`] an agent, the sensor senses for it, from its own
//! transform. What's sensed goes to the agent's
//! [`Perceived<S>`](crate::perceived::Perceived), one per sense.

mod hearing;
mod sensor;
mod share;
mod sight;
mod touch;

use std::marker::PhantomData;
use std::time::Duration;

use bevy::ecs::schedule::InternedScheduleLabel;
use bevy::prelude::*;

pub(crate) use self::hearing::hear;
pub use self::hearing::{Hearing, Noise};
pub(crate) use self::sensor::agent;
pub use self::sensor::{SensorOf, Sensors};
pub(crate) use self::share::told;
pub use self::share::{Listeners, Share, Shared};
pub use self::sight::{Facing, Perceivable, Sight};
pub(crate) use self::sight::{build_grid, look};
pub(crate) use self::touch::feel;
pub use self::touch::{Touch, Touched};
use crate::PerceptionSystems;
use crate::perceived::{self, Candidates, Perceived};

/// A way of sensing: a component, and its own configuration.
///
/// Register one with [`SensePlugin`], and fill its sensors'
/// [`Candidates<S>`](Candidates) from a system of your own in
/// [`PerceptionSystems::Gather`]: what's left is written to
/// [`Perceived<S>`](Perceived) on the agent.
///
/// ```
/// # use std::time::Duration;
/// # use bevy::prelude::*;
/// # use bevy_arbor_perception::sense::Sense;
/// /// On an agent: it smells what's near.
/// #[derive(Component, Reflect)]
/// struct Smell {
///     range: f32,
/// }
///
/// impl Sense for Smell {
///     const MEMORY: Duration = Duration::from_secs(20);
/// }
/// ```
pub trait Sense: Component + Reflect + TypePath {
    /// Whether it holds while it lasts, as sight does (in view, then lost),
    /// rather than being a moment, as a noise is.
    ///
    /// A sustained sense's sensors say when they've looked
    /// ([`Candidates::look_from`]): what an agent's sensors sensed at their
    /// last look is active, and what none of them did any more is lost.
    const SUSTAINED: bool = false;

    /// How long an agent remembers what it sensed this way, unless its
    /// [`Perceived<S>`](Perceived) says otherwise.
    const MEMORY: Duration;
}

/// Registers the sense `S`: its [`Perceived<S>`](Perceived), the systems that
/// write and forget it, sharing it, and its events. [`PerceptionPlugin`]
/// registers the built-in senses; add one for a sense of your own, after it.
///
/// [`PerceptionPlugin`]: crate::PerceptionPlugin
pub struct SensePlugin<S>(PhantomData<fn() -> S>);

impl<S> Default for SensePlugin<S> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

/// The schedule perception runs in, for each sense's systems to join.
#[derive(Resource, Clone, Copy)]
pub(crate) struct PerceptionSchedule(pub(crate) InternedScheduleLabel);

impl<S: Sense> Plugin for SensePlugin<S> {
    fn build(&self, app: &mut App) {
        app.register_type::<Perceived<S>>()
            .add_observer(sense_added::<S>)
            .add_observer(sensor_attached::<S>);
    }

    fn finish(&self, app: &mut App) {
        use PerceptionSystems::{Commit, Gather};

        let Some(PerceptionSchedule(schedule)) =
            app.world().get_resource::<PerceptionSchedule>().copied()
        else {
            warn!(
                "`SensePlugin<{}>` needs `PerceptionPlugin`",
                S::short_type_path()
            );
            return;
        };
        app.add_systems(
            schedule,
            (
                told::<S>.in_set(Gather),
                (
                    perceived::settle::<S>,
                    perceived::commit::<S>,
                    perceived::forget::<S>,
                    perceived::clear::<S>,
                )
                    .chain()
                    .in_set(Commit),
            ),
        );
    }
}

/// A sense on an entity: it gathers into `Candidates<S>`, and its agent keeps
/// a `Perceived<S>`, unless they're there already.
fn sense_added<S: Sense>(added: On<Add<S>>, sensors: Query<&SensorOf>, mut commands: Commands) {
    let sensor = added.entity;
    commands
        .entity(sensor)
        .insert_if_new(Candidates::<S>::default());
    let agent = agent(sensor, sensors.get(sensor).ok());
    commands
        .entity(agent)
        .insert_if_new(Perceived::<S>::default());
}

/// A sensor with `S` attached to an agent: the agent keeps a `Perceived<S>`.
fn sensor_attached<S: Sense>(
    attached: On<Insert<SensorOf>>,
    sensors: Query<&SensorOf, With<S>>,
    mut commands: Commands,
) {
    if let Ok(of) = sensors.get(attached.entity) {
        commands
            .entity(of.agent())
            .insert_if_new(Perceived::<S>::default());
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::prelude::*;

    use super::{Sense, SensePlugin};
    use crate::PerceptionSystems;
    use crate::perceived::Candidates;
    use crate::prelude::*;
    use crate::testing::*;

    /// A sense of moments of the game's own: it smells what's `Scented`
    /// within range.
    #[derive(Component, Reflect)]
    struct Smell {
        range: f32,
    }

    impl Sense for Smell {
        const MEMORY: Duration = Duration::from_millis(500);
    }

    #[derive(Component)]
    struct Scented;

    fn sniff(
        space: Res<Space>,
        scents: Query<(Entity, &GlobalTransform), With<Scented>>,
        mut noses: Query<(&Smell, &GlobalTransform, &mut Candidates<Smell>)>,
    ) {
        for (smell, nose, mut candidates) in &mut noses {
            let at = space.flatten(nose.translation());
            for (entity, scent) in &scents {
                let there = space.flatten(scent.translation());
                if at.distance(there) <= smell.range {
                    candidates.sense(entity, there);
                }
            }
        }
    }

    /// A sustained sense of the game's own: it senses what's `Scented` while
    /// it's within range, and loses it once it isn't.
    #[derive(Component, Reflect)]
    struct Radar {
        range: f32,
    }

    impl Sense for Radar {
        const MEMORY: Duration = Duration::from_secs(5);
        const SUSTAINED: bool = true;
    }

    fn sweep(
        space: Res<Space>,
        blips: Query<(Entity, &GlobalTransform), With<Scented>>,
        mut radars: Query<(&Radar, &GlobalTransform, &mut Candidates<Radar>)>,
    ) {
        for (radar, dish, mut candidates) in &mut radars {
            let at = space.flatten(dish.translation());
            candidates.look_from(at);
            for (entity, blip) in &blips {
                let there = space.flatten(blip.translation());
                if at.distance(there) <= radar.range {
                    candidates.sense(entity, there);
                }
            }
        }
    }

    fn with_senses(space: Space) -> TestApp {
        TestApp::builder()
            .plugins((
                PerceptionPlugin::default().with_space(space),
                SensePlugin::<Smell>::default(),
                SensePlugin::<Radar>::default(),
            ))
            .systems(
                FixedUpdate,
                (sniff, sweep).in_set(PerceptionSystems::Gather),
            )
            .build()
    }

    use bevy_harness::TestApp;

    #[test]
    fn a_sense_of_your_own_is_perceived_classified_and_forgotten() {
        for space in SPACES {
            let mut app = with_senses(space);
            let agent = app.spawn((Smell { range: 5.0 }, Team(1), GlobalTransform::IDENTITY));
            let prey = app.spawn((
                Scented,
                Team(2),
                GlobalTransform::from_translation(ahead(space, 3.0)),
            ));
            app.tick();
            let smelt = perceived::<Smell>(&app, agent).get(prey).unwrap();
            assert!(smelt.is_hostile(), "{space:?}: by the team fallback");
            assert!(!smelt.is_active(), "{space:?}: a moment");
            app.entity_mut(prey)
                .insert(GlobalTransform::from_translation(ahead(space, 50.0)));
            app.secs(0.6);
            assert!(
                perceived::<Smell>(&app, agent).get(prey).is_none(),
                "{space:?}: forgotten after its memory"
            );
        }
    }

    #[test]
    fn a_sustained_sense_of_your_own_is_active_then_lost() {
        for space in SPACES {
            let mut app = with_senses(space);
            let agent = app.spawn((Radar { range: 5.0 }, GlobalTransform::IDENTITY));
            let blip = app.spawn((
                Scented,
                GlobalTransform::from_translation(ahead(space, 3.0)),
            ));
            app.tick();
            assert!(
                perceived::<Radar>(&app, agent)
                    .get(blip)
                    .unwrap()
                    .is_active(),
                "{space:?}"
            );
            app.entity_mut(blip)
                .insert(GlobalTransform::from_translation(ahead(space, 50.0)));
            app.tick();
            let lost = perceived::<Radar>(&app, agent).get(blip).unwrap();
            assert!(!lost.is_active(), "{space:?}: lost");
            assert!(lost.trace().is_some(), "{space:?}: remembered");
        }
    }
}
