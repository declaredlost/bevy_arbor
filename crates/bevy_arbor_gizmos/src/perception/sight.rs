use bevy::prelude::*;
use bevy_arbor_perception::Space;
use bevy_arbor_perception::sense::{Facing, SensorOf, Sight};

use super::shape::{cone, turns};
use super::{PerceptionGizmos, ShowPerception};

/// Each sensor's sight cone, out to `range` and, fainter, to `lose_range`.
pub(super) fn draw(
    mut gizmos: Gizmos<PerceptionGizmos>,
    space: Res<Space>,
    sensors: Query<(
        &Sight,
        &GlobalTransform,
        Option<&Facing>,
        Option<&SensorOf>,
        Has<ShowPerception>,
    )>,
    shown: Query<(), With<ShowPerception>>,
    facings: Query<&Facing>,
) {
    let (space, config) = (*space, gizmos.config_ext.clone());
    if config.sight.is_none() && config.lose_sight.is_none() {
        return;
    }
    for (sight, transform, facing, of, own) in &sensors {
        // Shown itself, or its agent is.
        if !config.shows(own || of.is_some_and(|of| shown.contains(of.agent()))) {
            continue;
        }
        let eye = space.flatten(transform.transform_point(sight.eye));
        // Its own facing, else its agent's, as it looks.
        let facing = facing.or_else(|| of.and_then(|of| facings.get(of.agent()).ok()));
        let ahead = space.looking(transform, facing);
        for axis in turns(space, transform) {
            if let Some(color) = config.sight {
                gizmos.linestrip(cone(eye, ahead, axis, sight.half_angle, sight.range), color);
            }
            if let Some(color) = config.lose_sight
                && sight.lose_range > sight.range
            {
                gizmos.linestrip(
                    cone(eye, ahead, axis, sight.half_angle, sight.lose_range),
                    color,
                );
            }
        }
    }
}
