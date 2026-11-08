use std::sync::Arc;

use bevy::prelude::*;
use bevy_arbor_eqs::anchor::{Locate, Located};
use bevy_arbor_eqs::query::{Item, QueryContext};
use bevy_arbor_eqs::test::{Cost, Test};

use super::Reads;
use crate::avian::prelude::*;
use crate::dim;
use crate::sight::{Occluders, SensorOccluders, under};

/// What casting a ray reads.
type Rays = (
    SpatialQuery<'static, 'static>,
    Option<Res<'static, Occluders>>,
    Query<'static, 'static, &'static SensorOccluders>,
    Query<'static, 'static, &'static ColliderOf>,
    Query<'static, 'static, &'static ChildOf>,
    Query<'static, 'static, (), With<Sensor>>,
);

/// Whether an anchor can see an item: 1 if a ray from it gets there, from
/// any of its places, and 0 if not. No answer where it's nowhere.
///
/// What blocks the ray is what blocks sight: what's on [`Occluders`]' layers,
/// or the anchor's own [`SensorOccluders`] where it's a thing with them. The
/// agent's own colliders, the anchor's, trigger volumes (avian's `Sensor`),
/// and a thing item's own colliders don't. Costly, a ray an item: filter
/// cheaply first.
///
/// `LineOfSight::from(Hostiles::<Sight>::remembered())`, filtered to 0, keeps
/// what the enemy can't see, from where it is or was last seen: cover.
pub struct LineOfSight {
    from: Arc<dyn Locate>,
    rays: Reads<Rays>,
}

impl LineOfSight {
    /// Whether `from` can see each item.
    pub fn from(from: impl Locate) -> Self {
        Self {
            from: Arc::new(from),
            rays: Reads::new(),
        }
    }
}

impl Test for LineOfSight {
    fn test(&self, cx: &QueryContext, item: &Item) -> Option<f32> {
        self.test_all(cx, std::slice::from_ref(item))
            .into_iter()
            .next()
            .flatten()
    }

    fn test_all(&self, cx: &QueryContext, items: &[Item]) -> Vec<Option<f32>> {
        let anchors = cx.locate(&self.from);
        if anchors.is_empty() {
            return vec![None; items.len()];
        }
        self.rays
            .with(
                cx.world(),
                |(spatial, occluders, own, colliders, parents, triggers)| {
                    let everywhere = occluders.map_or(LayerMask::ALL, |occluders| occluders.0);
                    let agent = cx.agent();
                    // Whether a ray from `anchor` gets to `item`.
                    let sees = |anchor: &Located, item: &Item| {
                        let mask = anchor
                            .entity
                            .and_then(|entity| own.get(entity).ok())
                            .map_or(everywhere, |own| own.0);
                        let filter = SpatialQueryFilter::from_mask(mask);
                        // Neither the agent's own colliders nor the anchor's,
                        // where the ray starts, nor
                        // trigger volumes.
                        let blocks = |entity: Entity| {
                            !triggers.contains(entity)
                                && !under(&colliders, &parents, entity, agent)
                                && !anchor
                                    .entity
                                    .is_some_and(|from| under(&colliders, &parents, entity, from))
                        };
                        let to = item.position();
                        let Some(direction) = dim::direction(to - anchor.position) else {
                            return true;
                        };
                        let hit = spatial.cast_ray_predicate(
                            dim::point(anchor.position),
                            direction,
                            dim::distance(anchor.position, to),
                            true,
                            &filter,
                            &blocks,
                        );
                        hit.is_none_or(|hit| {
                            item.entity().is_some_and(|target| {
                                under(&colliders, &parents, hit.entity, target)
                            })
                        })
                    };
                    items
                        .iter()
                        .map(|item| {
                            let clear = anchors.iter().any(|anchor| sees(anchor, item));
                            Some(f32::from(u8::from(clear)))
                        })
                        .collect()
                },
            )
            .unwrap_or_else(|| vec![None; items.len()])
    }

