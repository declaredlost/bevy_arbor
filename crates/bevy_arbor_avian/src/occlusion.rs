use std::marker::PhantomData;

use bevy::prelude::*;
use bevy_arbor_perception::PerceptionSystems;
use bevy_arbor_perception::perceived::Candidates;
use bevy_arbor_perception::sense::{Sense, SensorOf, Sight};
use bevy_arbor_perception::space::Space;

use crate::avian::prelude::*;
use crate::dim;

/// Blocks what's out of sight: a ray from where each sensor looked to each of
/// its candidates for `S`, in [`PerceptionSystems::Occlude`]. What the ray hits
/// first on the way, if it isn't the candidate itself, blocks it.
///
/// What blocks it is what's on [`Occluders`]' layers, or a sensor's own
/// [`SensorOccluders`]: avian's `CollisionLayers`. The agent's and the
/// sensor's own colliders never do, nor trigger volumes (avian's `Sensor`); a
/// collider on the target, or on a child of it, or the body it's on, is the
/// target, not in the way. A candidate with [`SightPoints`] is seen
/// if a ray to any of them gets through.
///
/// For any sense whose sensors look ([`Candidates::look_from`]); a sense
/// that doesn't, a noise, say, it leaves be. Needs avian's `PhysicsPlugins`;
/// without them, it warns and blocks nothing.
pub struct LineOfSightPlugin<S: Sense = Sight>(PhantomData<fn() -> S>);

impl<S: Sense> Default for LineOfSightPlugin<S> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<S: Sense> Plugin for LineOfSightPlugin<S> {
    fn build(&self, app: &mut App) {
        app.init_resource::<Occluders>()
            .register_type::<Occluders>()
            .register_type::<SensorOccluders>()
            .register_type::<SightPoints>();
    }

    fn finish(&self, app: &mut App) {
        let Some(schedule) = schedule(app) else {
            warn!("`LineOfSightPlugin` needs `PerceptionPlugin`");
            return;
        };
        // Without avian's spatial queries, there's nothing to cast rays with:
        // say so, rather than fail each tick.
        if !app.is_plugin_added::<SpatialQueryPlugin>() {
            warn!("`LineOfSightPlugin` needs avian's `PhysicsPlugins`: nothing blocks sight");
            return;
        }
        app.add_systems(schedule, occlude::<S>.in_set(PerceptionSystems::Occlude));
    }
}

/// The schedule perception runs in, if it's there.
pub(crate) fn schedule(app: &App) -> Option<bevy::ecs::schedule::InternedScheduleLabel> {
    app.get_added_plugins::<bevy_arbor_perception::PerceptionPlugin>()
        .first()
        .map(|plugin| plugin.schedule())
}

/// What blocks sight: the layers whose colliders a ray from a sensor stops at.
/// Every layer, unless set; a sensor's [`SensorOccluders`] says otherwise for
/// it.
#[derive(Resource, Reflect, Debug, Clone, Copy, PartialEq, Eq)]
#[reflect(Resource, Debug, Default, PartialEq)]
pub struct Occluders(pub LayerMask);

impl Default for Occluders {
    fn default() -> Self {
        Self(LayerMask::ALL)
    }
}

/// On a sensor (or an agent sensing itself): what blocks its sight, over
/// [`Occluders`]. Eyes that see through glass leave its layer out.
#[derive(Component, Reflect, Debug, Clone, Copy, PartialEq, Eq)]
#[reflect(Component, Debug, PartialEq)]
pub struct SensorOccluders(pub LayerMask);

/// On something that can be seen: the points on it a sensor can see it by,
/// in its own space (a head, a chest, feet). Seen if any is in sight; without
/// them, seen by its middle.
#[derive(Component, Reflect, Debug, Clone, Default, PartialEq)]
#[reflect(Component, Debug, Default, PartialEq)]
pub struct SightPoints(pub Vec<Vec3>);

/// How far up a hierarchy a collider is looked for: deeper than any rig.
const DEPTH: usize = 32;

