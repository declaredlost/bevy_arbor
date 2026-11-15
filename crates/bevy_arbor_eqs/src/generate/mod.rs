//! Generators: what a query weighs, places around an anchor or things.
//!
//! Places in a [`Donut`], [`Grid`] or [`Fan`] around an anchor, or in a [`Row`]
//! through it; things with a component near an anchor ([`Nearby`]), or the
//! agent perceives ([`Percepts`](crate::perceived::Percepts)); or a
//! [`Generate`] of your own.

mod donut;
mod fan;
mod grid;
mod nearby;
mod row;
mod shape;

use bevy::prelude::*;

pub use self::donut::Donut;
pub use self::fan::Fan;
pub use self::grid::Grid;
pub use self::nearby::Nearby;
pub use self::row::Row;
use crate::query::{EnvQueryContext, EnvQueryItem};

/// A kind of generator: what makes the items a query weighs.
///
/// What it keeps between runs in a world, a `QueryState` of the things it
/// finds, is its [`State`](Self::State), made there the first time its query
/// runs.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_arbor_eqs::generate::Generate;
/// # use bevy_arbor_eqs::query::{EnvQueryContext, EnvQueryItem};
/// #[derive(Component)]
/// struct Crate;
///
/// /// Every crate, wherever it is.
/// struct Crates;
///
/// impl Generate for Crates {
///     type State =
///         QueryState<(Entity, &'static GlobalTransform), With<Crate>>;
///
///     fn generate(
///         &self,
///         crates: &mut Self::State,
///         cx: &EnvQueryContext,
///         out: &mut Vec<EnvQueryItem>,
///     ) {
///         for (entity, transform) in cx.query(crates) {
///             let position = cx.space().flatten(transform.translation());
///             out.push(EnvQueryItem::thing(entity, position));
///         }
///     }
/// }
/// ```
pub trait Generate: Send + Sync + 'static {
    /// What it keeps in a world between runs: `()` if nothing.
    type State: FromWorld + Send + Sync + 'static;

    /// The items, for the agent of `cx`: pushed onto `out`.
    fn generate(&self, state: &mut Self::State, cx: &EnvQueryContext, out: &mut Vec<EnvQueryItem>);

    /// Gets its state ready before each run, with the world to change.
    fn prepare(&self, _state: &mut Self::State, _world: &mut World) {}
}

#[cfg(test)]
mod tests {
    use bevy_arbor_perception::prelude::*;
    use bevy_harness::TestApp;

    use super::*;
    use crate::anchor::{At, Querier};
    use crate::testing::*;

    fn generated_in(
        app: &mut TestApp,
        agent: Entity,
        generator: &impl Generate,
    ) -> Vec<EnvQueryItem> {
        generated(app.world_mut(), agent, generator)
    }

    fn points(items: &[EnvQueryItem]) -> Vec<Vec3> {
        items.iter().map(|item| item.position).collect()
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
            let points = points(&generated_in(&mut app, agent, &ring));
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
        let grid = Grid::around(Querier).extent(2.0).spacing(1.0);
        let points = points(&generated_in(&mut app, agent, &grid));
        assert_eq!(points.len(), 25);
        assert!(points.contains(&Vec3::new(-2.0, 0.0, 2.0)));
        assert!(points.iter().all(|point| point.y == 0.0));
    }

    #[test]
    fn a_fan_is_ahead_of_its_anchor() {
        for space in [Space::Planar, Space::Volume] {
            let mut app = app(space);
            let agent = app.spawn(GlobalTransform::IDENTITY);
            let cone = Fan::ahead_of(Querier)
                .angle(0.5)
                .radius(1.0, 4.0)
                .rays(3)
                .points(4);
            let points = points(&generated_in(&mut app, agent, &cone));
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
            let row = Row::along(Querier).extent(2.0).spacing(1.0);
            let points = points(&generated_in(&mut app, agent, &row));
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
        assert_eq!(generated_in(&mut app, agent, &row).len(), 0);
    }

    #[test]
    fn a_row_too_fine_is_spread_out_rather_than_hang() {
        let mut app = app(Space::Volume);
        let agent = app.spawn(GlobalTransform::IDENTITY);
        let row = Row::along(Querier).extent(5.0).spacing(0.0);
        assert_eq!(generated_in(&mut app, agent, &row).len(), 101);
    }

    #[test]
    fn a_fan_from_what_looks_nowhere_has_nothing() {
        let mut app = app(Space::Volume);
        let agent = app.spawn(GlobalTransform::IDENTITY);
        let cone = Fan::ahead_of(At(Vec3::ZERO));
        assert_eq!(generated_in(&mut app, agent, &cone).len(), 0);
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
        let nearby = Nearby::<Medkit>::within(Querier, 5.0);
        let found: Vec<_> = generated_in(&mut app, agent, &nearby)
            .iter()
            .filter_map(|item| item.entity)
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
        let grid = Grid::around(Querier).extent(5.0).spacing(0.0);
        let points = points(&generated_in(&mut app, agent, &grid));
        assert_eq!(points.len(), 101 * 101, "at most 50 each way");
        assert!(
            points.contains(&Vec3::new(5.0, 0.0, 5.0)),
            "out to its extent"
        );
    }
}
