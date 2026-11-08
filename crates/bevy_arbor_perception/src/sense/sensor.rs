use bevy::prelude::*;

/// On a sensor: the agent it senses for, as a Bevy relationship.
///
/// A sensor is an entity with senses of its own (eyes on a head, a rear
/// motion sensor), looking from its own transform. What it senses goes to its
/// agent's `Perceived`. Despawning the agent despawns its sensors.
#[derive(Component, Reflect, Debug, Clone, Copy, PartialEq, Eq)]
#[reflect(Component, Debug, PartialEq)]
#[relationship(relationship_target = Sensors)]
pub struct SensorOf(pub Entity);

impl SensorOf {
    /// The agent it senses for.
    #[must_use]
    pub fn agent(&self) -> Entity {
        self.0
    }
}

/// On an agent: the sensors that sense for it, kept by [`SensorOf`].
#[derive(Component, Reflect, Debug, Default)]
#[reflect(Component, Debug)]
#[relationship_target(relationship = SensorOf, linked_spawn)]
pub struct Sensors(Vec<Entity>);

impl Sensors {
    /// The sensors.
    pub fn iter(&self) -> impl Iterator<Item = Entity> + '_ {
        self.0.iter().copied()
    }
}

/// The agent `sensor` senses for: its [`SensorOf`], or itself.
pub(crate) fn agent(sensor: Entity, of: Option<&SensorOf>) -> Entity {
    of.map_or(sensor, SensorOf::agent)
}

#[cfg(test)]
mod tests {
    use std::f32::consts::PI;
    use std::time::Duration;

    use bevy::prelude::*;

    use crate::prelude::*;
    use crate::testing::*;

    /// A sensor for `agent` at the origin, turned to look backwards, every
    /// tick.
    fn rear_eye(app: &mut TestApp, agent: Entity, space: Space) -> Entity {
        let turned = match space {
            Space::Planar => GlobalTransform::from_scale(Vec3::new(-1.0, 1.0, 1.0)),
            Space::Volume => GlobalTransform::from_rotation(Quat::from_rotation_y(PI)),
        };
        app.spawn((
            Sight::new(10.0).with_interval(Duration::ZERO),
            SensorOf(agent),
            turned,
        ))
    }

    use bevy_harness::TestApp;

    #[test]
    fn a_sensor_sees_for_its_agent_from_its_own_transform() {
        for space in SPACES {
            let mut app = app(space);
            let agent = app.spawn(GlobalTransform::IDENTITY);
            let eye = rear_eye(&mut app, agent, space);
            let back = thing(&mut app, behind(space, 5.0));
            app.tick();
            assert!(sees(&app, agent, back), "{space:?}: its agent sees it");
            assert!(
                app.world().get::<Perceived<Sight>>(eye).is_none(),
                "{space:?}: the sensor keeps nothing itself"
            );
        }
    }

    #[test]
    fn what_any_of_its_sensors_sees_stays_in_view() {
        for space in SPACES {
            let mut app = logged(space);
            let agent = looker(&mut app, Sight::new(10.0));
            rear_eye(&mut app, agent, space);
            let target = thing(&mut app, ahead(space, 5.0));
            app.tick();
            assert!(sees(&app, agent, target), "{space:?}: in front");
            app.entity_mut(target)
                .insert(GlobalTransform::from_translation(behind(space, 5.0)));
            app.tick();
            assert!(
                sees(&app, agent, target),
                "{space:?}: behind, by the rear eye"
            );
            app.entity_mut(target)
                .insert(GlobalTransform::from_translation(ahead(space, 50.0)));
            app.tick();
            assert!(!sees(&app, agent, target), "{space:?}: neither sees it");
            let lost = log(&app)
                .iter()
                .filter(|(event, ..)| *event == "lost")
                .count();
            assert_eq!(lost, 1, "{space:?}: lost once, when both had");
        }
    }

