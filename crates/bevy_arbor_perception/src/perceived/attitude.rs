use bevy::prelude::*;

/// How an agent stands toward what it perceives.
#[derive(Reflect, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[reflect(Debug, PartialEq, Hash, Default)]
pub enum Attitude {
    /// To be fought or fled.
    Hostile,
    /// Neither.
    #[default]
    Neutral,
    /// On its side.
    Friendly,
}

/// On an agent or what it perceives: its side, for the attitude nothing else
/// gave.
///
/// The fallback is: the same team is friendly, another is hostile, and either
/// side without one is neutral. A system in
/// [`PerceptionSystems::Classify`](crate::PerceptionSystems::Classify) can
/// say otherwise, and what it says stands.
#[derive(Component, Reflect, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[reflect(Component, Debug, PartialEq, Hash)]
pub struct Team(pub u8);

impl Team {
    /// How `mine` stands toward `theirs`, by teams alone.
    #[must_use]
    pub fn attitude(mine: Option<Self>, theirs: Option<Self>) -> Attitude {
        match (mine, theirs) {
            (Some(mine), Some(theirs)) if mine == theirs => Attitude::Friendly,
            (Some(_), Some(_)) => Attitude::Hostile,
            _ => Attitude::Neutral,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::prelude::*;

    use crate::prelude::*;
    use crate::testing::*;

    #[test]
    fn teams_give_the_attitude_nothing_else_did() {
        for space in SPACES {
            let mut app = app(space);
            let agent = app.spawn((
                Sight::new(10.0).with_interval(Duration::ZERO),
                Team(1),
                GlobalTransform::IDENTITY,
            ));
            let ally = app.spawn((
                Perceivable,
                Team(1),
                GlobalTransform::from_translation(ahead(space, 2.0)),
            ));
            let enemy = app.spawn((
                Perceivable,
                Team(2),
                GlobalTransform::from_translation(ahead(space, 3.0)),
            ));
            let stranger = thing(&mut app, ahead(space, 4.0));
            app.tick();
            let perceived = perceived::<Sight>(&app, agent);
            assert!(perceived.get(ally).unwrap().is_friendly(), "{space:?}");
            assert!(perceived.get(enemy).unwrap().is_hostile(), "{space:?}");
            assert_eq!(
                perceived.get(stranger).unwrap().attitude(),
                Attitude::Neutral,
                "{space:?}"
            );
        }
    }
}
