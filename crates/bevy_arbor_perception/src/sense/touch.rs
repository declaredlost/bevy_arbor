use std::time::Duration;

use bevy::prelude::*;

use super::Sense;
use crate::perceived::Candidates;
use crate::space::Space;

/// On an agent: it feels what touches it, a hit, a bump: what [`Touched`]
/// says.
#[derive(Component, Reflect, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[reflect(Component, Debug, Default, PartialEq)]
pub struct Touch;

impl Sense for Touch {
    /// Five seconds after it was felt.
    const MEMORY: Duration = Duration::from_secs(5);
}

/// `by` touched `agent`: a hit, a bump. Write it as a message, for an agent
/// with [`Touch`] to feel, where `by` is.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Touched {
    /// Who was touched.
    pub agent: Entity,
    /// What touched it.
    pub by: Entity,
}

/// Each touched agent feels what touched it.
pub(crate) fn feel(
    space: Res<Space>,
    mut touches: MessageReader<Touched>,
    places: Query<&GlobalTransform>,
    mut agents: Query<&mut Candidates<Touch>>,
) {
    for touch in touches.read() {
        if touch.agent == touch.by {
            continue;
        }
        let Ok(mut candidates) = agents.get_mut(touch.agent) else {
            continue;
        };
        // Where what touched it is; gone, where the agent felt it.
        let Some(position) = places
            .get(touch.by)
            .or_else(|_| places.get(touch.agent))
            .ok()
            .map(|transform| space.flatten(transform.translation()))
        else {
            continue;
        };
        candidates.sense(touch.by, position);
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use crate::prelude::*;
    use crate::testing::*;

    #[test]
    fn a_touch_is_felt_where_what_touched_is() {
        for space in SPACES {
            let mut app = app(space);
            let agent = app.spawn((Touch, Team(1), GlobalTransform::IDENTITY));
            let by = app.spawn((
                Team(2),
                GlobalTransform::from_translation(behind(space, 1.0)),
            ));
            app.write(Touched { agent, by });
            app.tick();
            let percept = perceived::<Touch>(&app, agent).get(by).unwrap();
            let felt = traces::<Touch>(&app, agent).get(by).unwrap();
            assert_eq!(
                felt.position,
                space.flatten(behind(space, 1.0)),
                "{space:?}"
            );
            assert!(percept.is_hostile(), "{space:?}: by team");
            assert!(!percept.is_active(), "{space:?}");
        }
    }

    #[test]
    fn something_that_perceives_nothing_feels_nothing() {
        for space in SPACES {
            let mut app = app(space);
            let rock = app.spawn(GlobalTransform::IDENTITY);
            let by = app.spawn(GlobalTransform::IDENTITY);
            app.write(Touched { agent: rock, by });
            app.tick();
            assert!(
                app.world().get::<Perceived<Touch>>(rock).is_none(),
                "{space:?}"
            );
        }
    }
}