/// Each sensor that looked blocks the candidates something's in the way of.
fn occlude<S: Sense>(
    spatial: SpatialQuery,
    occluders: Res<Occluders>,
    space: Res<Space>,
    colliders: Query<&ColliderOf>,
    parents: Query<&ChildOf>,
    triggers: Query<(), With<Sensor>>,
    places: Query<(&GlobalTransform, &SightPoints)>,
    mut sensors: Query<(
        Entity,
        Option<&SensorOf>,
        Option<&SensorOccluders>,
        &mut Candidates<S>,
    )>,
) {
    let space = *space;
    // Whether `entity`, a collider, or the body it's on, is `root` or below it.
    let under = |entity: Entity, root: Entity| {
        let body = colliders.get(entity).map_or(entity, |of| of.body);
        [entity, body].into_iter().any(|start| {
            std::iter::successors(Some(start), |&at| parents.get(at).ok().map(ChildOf::parent))
                .take(DEPTH)
                .any(|at| at == root)
        })
    };
    for (sensor, of, own, mut candidates) in &mut sensors {
        let Some(eye) = candidates.eye() else {
            continue;
        };
        let agent = of.map_or(sensor, SensorOf::agent);
        let filter = SpatialQueryFilter::from_mask(own.map_or(occluders.0, |own| own.0));
        // Trigger volumes don't block sight, nor the agent's or sensor's own
        // colliders.
        let blocks = |entity: Entity| {
            !triggers.contains(entity) && !under(entity, agent) && !under(entity, sensor)
        };
        for candidate in candidates.iter_mut() {
            let target = candidate.entity();
            // The target's own: a collider on it or below it, or the body it's
            // on.
            let reaches = |hit: Entity| {
                under(hit, target)
                    || std::iter::successors(Some(target), |&at| {
                        parents.get(at).ok().map(ChildOf::parent)
                    })
                    .take(DEPTH)
                    .any(|at| colliders.get(hit).map_or(hit, |of| of.body) == at)
            };
            let in_sight = |point: Vec3| {
                let Some(direction) = dim::direction(point - eye) else {
                    return true;
                };
                let hit = spatial.cast_ray_predicate(
                    dim::point(eye),
                    direction,
                    dim::distance(eye, point),
                    true,
                    &filter,
                    &blocks,
                );
                hit.is_none_or(|hit| reaches(hit.entity))
            };
            let seen = match places.get(target) {
                Ok((transform, points)) if !points.0.is_empty() => points
                    .0
                    .iter()
                    .any(|&offset| in_sight(space.flatten(transform.transform_point(offset)))),
                _ => in_sight(candidate.position()),
            };
            if !seen {
                candidate.block();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use super::{SensorOccluders, SightPoints};
    use crate::avian::prelude::*;
    use crate::testing::*;

    /// Lets physics put what was spawned in its spatial index, and the agents
    /// look again.
    fn settle(app: &mut bevy_harness::TestApp) {
        app.ticks(3);
    }

    #[test]
    fn a_wall_in_the_way_blocks_sight() {
        let mut app = app();
        let agent = looker(&mut app);
        let target = thing(&mut app, 10.0);
        let wall = wall(&mut app, 5.0, 4.0, CollisionLayers::default());
        settle(&mut app);
        assert!(!sees(&app, agent, target), "behind the wall");
        app.entity_mut(wall).despawn();
        settle(&mut app);
        assert!(sees(&app, agent, target), "the wall's gone");
    }

    #[test]
    fn neither_the_target_nor_the_agent_blocks_its_own_sight() {
        let mut app = app();
        let agent = looker(&mut app);
        app.entity_mut(agent)
            .insert((RigidBody::Static, block(2.0)));
        let target = thing(&mut app, 10.0);
        app.entity_mut(target)
            .insert((RigidBody::Static, block(2.0)));
        settle(&mut app);
        assert!(sees(&app, agent, target));
    }

    #[test]
    fn a_sensor_sees_through_what_its_own_occluders_leave_out() {
        let mut app = app();
        let glass = CollisionLayers::new(LayerMask(0b10), LayerMask::ALL);
        wall(&mut app, 5.0, 4.0, glass);
        let blind = looker(&mut app);
        let sharp = looker(&mut app);
        app.entity_mut(sharp)
            .insert(SensorOccluders(LayerMask::ALL & !LayerMask(0b10)));
        let target = thing(&mut app, 10.0);
        settle(&mut app);
        assert!(!sees(&app, blind, target), "glass blocks it, by default");
        assert!(sees(&app, sharp, target), "it sees through glass");
    }

    #[test]
    fn a_head_over_a_low_wall_is_seen() {
        let mut app = app();
        wall(&mut app, 5.0, 1.0, CollisionLayers::default());
        let agent = looker(&mut app);
        let crouching = thing(&mut app, 10.0);
        let standing = thing(&mut app, 10.0);
        app.entity_mut(standing)
            .insert(SightPoints(vec![Vec3::ZERO, Vec3::new(0.0, 3.0, 0.0)]));
        settle(&mut app);
        assert!(
            !sees(&app, agent, crouching),
            "its middle is behind the wall"
        );
        assert!(sees(&app, agent, standing), "its head shows over it");
    }

    #[test]
    fn an_agent_inside_a_trigger_volume_still_sees() {
        let mut app = app();
        let agent = looker(&mut app);
        // An aggro zone around the agent, on the default layer.
        app.spawn((RigidBody::Static, Sensor, block(30.0), Transform::default()));
        let target = thing(&mut app, 10.0);
        settle(&mut app);
        assert!(sees(&app, agent, target), "a trigger isn't in the way");
    }

    #[test]
    fn colliders_on_children_are_their_owners() {
        let mut app = app();
        let agent = looker(&mut app);
        // The agent's own collider, on a child, no rigid body.
        app.spawn((block(2.0), ChildOf(agent), Transform::default()));
        let target = thing(&mut app, 10.0);
        // The target's, the same way.
        app.spawn((block(2.0), ChildOf(target), Transform::default()));
        settle(&mut app);
        assert!(sees(&app, agent, target), "neither hides it");
    }
}
