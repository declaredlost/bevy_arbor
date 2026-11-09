use bevy::ecs::entity::EntityHashSet;
use bevy::ecs::system::SystemParamItem;
use bevy::prelude::*;
use bevy_arbor_perception::Space;
use bevy_arbor_perception::sense::{Perceivable, SightIndex, SightIndexApp};

use crate::avian::prelude::*;
use crate::dim;
use crate::plugin::check_space;

/// The layers what can be seen is on: what sight asks avian for, as it
/// finds what's near a sensor. Every layer, unless set.
///
/// Put what's [`Perceivable`] on a layer of its own and set it here: a look
/// then skips the walls and tiles round it.
#[derive(Resource, Reflect, Debug, Clone, Copy, PartialEq, Eq)]
#[reflect(Resource, Debug, Default, PartialEq)]
pub struct PerceivableLayers(pub LayerMask);

impl Default for PerceivableLayers {
    fn default() -> Self {
        Self(LayerMask::ALL)
    }
}

/// Sight finds what's near a sensor through avian's colliders, instead of
/// perception's own grid: nothing to rebuild, avian keeps them up to date.
///
/// Only what has a collider can be found: a [`Perceivable`] without one, on
/// it or on a child of it, isn't seen. Colliders on
/// [`PerceivableLayers`]' layers are asked, trigger volumes (avian's
/// `Sensor`) too. In [`AvianPerceptionPlugins`](crate::AvianPerceptionPlugins);
/// without avian's `PhysicsPlugins`, it warns and sight keeps perception's
/// grid.
#[derive(Default)]
pub struct ColliderSightPlugin;

impl Plugin for ColliderSightPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PerceivableLayers>()
            .register_type::<PerceivableLayers>();
    }

    fn finish(&self, app: &mut App) {
        check_space(app);
        // Without avian's spatial queries, perception's own grid stays.
        if !app.is_plugin_added::<SpatialQueryPlugin>() {
            warn!("`ColliderSightPlugin` needs avian's `PhysicsPlugins`: sight uses its own grid");
            return;
        }
        app.set_sight_index::<Colliders>();
    }
}

/// What finding what's near through avian reads.
type Near = (
    SpatialQuery<'static, 'static>,
    Option<Res<'static, PerceivableLayers>>,
    Res<'static, Space>,
    Query<'static, 'static, &'static ColliderOf>,
    Query<'static, 'static, &'static GlobalTransform, With<Perceivable>>,
);

/// Avian's colliders, as sight's index.
struct Colliders;

impl SightIndex for Colliders {
    type Param = Near;

    fn near(
        (spatial, layers, space, colliders, perceivable): &SystemParamItem<Self::Param>,
        point: Vec3,
        radius: f32,
        found: &mut dyn FnMut(Entity, Vec3),
    ) {
        let mask = layers.as_ref().map_or(LayerMask::ALL, |layers| layers.0);
        let filter = SpatialQueryFilter::from_mask(mask);
        let mut seen = EntityHashSet::default();
        spatial.shape_intersections_callback(
            &dim::ball(radius),
            dim::point(point),
            dim::upright(),
            &filter,
            |collider| {
                // What's perceivable: the collider itself, or the body it's on.
                let body = colliders.get(collider).map_or(collider, |of| of.body);
                let thing = [collider, body]
                    .into_iter()
                    .find(|&entity| perceivable.contains(entity));
                // Once each, however many colliders it has.
                if let Some(thing) = thing
                    && let Ok(transform) = perceivable.get(thing)
                    && seen.insert(thing)
                {
                    found(thing, space.flatten(transform.translation()));
                }
                true
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use bevy_arbor_perception::sense::Perceivable;

    use crate::avian::prelude::*;
    use crate::sight::PerceivableLayers;
    use crate::testing::*;

    #[test]
    fn what_has_a_collider_is_found_and_what_has_none_is_not() {
        let mut app = app();
        let agent = looker(&mut app);
        let solid = thing(&mut app, 5.0);
        let bare = app.spawn((Perceivable, Transform::from_translation(ahead(6.0))));
        app.ticks(3);
        assert!(sees(&app, agent, solid), "through avian");
        assert!(!sees(&app, agent, bare), "nothing to find it by");
    }

    #[test]
    fn a_collider_on_a_child_finds_its_perceivable_owner() {
        let mut app = app();
        let agent = looker(&mut app);
        let owner = app.spawn((
            Perceivable,
            RigidBody::Static,
            Transform::from_translation(ahead(5.0)),
        ));
        app.spawn((crate::dim::ball(0.25), ChildOf(owner), Transform::default()));
        app.ticks(3);
        assert!(sees(&app, agent, owner));
    }

    #[test]
    fn only_colliders_on_the_perceivable_layers_are_asked() {
        let mut app = app();
        app.world_mut()
            .insert_resource(PerceivableLayers(LayerMask(1 << 4)));
        let agent = looker(&mut app);
        // Either side, so neither hides the other.
        let elsewhere = thing(&mut app, -5.0);
        let on = thing(&mut app, 5.0);
        app.entity_mut(on)
            .insert(CollisionLayers::new(LayerMask(1 << 4), LayerMask::ALL));
        app.ticks(3);
        assert!(sees(&app, agent, on), "on the layer");
        assert!(!sees(&app, agent, elsewhere), "off it");
    }
}
