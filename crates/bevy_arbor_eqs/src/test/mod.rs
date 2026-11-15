//! Tests: what a query filters and scores items by, built in or your own.
//!
//! [`Distance`] to an anchor, [`Dot`] from one, whether the agent
//! [`Perceives`](crate::perceived::Perceives) a thing, [`Random`], a system of
//! yours as a [`Measure`], or a [`Test`] of your own.

mod distance;
mod dot;
mod measure;
mod random;

use bevy::prelude::*;

pub use self::distance::Distance;
pub use self::dot::Dot;
pub use self::measure::{Batch, Measure, Measuring};
pub use self::random::Random;
use crate::query::{EnvQueryContext, EnvQueryItem};

/// How costly a test is to run on an item: filters run cheapest first.
///
/// The costly ones then see fewer items.
#[derive(Reflect, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
#[reflect(Debug, PartialEq, Default)]
pub enum Cost {
    /// Arithmetic: a distance, a dot product.
    #[default]
    Low,
    /// A look at a component or two.
    Medium,
    /// A ray cast, a path.
    High,
}

/// A kind of test: a number for each item, which a query filters or scores by.
///
/// A yes or no is 1 or 0. `None` is no answer: the item fails a filter on it,
/// and scores nothing by it.
///
/// It's asked about every item at once, so what's costly to set up, a ray
/// query or a path search, is set up once a run. What it keeps between runs in
/// a world, a `SystemState`, a `QueryState`, is its [`State`](Self::State):
/// made there the first time its query runs, as Bevy makes a system's.
pub trait Test: Send + Sync + 'static {
    /// What it keeps in a world between runs: `()` if nothing.
    type State: FromWorld + Send + Sync + 'static;

    /// Its number for each of `items`, in order, for the agent of `cx`.
    fn test(
        &self,
        state: &mut Self::State,
        cx: &EnvQueryContext,
        items: &[EnvQueryItem],
    ) -> Vec<Option<f32>>;

    /// How costly it is.
    fn cost(&self) -> Cost {
        Cost::Low
    }

    /// Gets its state ready before each run, with the world to change.
    ///
    /// For what can only be done that way: initializing a system, or seeing
    /// what it reads replaced.
    fn prepare(&self, _state: &mut Self::State, _world: &mut World) {}
}

#[cfg(test)]
mod tests {
    use bevy_arbor_perception::prelude::*;
    use bevy_harness::TestApp;

    use super::*;
    use crate::anchor::{At, Querier};
    use crate::perceived::Percepts;
    use crate::testing::*;

    fn tested_one(
        app: &mut TestApp,
        agent: Entity,
        test: &impl Test,
        item: EnvQueryItem,
    ) -> Option<f32> {
        tested(app.world_mut(), agent, test, &[item])[0]
    }

    #[test]
    fn distance_is_to_the_nearest_of_its_anchor() {
        let mut app = app(Space::Volume);
        let agent = looker(&mut app);
        enemy(&mut app, Vec3::new(0.0, 0.0, -10.0));
        enemy(&mut app, Vec3::new(0.0, 0.0, -4.0));
        app.tick();
        let item = EnvQueryItem::place(Vec3::new(0.0, 0.0, -1.0));
        assert_eq!(
            tested_one(&mut app, agent, &Distance::to(Querier), item),
            Some(1.0)
        );
        assert_eq!(
            tested_one(
                &mut app,
                agent,
                &Distance::to(Percepts::<Sight>::all().active().hostile()),
                item
            ),
            Some(3.0),
            "the nearer enemy"
        );
        assert_eq!(
            tested_one(
                &mut app,
                agent,
                &Distance::to(Percepts::<Hearing>::all().active().hostile()),
                item
            ),
            None,
            "nowhere"
        );
    }

