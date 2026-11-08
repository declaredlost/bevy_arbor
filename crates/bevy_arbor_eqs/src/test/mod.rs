//! Tests: what a query filters and scores items by. [`Distance`] to an anchor,
//! [`Dot`] from one, whether the agent [`Perceives`] a thing, [`Random`], a
//! system of yours as a [`Measure`], or a [`Test`] of your own.

mod distance;
mod dot;
mod measure;
mod perceives;
mod random;

use bevy::prelude::*;

pub use self::distance::Distance;
pub use self::dot::Dot;
pub use self::measure::{Batch, Measure};
pub use self::perceives::Perceives;
pub use self::random::Random;
use crate::query::{Item, QueryContext};

/// How costly a test is to run on an item: filters run cheapest first, so the
/// costly ones see fewer items.
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
pub trait Test: Send + Sync + 'static {
    /// Its number for `item`, for the agent of `cx`.
    fn test(&self, cx: &QueryContext, item: &Item) -> Option<f32>;

    /// Its number for each of `items`, in order: [`test`](Self::test) for
    /// each, unless it says otherwise. A test with setup per run, a lock or a
    /// cached system state, does that once here.
    fn test_all(&self, cx: &QueryContext, items: &[Item]) -> Vec<Option<f32>> {
        items.iter().map(|item| self.test(cx, item)).collect()
    }

    /// How costly it is.
    fn cost(&self) -> Cost {
        Cost::Low
    }

    /// Gets ready to run in `world`, before each query it's in: for what can
    /// only be set up with the world to change, as a cached system state.
    fn prepare(&self, _world: &mut World) {}
}

#[cfg(test)]
mod tests {
    use bevy_arbor_perception::prelude::*;
    use bevy_harness::TestApp;

    use super::*;
    use crate::anchor::{Agent, At, Hostiles};
    use crate::testing::*;

    fn tested(app: &TestApp, agent: Entity, test: &dyn Test, item: Item) -> Option<f32> {
        test.test(
            &QueryContext::new(app.world(), agent, Space::Volume, 1),
            &item,
        )
    }

    #[test]
    fn distance_is_to_the_nearest_of_its_anchor() {
        let mut app = app(Space::Volume);
        let agent = looker(&mut app);
        enemy(&mut app, Vec3::new(0.0, 0.0, -10.0));
        enemy(&mut app, Vec3::new(0.0, 0.0, -4.0));
        app.tick();
        let item = Item::Point(Vec3::new(0.0, 0.0, -1.0));
        assert_eq!(tested(&app, agent, &Distance::to(Agent), item), Some(1.0));
        assert_eq!(
            tested(
                &app,
                agent,
                &Distance::to(Hostiles::<Sight>::active()),
                item
            ),
            Some(3.0),
            "the nearer enemy"
        );
        assert_eq!(
            tested(
                &app,
                agent,
                &Distance::to(Hostiles::<Hearing>::active()),
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
        let facing = Dot::of(Agent);
        let ahead = tested(&app, agent, &facing, Item::Point(Vec3::new(0.0, 0.0, -3.0)));
        let behind = tested(&app, agent, &facing, Item::Point(Vec3::new(0.0, 0.0, 3.0)));
        let side = tested(&app, agent, &facing, Item::Point(Vec3::new(3.0, 0.0, 0.0)));
        assert!((ahead.unwrap() - 1.0).abs() < 1e-5);
        assert!((behind.unwrap() + 1.0).abs() < 1e-5);
        assert!(side.unwrap().abs() < 1e-5);
        assert_eq!(
            tested(&app, agent, &Dot::of(At(Vec3::ZERO)), Item::Point(Vec3::X)),
            None,
            "a place looks nowhere"
        );
    }

    #[test]
    fn perceives_is_yes_or_no_for_things_only() {
        let mut app = app(Space::Volume);
        let agent = looker(&mut app);
        let seen = enemy(&mut app, Vec3::new(0.0, 0.0, -5.0));
        let unseen = enemy(&mut app, Vec3::new(0.0, 0.0, 5.0));
        app.tick();
        let test = Perceives::<Sight>::at_all().active().hostile();
        let thing = |entity| Item::Entity {
            entity,
            position: Vec3::ZERO,
        };
        assert_eq!(tested(&app, agent, &test, thing(seen)), Some(1.0));
        assert_eq!(tested(&app, agent, &test, thing(unseen)), Some(0.0));
        assert_eq!(tested(&app, agent, &test, Item::Point(Vec3::ZERO)), None);
    }

    #[test]
    fn random_is_from_zero_up_to_one_and_varies() {
        let app = app(Space::Volume);
        let cx = QueryContext::new(app.world(), Entity::PLACEHOLDER, Space::Volume, 7);
        let rolls: Vec<f32> = (0..100)
            .map(|_| Random.test(&cx, &Item::Point(Vec3::ZERO)).unwrap())
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
        let side = tested(
            &app,
            agent,
            &Dot::of(Agent),
            Item::Point(Vec3::new(3.0, 0.0, 0.0)),
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
        let facing = Dot::of(Hostiles::<Sight>::remembered());
        assert_eq!(
            tested(&app, agent, &facing, Item::Point(Vec3::ZERO)),
            None,
            "where it was is known, which way it faces now isn't"
        );
    }

    /// An anchor counting how often it's located.
    struct Counted(std::sync::Arc<std::sync::atomic::AtomicUsize>);

    impl crate::anchor::Locate for Counted {
        fn locate(&self, _cx: &QueryContext, out: &mut Vec<crate::anchor::Located>) {
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
        let app = app(Space::Volume);
        let count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let test = Distance::to(Counted(count.clone()));
        let cx = QueryContext::new(app.world(), Entity::PLACEHOLDER, Space::Volume, 1);
        let items: Vec<Item> = (0..5u8)
            .map(|x| Item::Point(Vec3::new(f32::from(x), 0.0, 0.0)))
            .collect();
        test.test_all(&cx, &items);
        assert_eq!(count.load(std::sync::atomic::Ordering::Relaxed), 1);
    }
}
