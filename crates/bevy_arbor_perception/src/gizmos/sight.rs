use bevy::prelude::*;

use super::shape::cone;
use super::{PerceptionGizmos, ShowPerception};
use crate::sense::{Facing, SensorOf, Sight, ahead};
use crate::space::Space;

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
        let ahead = ahead(space, transform, facing);
        for axis in space.turns(transform) {
            if let Some(colour) = config.sight {
                gizmos.linestrip(
                    cone(eye, ahead, axis, sight.half_angle, sight.range),
                    colour,
                );
            }
            if let Some(colour) = config.lose_sight
                && sight.lose_range > sight.range
            {
                gizmos.linestrip(
                    cone(eye, ahead, axis, sight.half_angle, sight.lose_range),
                    colour,
                );
            }
        }
    }
}
