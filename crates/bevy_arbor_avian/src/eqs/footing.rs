use bevy::prelude::*;
use bevy_arbor_eqs::query::{Item, QueryContext};
use bevy_arbor_eqs::test::{Cost, Test};

use super::Reads;
use crate::avian::prelude::*;
use crate::dim;
use crate::sight::{owns, under};

/// What looking for the ground reads.
type Ground = (
    SpatialQuery<'static, 'static>,
    Query<'static, 'static, &'static ColliderOf>,
    Query<'static, 'static, &'static ChildOf>,
    Query<'static, 'static, (), With<Sensor>>,
);

/// How far down the ground is under an item: a ray straight down from it,
/// to the first collider it hits. 0 inside one; no answer with nothing
/// below, within [`within`](Self::within).
///
/// Down is -Y: in 2D, a side view's down, not a top-down game's. Every
/// layer is ground, unless set [`on`](Self::on) some. The agent's own
/// colliders aren't, nor a thing item's own, nor trigger volumes (avian's
/// `Sensor`). Costly, a ray an item: filter cheaply first.
///
/// Generators lay points out level with their anchor, and in a side view
/// some are over a drop. `Footing::new()`, filtered to at most how far an
/// agent's middle is above its feet, keeps the points it could stand at.
pub struct Footing {
    layers: LayerMask,
    reach: f32,
    ground: Reads<Ground>,
}

impl Footing {
    /// How far down the ground is under each item, however far.
    #[must_use]
    pub fn new() -> Self {
        Self {
            layers: LayerMask::ALL,
            reach: f32::MAX,
            ground: Reads::new("Footing"),
        }
    }

    /// Only colliders on `layers` are ground: the level's floors, not what's
    /// standing on them.
    #[must_use]
    pub fn on(mut self, layers: impl Into<LayerMask>) -> Self {
        self.layers = layers.into();
        self
    }

    /// Looks no further down than `reach`: anything deeper is no ground.
    #[must_use]
    pub fn within(mut self, reach: f32) -> Self {
        self.reach = reach.max(0.0);
        self
    }
}

impl Default for Footing {
    fn default() -> Self {
        Self::new()
    }
}

impl Test for Footing {
    fn test(&self, cx: &QueryContext, item: &Item) -> Option<f32> {
        self.test_all(cx, std::slice::from_ref(item))
            .into_iter()
            .next()
            .flatten()
    }

    fn test_all(&self, cx: &QueryContext, items: &[Item]) -> Vec<Option<f32>> {
        let agent = cx.agent();
        let filter = SpatialQueryFilter::from_mask(self.layers);
        let Some(down) = dim::direction(Vec3::NEG_Y) else {
            return vec![None; items.len()];
        };
        self.ground
            .with(cx.world(), |(spatial, colliders, parents, triggers)| {
                items
                    .iter()
                    .map(|item| {
                        // Neither the agent's own, nor a thing item's own,
                        // nor trigger volumes.
                        let ground = |entity: Entity| {
                            !triggers.contains(entity)
                                && !under(&colliders, &parents, entity, agent)
                                && !item
                                    .entity()
                                    .is_some_and(|own| owns(&colliders, &parents, entity, own))
                        };
                        // Solid: from inside a collider, it's 0 down.
                        spatial
                            .cast_ray_predicate(
                                dim::point(item.position()),
                                down,
                                self.reach,
                                true,
                                &filter,
                                &ground,
                            )
                            .map(|hit| hit.distance)
                    })
                    .collect()
            })
            .unwrap_or_else(|| vec![None; items.len()])
    }

    fn cost(&self) -> Cost {
        Cost::High
    }

    fn prepare(&self, world: &mut World) {
        self.ground.prepare(world);
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use bevy_arbor_eqs::query::{Item, QueryContext};
    use bevy_arbor_eqs::test::Test;

    use crate::avian::prelude::*;
    use crate::eqs::Footing;
    use crate::testing::*;

    fn down(
        app: &mut bevy_harness::TestApp,
        agent: Entity,
        test: &Footing,
        item: Item,
    ) -> Option<f32> {
        test.prepare(app.world_mut());
        let cx = QueryContext::new(app.world(), agent, SPACE, 1);
        test.test(&cx, &item)
    }

    /// A floor a unit thick, its top at -1, under `ahead(-2..2)` only.
    fn floor(app: &mut bevy_harness::TestApp, layers: CollisionLayers) -> Entity {
        #[cfg(dim = "2d")]
        let shape = Collider::rectangle(4.0, 1.0);
        #[cfg(dim = "3d")]
        let shape = Collider::cuboid(1.0, 1.0, 4.0);
        app.spawn((
            RigidBody::Static,
            shape,
            layers,
            Transform::from_xyz(0.0, -1.5, 0.0),
        ))
    }

    fn close(found: Option<f32>, to: f32) -> bool {
        found.is_some_and(|found| (found - to).abs() < 1e-3)
    }

    #[test]
    fn it_is_how_far_down_the_ground_is_and_over_a_drop_there_is_none() {
        let mut app = app();
        let agent = looker(&mut app);
        floor(&mut app, CollisionLayers::default());
        app.ticks(3);
        let test = Footing::new();
        let over = down(&mut app, agent, &test, Item::Point(ahead(1.0)));
        assert!(close(over, 1.0), "over the floor: {over:?}");
        let inside = down(
            &mut app,
            agent,
            &test,
            Item::Point(ahead(1.0) - Vec3::Y * 1.5),
        );
        assert_eq!(inside, Some(0.0), "inside it");
        assert_eq!(
            down(&mut app, agent, &test, Item::Point(ahead(5.0))),
            None,
            "over the drop"
        );
        assert_eq!(
            down(
                &mut app,
                agent,
                &Footing::new().within(0.5),
                Item::Point(ahead(1.0))
            ),
            None,
            "deeper than it reaches"
        );
    }

    #[test]
    fn only_colliders_on_its_layers_are_ground() {
        let mut app = app();
        let agent = looker(&mut app);
        let ground = LayerMask(0b10);
        floor(&mut app, CollisionLayers::default());
        app.ticks(3);
        let test = Footing::new().on(ground);
        assert_eq!(
            down(&mut app, agent, &test, Item::Point(ahead(1.0))),
            None,
            "not on the ground's layer"
        );
    }

    #[test]
    fn neither_the_agent_nor_a_thing_item_stands_on_itself() {
        let mut app = app();
        let agent = looker(&mut app);
        app.entity_mut(agent)
            .insert((RigidBody::Kinematic, block(2.0)));
        floor(&mut app, CollisionLayers::default());
        // A crate, its middle 1 ahead, standing on the floor.
        let crate_ = app.spawn((
            RigidBody::Static,
            block(1.0),
            Transform::from_translation(ahead(1.0) - Vec3::Y * 0.5),
        ));
        app.ticks(3);
        let test = Footing::new();
        let under_agent = down(&mut app, agent, &test, Item::Point(Vec3::ZERO));
        assert!(close(under_agent, 1.0), "past itself: {under_agent:?}");
        let item = Item::Entity {
            entity: crate_,
            position: ahead(1.0) - Vec3::Y * 0.5,
        };
        let under_crate = down(&mut app, agent, &test, item);
        assert!(close(under_crate, 0.5), "past its own: {under_crate:?}");
    }
}
