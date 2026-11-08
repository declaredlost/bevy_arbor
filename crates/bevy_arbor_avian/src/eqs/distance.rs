use bevy::prelude::*;
use bevy_arbor_eqs::query::{Item, QueryContext};
use bevy_arbor_eqs::test::{Cost, Test};

use super::Reads;
use crate::avian::math::ToF32Precision;
use crate::avian::prelude::*;
use crate::dim;
use crate::sight::{Occluders, under};

/// What looking for the nearest occluder reads.
type Nearest = (
    SpatialQuery<'static, 'static>,
    Option<Res<'static, Occluders>>,
    Query<'static, 'static, &'static ColliderOf>,
    Query<'static, 'static, &'static ChildOf>,
    Query<'static, 'static, (), With<Sensor>>,
);

/// How far an item is from the nearest collider that blocks sight: 0 inside
/// one. No answer where there's none at all.
///
/// What blocks sight is what's on [`Occluders`]' layers, unless set
/// [`on`](Self::on) others. The agent's own colliders don't count, nor a
/// thing item's own, nor trigger volumes (avian's `Sensor`).
///
/// Scored lower, it prefers spots hugging cover: up against the wall, not
/// anywhere behind it. Costly, a lookup an item: filter cheaply first.
pub struct OccluderDistance {
    layers: Option<LayerMask>,
    nearest: Reads<Nearest>,
}

impl OccluderDistance {
    /// How far each item is from what blocks sight.
    #[must_use]
    pub fn new() -> Self {
        Self {
            layers: None,
            nearest: Reads::new(),
        }
    }

    /// From colliders on `layers`, not [`Occluders`]'.
    #[must_use]
    pub fn on(mut self, layers: impl Into<LayerMask>) -> Self {
        self.layers = Some(layers.into());
        self
    }
}

impl Default for OccluderDistance {
    fn default() -> Self {
        Self::new()
    }
}

impl Test for OccluderDistance {
    fn test(&self, cx: &QueryContext, item: &Item) -> Option<f32> {
        self.test_all(cx, std::slice::from_ref(item))
            .into_iter()
            .next()
            .flatten()
    }

    fn test_all(&self, cx: &QueryContext, items: &[Item]) -> Vec<Option<f32>> {
        let agent = cx.agent();
        self.nearest
            .with(
                cx.world(),
                |(spatial, occluders, colliders, parents, triggers)| {
                    let layers = self.layers.unwrap_or_else(|| {
                        occluders.map_or(LayerMask::ALL, |occluders| occluders.0)
                    });
                    let filter = SpatialQueryFilter::from_mask(layers);
                    items
                        .iter()
                        .map(|item| {
                            let at = dim::point(item.position());
                            // Neither the agent's own, nor a thing item's own,
                            // nor trigger volumes.
                            let counts = |entity: Entity| {
                                !triggers.contains(entity)
                                    && !under(&colliders, &parents, entity, agent)
                                    && !item
                                        .entity()
                                        .is_some_and(|own| under(&colliders, &parents, entity, own))
                            };
                            // Solid: a point inside a collider is on it, 0
                            // away.
                            let projection =
                                spatial.project_point_predicate(at, true, &filter, &counts)?;
                            Some((projection.point - at).f32().length())
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
        self.nearest.prepare(world);
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use bevy_arbor_eqs::query::{Item, QueryContext};
    use bevy_arbor_eqs::test::Test;

    use crate::avian::prelude::*;
    use crate::eqs::OccluderDistance;
    use crate::testing::*;

    fn far(
        app: &mut bevy_harness::TestApp,
        agent: Entity,
        test: &OccluderDistance,
        at: Vec3,
    ) -> Option<f32> {
        test.prepare(app.world_mut());
        let cx = QueryContext::new(app.world(), agent, SPACE, 1);
        test.test(&cx, &Item::Point(at))
    }

    #[test]
    fn it_is_how_far_the_nearest_wall_is() {
        let mut app = app();
        let agent = looker(&mut app);
        // A wall a unit thick, its near face at 4.5.
        wall(&mut app, 5.0, 4.0, CollisionLayers::default());
        app.ticks(3);
        let test = OccluderDistance::new();
        let near = far(&mut app, agent, &test, ahead(3.5)).unwrap();
        assert!((near - 1.0).abs() < 1e-3, "{near}");
        assert_eq!(far(&mut app, agent, &test, ahead(5.0)), Some(0.0), "inside");
    }

    #[test]
    fn with_nothing_to_hide_behind_there_is_no_answer() {
        let mut app = app();
        let agent = looker(&mut app);
        app.ticks(3);
        assert_eq!(
            far(&mut app, agent, &OccluderDistance::new(), ahead(3.0)),
            None
        );
    }

    #[test]
    fn the_agent_is_not_its_own_cover() {
        let mut app = app();
        let agent = looker(&mut app);
        app.entity_mut(agent)
            .insert((RigidBody::Kinematic, block(2.0)));
        wall(&mut app, 5.0, 4.0, CollisionLayers::default());
        app.ticks(3);
        let near = far(&mut app, agent, &OccluderDistance::new(), Vec3::ZERO).unwrap();
        assert!((near - 4.5).abs() < 1e-3, "to the wall, not itself: {near}");
    }
}
