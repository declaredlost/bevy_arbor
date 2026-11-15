//! What the agent perceives, in queries: as an anchor, items, or a test.
//!
//! [`Percepts`] is what it perceives by a sense, an anchor or a generator;
//! [`Perceives`], a test of whether it perceives a thing. Both narrow to what's
//! active, hostile, or both, alike.

mod perceives;
mod percepts;
mod which;

pub use self::perceives::Perceives;
pub use self::percepts::Percepts;
use self::which::Which;

#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use bevy_arbor_perception::prelude::*;
    use bevy_harness::TestApp;

    use super::{Perceives, Percepts};
    use crate::anchor::Locate;
    use crate::query::{EnvQueryContext, EnvQueryItem};
    use crate::testing::*;

    /// Where `anchor` is, for `agent`.
    fn located(app: &mut TestApp, agent: Entity, anchor: &dyn Locate) -> Vec<Vec3> {
        let cx = EnvQueryContext::new(app.world(), agent, Space::Volume, 1);
        let mut out = Vec::new();
        anchor.locate(&cx, &mut out);
        out.into_iter().map(|located| located.position).collect()
    }

    fn points(items: &[EnvQueryItem]) -> Vec<Vec3> {
        items.iter().map(|item| item.position).collect()
    }

    fn tested_one(
        app: &mut TestApp,
        agent: Entity,
        test: &impl crate::test::Test,
        item: EnvQueryItem,
    ) -> Option<f32> {
        tested(app.world_mut(), agent, test, &[item])[0]
    }

    #[test]
    fn as_an_anchor_they_are_where_they_are_or_were_last_seen() {
        let mut app = app(Space::Volume);
        let agent = looker(&mut app);
        let enemy = enemy(&mut app, Vec3::new(0.0, 0.0, -5.0));
        app.tick();
        assert_eq!(
            located(
                &mut app,
                agent,
                &Percepts::<Sight>::all().active().hostile()
            ),
            [Vec3::new(0.0, 0.0, -5.0)]
        );
        app.entity_mut(enemy)
            .insert(GlobalTransform::from_xyz(0.0, 0.0, 50.0));
        app.tick();
        assert!(
            located(
                &mut app,
                agent,
                &Percepts::<Sight>::all().active().hostile()
            )
            .is_empty(),
            "out of view"
        );
        assert_eq!(
            located(&mut app, agent, &Percepts::<Sight>::all().hostile()).len(),
            1,
            "where it was last seen"
        );
    }

    #[test]
    fn percepts_are_what_the_agent_perceives() {
        let mut app = app(Space::Volume);
        let agent = looker(&mut app);
        let enemy = enemy(&mut app, Vec3::new(0.0, 0.0, -5.0));
        let friend = app.spawn((
            Perceivable,
            Allegiance(1),
            GlobalTransform::from_xyz(1.0, 0.0, -5.0),
        ));
        app.tick();
        let all = generated(app.world_mut(), agent, &Percepts::<Sight>::all());
        let all: Vec<_> = all.iter().filter_map(|item| item.entity).collect();
        assert!(all.contains(&enemy) && all.contains(&friend));
        let hostile = generated(app.world_mut(), agent, &Percepts::<Sight>::all().hostile());
        assert_eq!(
            hostile
                .iter()
                .filter_map(|item| item.entity)
                .collect::<Vec<_>>(),
            [enemy]
        );
    }

    #[test]
    fn a_percept_only_remembered_is_where_it_was_last_seen() {
        let mut app = app(Space::Volume);
        let agent = looker(&mut app);
        let enemy = enemy(&mut app, Vec3::new(0.0, 0.0, -5.0));
        app.tick();
        // Lost behind the agent, then gone on elsewhere unseen.
        app.entity_mut(enemy)
            .insert(GlobalTransform::from_xyz(0.0, 0.0, 5.0));
        app.tick();
        app.entity_mut(enemy)
            .insert(GlobalTransform::from_xyz(0.0, 0.0, 50.0));
        app.tick();
        let items = generated(app.world_mut(), agent, &Percepts::<Sight>::all());
        assert_eq!(
            points(&items),
            [Vec3::new(0.0, 0.0, 5.0)],
            "where it was lost, not where it really is now"
        );
    }

    #[test]
    fn perceives_is_yes_or_no_for_things_only() {
        let mut app = app(Space::Volume);
        let agent = looker(&mut app);
        let seen = enemy(&mut app, Vec3::new(0.0, 0.0, -5.0));
        let unseen = enemy(&mut app, Vec3::new(0.0, 0.0, 5.0));
        app.tick();
        let test = Perceives::<Sight>::all().active().hostile();
        let thing = |entity| EnvQueryItem::thing(entity, Vec3::ZERO);
        assert_eq!(tested_one(&mut app, agent, &test, thing(seen)), Some(1.0));
        assert_eq!(tested_one(&mut app, agent, &test, thing(unseen)), Some(0.0));
        assert_eq!(
            tested_one(&mut app, agent, &test, EnvQueryItem::place(Vec3::ZERO)),
            None
        );
    }
}
