//! Generators: what a query weighs. Places in a [`Donut`], [`Grid`] or
//! [`Fan`] around an anchor, or in a [`Row`] through it; things the agent
//! perceives ([`Percepts`]), or things with a component near an anchor
//! ([`Nearby`]); or a [`Generate`] of your own, keeping its Bevy query in a
//! [`CachedQuery`].

mod cached;
mod donut;
mod fan;
mod grid;
mod row;
mod things;

use std::f32::consts::TAU;

use bevy::prelude::*;
use bevy_arbor_perception::Space;

pub use self::cached::CachedQuery;
pub use self::donut::Donut;
pub use self::fan::Fan;
pub use self::grid::Grid;
pub use self::row::Row;
pub use self::things::{Nearby, Percepts};
use crate::query::{Item, QueryContext};

/// A kind of generator: what makes the items a query weighs.
pub trait Generate: Send + Sync + 'static {
    /// The items, for the agent of `cx`: pushed onto `out`.
    fn generate(&self, cx: &QueryContext, out: &mut Vec<Item>);
}

/// The point `radius` from `center` at `angle` round it, in the plane things
/// stand on: XY in 2D, XZ in 3D, where 0 is +X.
pub(crate) fn round(space: Space, center: Vec3, angle: f32, radius: f32) -> Vec3 {
    let (sin, cos) = angle.sin_cos();
    match space {
        Space::Planar => center + Vec3::new(cos, sin, 0.0) * radius,
        Space::Volume => center + Vec3::new(cos, 0.0, -sin) * radius,
    }
}

/// How many points a grid or row has each way of its anchor, at most: one
/// asking for more is spread further apart, so a tiny spacing can't hang the
/// game.
pub(crate) const MOST_STEPS: i32 = 50;

/// How many points each way, and how far apart, out to `extent`, `spacing`
/// apart; if that's more than [`MOST_STEPS`], how far apart they're spread
/// instead.
pub(crate) fn steps(extent: f32, spacing: f32) -> Result<(i32, f32), f32> {
    let wanted = (extent / spacing).floor();
    #[expect(clippy::cast_precision_loss, reason = "a small step count")]
    let most = MOST_STEPS as f32;
    if wanted > most {
        return Err(extent / most);
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "at most MOST_STEPS, and never negative"
    )]
    Ok((wanted as i32, spacing))
}

/// `count` angles evenly round a circle.
pub(crate) fn angles(count: u32) -> impl Iterator<Item = f32> {
    #[expect(
        clippy::cast_precision_loss,
        reason = "a count of points, far below where f32 loses whole numbers"
    )]
    let step = TAU / count.max(1) as f32;
    #[expect(clippy::cast_precision_loss, reason = "as above")]
    (0..count).map(move |index| index as f32 * step)
}

#[cfg(test)]
mod tests {
    use bevy_arbor_perception::prelude::*;
    use bevy_harness::TestApp;

    use super::*;
    use crate::anchor::{Agent, At};
    use crate::testing::*;

    fn generated(
        app: &TestApp,
        agent: Entity,
        space: Space,
        generator: &dyn Generate,
    ) -> Vec<Item> {
        let cx = QueryContext::new(app.world(), agent, space, 1);
        let mut out = Vec::new();
        generator.generate(&cx, &mut out);
        out
    }

    fn points(items: &[Item]) -> Vec<Vec3> {
        items.iter().map(Item::position).collect()
    }

    #[test]
    fn a_donut_is_round_its_anchor_in_the_plane_things_stand_on() {
        for space in [Space::Planar, Space::Volume] {
            let mut app = app(space);
            let agent = app.spawn(GlobalTransform::IDENTITY);
            let ring = Donut::around(At(Vec3::new(1.0, 2.0, 0.0)))
                .radius(2.0, 4.0)
                .rings(2)
                .points(8);
            let points = points(&generated(&app, agent, space, &ring));
            assert_eq!(points.len(), 16, "{space:?}");
            let center = space.flatten(Vec3::new(1.0, 2.0, 0.0));
            for point in &points {
                let distance = point.distance(center);
                assert!(
                    (distance - 2.0).abs() < 1e-4 || (distance - 4.0).abs() < 1e-4,
                    "{space:?}: {distance}"
                );
                match space {
                    Space::Planar => assert!(point.z.abs() < 1e-6),
                    Space::Volume => assert!((point.y - 2.0).abs() < 1e-6, "on its level"),
                }
            }
        }
    }

    #[test]
    fn a_grid_covers_its_extent() {
        let mut app = app(Space::Volume);
        let agent = app.spawn(GlobalTransform::IDENTITY);
        let grid = Grid::around(Agent).extent(2.0).spacing(1.0);
        let points = points(&generated(&app, agent, Space::Volume, &grid));
        assert_eq!(points.len(), 25);
        assert!(points.contains(&Vec3::new(-2.0, 0.0, 2.0)));
        assert!(points.iter().all(|point| point.y == 0.0));
    }