    #[test]
    fn dot_is_how_much_an_anchor_looks_toward_it() {
        let mut app = app(Space::Volume);
        let agent = app.spawn(GlobalTransform::IDENTITY);
        let facing = Dot::of(Querier);
        let ahead = tested_one(
            &mut app,
            agent,
            &facing,
            EnvQueryItem::place(Vec3::new(0.0, 0.0, -3.0)),
        );
        let behind = tested_one(
            &mut app,
            agent,
            &facing,
            EnvQueryItem::place(Vec3::new(0.0, 0.0, 3.0)),
        );
        let side = tested_one(
            &mut app,
            agent,
            &facing,
            EnvQueryItem::place(Vec3::new(3.0, 0.0, 0.0)),
        );
        assert!((ahead.unwrap() - 1.0).abs() < 1e-5);
        assert!((behind.unwrap() + 1.0).abs() < 1e-5);
        assert!(side.unwrap().abs() < 1e-5);
        assert_eq!(
            tested_one(
                &mut app,
                agent,
                &Dot::of(At(Vec3::ZERO)),
                EnvQueryItem::place(Vec3::X)
            ),
            None,
            "a place looks nowhere"
        );
    }

    #[test]
    fn random_is_from_zero_up_to_one_and_varies() {
        let app = app(Space::Volume);
        let cx = EnvQueryContext::new(app.world(), Entity::PLACEHOLDER, Space::Volume, 7);
        let items = [EnvQueryItem::place(Vec3::ZERO); 100];
        let rolls: Vec<f32> = Random
            .test(&mut (), &cx, &items)
            .into_iter()
            .flatten()
            .collect();
        assert!(rolls.iter().all(|roll| (0.0..1.0).contains(roll)));
        assert!(
            rolls
                .windows(2)
                .any(|pair| (pair[0] - pair[1]).abs() > f32::EPSILON)
        );
    }

    #[test]
    fn dot_follows_facing_where_the_transform_does_not_turn() {
        let mut app = app(Space::Volume);
        let agent = app.spawn((GlobalTransform::IDENTITY, Facing(Dir3::X)));
        let side = tested_one(
            &mut app,
            agent,
            &Dot::of(Querier),
            EnvQueryItem::place(Vec3::new(3.0, 0.0, 0.0)),
        );
        assert!((side.unwrap() - 1.0).abs() < 1e-5, "it faces +X: {side:?}");
    }

    #[test]
    fn a_hostile_only_remembered_faces_no_way_known() {
        let mut app = app(Space::Volume);
        let agent = looker(&mut app);
        let enemy = enemy(&mut app, Vec3::new(0.0, 0.0, -5.0));
        app.tick();
        app.entity_mut(enemy)
            .insert(GlobalTransform::from_xyz(0.0, 0.0, 50.0));
        app.tick();
        let facing = Dot::of(Percepts::<Sight>::all().hostile());
        assert_eq!(
            tested_one(&mut app, agent, &facing, EnvQueryItem::place(Vec3::ZERO)),
            None,
            "where it was is known, which way it faces now isn't"
        );
    }

    /// An anchor counting how often it's located.
    struct Counted(std::sync::Arc<std::sync::atomic::AtomicUsize>);

    impl crate::anchor::Locate for Counted {
        fn locate(&self, _cx: &EnvQueryContext, out: &mut Vec<crate::anchor::Located>) {
            self.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            out.push(crate::anchor::Located {
                position: Vec3::ZERO,
                ahead: Vec3::ZERO,
                entity: None,
            });
        }
    }

    #[test]
    fn an_anchor_is_located_once_a_run_however_many_items() {
        let mut app = app(Space::Volume);
        let count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let test = Distance::to(Counted(count.clone()));
        let items: Vec<EnvQueryItem> = (0..5u8)
            .map(|x| EnvQueryItem::place(Vec3::new(f32::from(x), 0.0, 0.0)))
            .collect();
        tested(app.world_mut(), Entity::PLACEHOLDER, &test, &items);
        assert_eq!(count.load(std::sync::atomic::Ordering::Relaxed), 1);
    }
}