    fn cost(&self) -> Cost {
        Cost::High
    }

    fn prepare(&self, world: &mut World) {
        self.rays.prepare(world);
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use bevy_arbor_eqs::anchor::{Agent, At, Thing};
    use bevy_arbor_eqs::query::{Item, QueryContext};
    use bevy_arbor_eqs::test::Test;

    use crate::avian::prelude::*;
    use crate::eqs::LineOfSight;
    use crate::sight::SensorOccluders;
    use crate::testing::*;

    fn sees(
        app: &mut bevy_harness::TestApp,
        agent: Entity,
        test: &LineOfSight,
        to: Vec3,
    ) -> Option<f32> {
        test.prepare(app.world_mut());
        let cx = QueryContext::new(app.world(), agent, SPACE, 1);
        test.test(&cx, &Item::Point(to))
    }

    #[test]
    fn a_wall_in_the_way_blocks_it() {
        let mut app = app();
        let agent = looker(&mut app);
        wall(&mut app, 5.0, 4.0, CollisionLayers::default());
        app.ticks(3);
        let test = LineOfSight::from(Agent);
        assert_eq!(
            sees(&mut app, agent, &test, ahead(10.0)),
            Some(0.0),
            "behind the wall"
        );
        assert_eq!(
            sees(&mut app, agent, &test, ahead(3.0)),
            Some(1.0),
            "before it"
        );
        assert_eq!(
            sees(&mut app, agent, &test, -ahead(10.0)),
            Some(1.0),
            "the other way"
        );
    }

    #[test]
    fn the_agent_does_not_block_its_own() {
        let mut app = app();
        let agent = looker(&mut app);
        app.entity_mut(agent)
            .insert((RigidBody::Kinematic, block(2.0)));
        app.ticks(3);
        let test = LineOfSight::from(At(-ahead(3.0)));
        assert_eq!(
            sees(&mut app, agent, &test, ahead(3.0)),
            Some(1.0),
            "through itself"
        );
    }

    #[test]
    fn a_thing_item_does_not_block_itself() {
        let mut app = app();
        let agent = looker(&mut app);
        let crate_ = wall(&mut app, 5.0, 1.0, CollisionLayers::default());
        app.ticks(3);
        let test = LineOfSight::from(Agent);
        test.prepare(app.world_mut());
        let cx = QueryContext::new(app.world(), agent, SPACE, 1);
        let item = Item::Entity {
            entity: crate_,
            position: ahead(5.0),
        };
        assert_eq!(test.test(&cx, &item), Some(1.0));
    }

    #[test]
    fn an_anchor_does_not_block_its_own_sight() {
        let mut app = app();
        let agent = looker(&mut app);
        let enemy = app.spawn((
            RigidBody::Static,
            block(2.0),
            Transform::from_translation(ahead(5.0)),
        ));
        app.ticks(3);
        let test = LineOfSight::from(Thing(enemy));
        assert_eq!(
            sees(&mut app, agent, &test, ahead(10.0)),
            Some(1.0),
            "from inside its own collider"
        );
    }

    #[test]
    fn an_anchor_with_its_own_occluders_sees_through_what_they_leave_out() {
        let mut app = app();
        let agent = looker(&mut app);
        wall(&mut app, 5.0, 4.0, CollisionLayers::default());
        let eye = app.spawn((SensorOccluders(LayerMask::NONE), Transform::default()));
        app.ticks(3);
        let test = LineOfSight::from(Thing(eye));
        assert_eq!(sees(&mut app, agent, &test, ahead(10.0)), Some(1.0));
    }

    #[test]
    fn one_test_runs_in_two_worlds() {
        let test = LineOfSight::from(Agent);
        for _ in 0..2 {
            let mut app = app();
            let agent = looker(&mut app);
            wall(&mut app, 5.0, 4.0, CollisionLayers::default());
            app.ticks(3);
            assert_eq!(sees(&mut app, agent, &test, ahead(10.0)), Some(0.0));
        }
    }
}