    #[test]
    fn a_fan_is_ahead_of_its_anchor() {
        for space in [Space::Planar, Space::Volume] {
            let mut app = app(space);
            let agent = app.spawn(GlobalTransform::IDENTITY);
            let cone = Fan::from(Agent).angle(0.5).range(1.0, 4.0).rays(3, 4);
            let points = points(&generated(&app, agent, space, &cone));
            assert_eq!(points.len(), 12, "{space:?}");
            let ahead = match space {
                Space::Planar => Vec3::X,
                Space::Volume => Vec3::NEG_Z,
            };
            for point in &points {
                let cos = point.normalize().dot(ahead);
                assert!(
                    cos >= 0.25f32.cos() - 1e-4,
                    "{space:?}: within the cone, {cos}"
                );
            }
        }
    }

    #[test]
    fn a_row_is_behind_and_ahead_of_its_anchor_the_way_it_looks() {
        for space in [Space::Planar, Space::Volume] {
            let mut app = app(space);
            let agent = app.spawn(GlobalTransform::from_xyz(1.0, 2.0, 0.0));
            let row = Row::along(Agent).extent(2.0).spacing(1.0);
            let points = points(&generated(&app, agent, space, &row));
            let ahead = match space {
                Space::Planar => Vec3::X,
                Space::Volume => Vec3::NEG_Z,
            };
            let center = Vec3::new(1.0, 2.0, 0.0);
            let expected: Vec<Vec3> = [-2.0, -1.0, 0.0, 1.0, 2.0]
                .into_iter()
                .map(|out_by| center + ahead * out_by)
                .collect();
            assert_eq!(points, expected, "{space:?}");
        }
    }

    #[test]
    fn a_row_through_what_looks_nowhere_has_nothing() {
        let mut app = app(Space::Volume);
        let agent = app.spawn(GlobalTransform::IDENTITY);
        let row = Row::along(At(Vec3::ZERO));
        assert_eq!(generated(&app, agent, Space::Volume, &row).len(), 0);
    }

    #[test]
    fn a_row_too_fine_is_spread_out_rather_than_hang() {
        let mut app = app(Space::Volume);
        let agent = app.spawn(GlobalTransform::IDENTITY);
        let row = Row::along(Agent).extent(5.0).spacing(0.0);
        assert_eq!(generated(&app, agent, Space::Volume, &row).len(), 101);
    }

    #[test]
    fn a_fan_from_what_looks_nowhere_has_nothing() {
        let mut app = app(Space::Volume);
        let agent = app.spawn(GlobalTransform::IDENTITY);
        let cone = Fan::from(At(Vec3::ZERO));
        assert_eq!(generated(&app, agent, Space::Volume, &cone).len(), 0);
    }

    #[test]
    fn percepts_are_what_the_agent_perceives() {
        let mut app = app(Space::Volume);
        let agent = looker(&mut app);
        let enemy = enemy(&mut app, Vec3::new(0.0, 0.0, -5.0));
        let friend = app.spawn((
            Perceivable,
            Team(1),
            GlobalTransform::from_xyz(1.0, 0.0, -5.0),
        ));
        app.tick();
        let all = generated(&app, agent, Space::Volume, &Percepts::<Sight>::all());
        let all: Vec<_> = all.iter().filter_map(Item::entity).collect();
        assert!(all.contains(&enemy) && all.contains(&friend));
        let hostile = generated(
            &app,
            agent,
            Space::Volume,
            &Percepts::<Sight>::all().hostile(),
        );
        assert_eq!(
            hostile.iter().filter_map(Item::entity).collect::<Vec<_>>(),
            [enemy]
        );
    }

    #[derive(Component)]
    struct Medkit;

    #[test]
    fn nearby_is_what_has_the_component_within_range() {
        let mut app = app(Space::Volume);
        let agent = app.spawn((Medkit, GlobalTransform::IDENTITY));
        let near = app.spawn((Medkit, GlobalTransform::from_xyz(3.0, 0.0, 0.0)));
        app.spawn((Medkit, GlobalTransform::from_xyz(30.0, 0.0, 0.0)));
        app.spawn(GlobalTransform::from_xyz(1.0, 0.0, 0.0));
        let nearby = Nearby::<Medkit>::within(Agent, 5.0);
        let found: Vec<_> = generated(&app, agent, Space::Volume, &nearby)
            .iter()
            .filter_map(Item::entity)
            .collect();
        assert_eq!(
            found,
            [near],
            "not the agent's own, nor what's far, nor without one"
        );
    }

    #[test]
    fn a_grid_too_fine_is_spread_out_rather_than_hang() {
        let mut app = app(Space::Volume);
        let agent = app.spawn(GlobalTransform::IDENTITY);
        let grid = Grid::around(Agent).extent(5.0).spacing(0.0);
        let points = points(&generated(&app, agent, Space::Volume, &grid));
        assert_eq!(points.len(), 101 * 101, "at most 50 each way");
        assert!(
            points.contains(&Vec3::new(5.0, 0.0, 5.0)),
            "out to its extent"
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
        let items = generated(&app, agent, Space::Volume, &Percepts::<Sight>::all());
        assert_eq!(
            points(&items),
            [Vec3::new(0.0, 0.0, 5.0)],
            "where it was lost, not where it really is now"
        );
    }
}