    #[test]
    fn a_sensor_does_not_hear_its_own_agent() {
        for space in SPACES {
            let mut app = app(space);
            let agent = app.spawn(GlobalTransform::IDENTITY);
            app.spawn((
                Hearing::new(10.0),
                SensorOf(agent),
                GlobalTransform::IDENTITY,
            ));
            let other = app.spawn(GlobalTransform::IDENTITY);
            app.write(Noise::new(agent, ahead(space, 1.0)));
            app.write(Noise::new(other, ahead(space, 1.0)));
            app.tick();
            let heard = perceived::<Hearing>(&app, agent);
            assert!(heard.get(agent).is_none(), "{space:?}: its own");
            assert!(heard.get(other).is_some(), "{space:?}");
        }
    }

    #[test]
    fn a_sensor_attached_later_gives_its_agent_what_it_senses() {
        for space in SPACES {
            let mut app = app(space);
            let agent = app.spawn(GlobalTransform::IDENTITY);
            let eye = app.spawn((
                Sight::new(10.0).with_interval(Duration::ZERO),
                GlobalTransform::IDENTITY,
            ));
            let target = thing(&mut app, ahead(space, 5.0));
            app.tick();
            assert!(
                sees(&app, eye, target),
                "{space:?}: on its own, it's an agent"
            );
            app.entity_mut(eye).insert(SensorOf(agent));
            app.tick();
            assert!(sees(&app, agent, target), "{space:?}: now for its agent");
        }
    }

    #[test]
    fn despawning_the_agent_despawns_its_sensors() {
        for space in SPACES {
            let mut app = app(space);
            let agent = app.spawn(GlobalTransform::IDENTITY);
            let eye = rear_eye(&mut app, agent, space);
            app.tick();
            app.entity_mut(agent).despawn();
            app.tick();
            assert!(app.world().get_entity(eye).is_err(), "{space:?}");
        }
    }

    #[test]
    fn what_only_a_gone_sensor_saw_is_lost() {
        for space in SPACES {
            let mut app = logged(space);
            let agent = app.spawn(GlobalTransform::IDENTITY);
            let eye = rear_eye(&mut app, agent, space);
            let target = thing(&mut app, behind(space, 5.0));
            app.tick();
            assert!(sees(&app, agent, target), "{space:?}: seen");
            app.entity_mut(eye).despawn();
            app.tick();
            assert!(!sees(&app, agent, target), "{space:?}: its eye gone");
            assert!(
                log(&app).iter().any(|(event, ..)| *event == "lost"),
                "{space:?}: lost"
            );
        }
    }

    #[test]
    fn what_a_sense_taken_off_saw_is_lost() {
        for space in SPACES {
            let mut app = app(space);
            let agent = looker(&mut app, Sight::new(10.0));
            let target = thing(&mut app, ahead(space, 5.0));
            app.tick();
            assert!(sees(&app, agent, target), "{space:?}");
            app.entity_mut(agent).remove::<Sight>();
            app.tick();
            assert!(!sees(&app, agent, target), "{space:?}: blind now");
        }
    }

    #[test]
    fn attached_a_sensor_is_no_agent_of_its_own_and_detached_it_is_again() {
        for space in SPACES {
            let mut app = logged(space);
            let agent = app.spawn(GlobalTransform::IDENTITY);
            let eye = app.spawn((
                Sight::new(10.0).with_interval(Duration::ZERO),
                GlobalTransform::IDENTITY,
            ));
            thing(&mut app, ahead(space, 5.0));
            app.tick();
            app.entity_mut(eye).insert(SensorOf(agent));
            app.tick();
            assert!(
                app.world().get::<Perceived<Sight>>(eye).is_none(),
                "{space:?}: its agent's, not its own"
            );
            app.entity_mut(eye).remove::<SensorOf>();
            app.tick();
            assert!(
                app.world().get::<Perceived<Sight>>(eye).is_some(),
                "{space:?}: its own again"
            );
            assert!(
                perceived::<Sight>(&app, agent).active().next().is_none(),
                "{space:?}: the agent lost what it saw"
            );
        }
    }
}
