use bevy::prelude::*;
use bevy_arbor_eqs::query::{Item, QueryContext};
use bevy_arbor_eqs::test::{Cost, Test};

use super::Reads;
use crate::avian::prelude::*;
use crate::dim;
use crate::sight::{owns, under};

/// What looking for room reads.
type Room = (
    SpatialQuery<'static, 'static>,
    Query<'static, 'static, &'static ColliderOf>,
    Query<'static, 'static, &'static ChildOf>,
    Query<'static, 'static, (), With<Sensor>>,
);

/// Whether there's room at an item: 1 if `shape`, centered on it, overlaps
/// no collider, and 0 if it does. A point inside a wall, or one a crate
/// stands on, has none.
///
/// Every layer counts, unless set [`on`](Self::on) some. The agent's own
/// colliders don't, nor a thing item's own, nor trigger volumes (avian's
/// `Sensor`). Costly, a shape test an item: filter cheaply first.
///
/// `Clearance::of(Collider::circle(0.4))`, filtered to 1, keeps the spots
/// an agent that size fits in.
pub struct Clearance {
    shape: Collider,
    layers: LayerMask,
    room: Reads<Room>,
}

impl Clearance {
    /// Whether `shape`, centered on each item, has room: the agent's size,
    /// or a little more.
    #[must_use]
    pub fn of(shape: Collider) -> Self {
        Self {
            shape,
            layers: LayerMask::ALL,
            room: Reads::new("Clearance"),
        }
    }

    /// Only colliders on `layers` take room: the level's walls, not what's
    /// moving through it.
    #[must_use]
    pub fn on(mut self, layers: impl Into<LayerMask>) -> Self {
        self.layers = layers.into();
        self
    }
}

impl Test for Clearance {
    fn test(&self, cx: &QueryContext, item: &Item) -> Option<f32> {
        self.test_all(cx, std::slice::from_ref(item))
            .into_iter()
            .next()
            .flatten()
    }

    fn test_all(&self, cx: &QueryContext, items: &[Item]) -> Vec<Option<f32>> {
        let agent = cx.agent();
        let filter = SpatialQueryFilter::from_mask(self.layers);
        self.room
            .with(cx.world(), |(spatial, colliders, parents, triggers)| {
                items
                    .iter()
                    .map(|item| {
                        let mut room = true;
                        spatial.shape_intersections_callback(
                            &self.shape,
                            dim::point(item.position()),
                            dim::upright(),
                            &filter,
                            |entity| {
                                // Trigger volumes, the agent's own colliders
                                // and a thing item's own take no room.
                                let solid = !triggers.contains(entity)
                                    && !under(&colliders, &parents, entity, agent)
                                    && !item
                                        .entity()
                                        .is_some_and(|own| owns(&colliders, &parents, entity, own));
                                room &= !solid;
                                room
                            },
                        );
                        Some(f32::from(u8::from(room)))
                    })
                    .collect()
            })
            .unwrap_or_else(|| vec![None; items.len()])
    }

    fn cost(&self) -> Cost {
        Cost::High
    }

    fn prepare(&self, world: &mut World) {
        self.room.prepare(world);
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use bevy_arbor_eqs::query::{Item, QueryContext};
    use bevy_arbor_eqs::test::Test;

    use crate::avian::prelude::*;
    use crate::eqs::Clearance;
    use crate::testing::*;

    fn room(
        app: &mut bevy_harness::TestApp,
        agent: Entity,
        test: &Clearance,
        item: Item,
    ) -> Option<f32> {
        test.prepare(app.world_mut());
        let cx = QueryContext::new(app.world(), agent, SPACE, 1);
        test.test(&cx, &item)
    }

    fn body() -> Collider {
        #[cfg(dim = "2d")]
        return Collider::circle(0.4);
        #[cfg(dim = "3d")]
        return Collider::sphere(0.4);
    }

    #[test]
    fn inside_a_wall_there_is_no_room_and_beside_it_there_is() {
        let mut app = app();
        let agent = looker(&mut app);
        wall(&mut app, 5.0, 4.0, CollisionLayers::default());
        app.ticks(3);
        let test = Clearance::of(body());
        assert_eq!(
            room(&mut app, agent, &test, Item::Point(ahead(5.0))),
            Some(0.0),
            "in it"
        );
        assert_eq!(
            room(&mut app, agent, &test, Item::Point(ahead(3.0))),
            Some(1.0),
            "before it"
        );
    }

    #[test]
    fn layers_left_out_take_no_room() {
        let mut app = app();
        let agent = looker(&mut app);
        wall(
            &mut app,
            5.0,
            4.0,
            CollisionLayers::new(LayerMask(1 << 3), LayerMask::ALL),
        );
        app.ticks(3);
        let test = Clearance::of(body()).on(LayerMask(1 << 2));
        assert_eq!(
            room(&mut app, agent, &test, Item::Point(ahead(5.0))),
            Some(1.0)
        );
    }

    #[test]
    fn the_agent_and_a_thing_item_take_none_of_their_own_room() {
        let mut app = app();
        let agent = looker(&mut app);
        app.entity_mut(agent)
            .insert((RigidBody::Kinematic, block(2.0)));
        let crate_ = wall(&mut app, 5.0, 1.0, CollisionLayers::default());
        app.ticks(3);
        let test = Clearance::of(body());
        assert_eq!(
            room(&mut app, agent, &test, Item::Point(Vec3::ZERO)),
            Some(1.0),
            "itself"
        );
        let item = Item::Entity {
            entity: crate_,
            position: ahead(5.0),
        };
        assert_eq!(room(&mut app, agent, &test, item), Some(1.0), "the crate");
    }
}
