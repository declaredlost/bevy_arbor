use std::f32::consts::PI;

use bevy::prelude::*;
use bevy_arbor_perception::Space;
use bevy_arbor_perception::sense::{Hearing, SensorOf};

use super::shape::{cone, ring_axis};
use super::{PerceptionGizmos, ShowPerception};

/// Each sensor's hearing range, as a ring around it.
pub(super) fn draw(
    mut gizmos: Gizmos<PerceptionGizmos>,
    space: Res<Space>,
    sensors: Query<(
        &Hearing,
        &GlobalTransform,
        Option<&SensorOf>,
        Has<ShowPerception>,
    )>,
    shown: Query<(), With<ShowPerception>>,
) {
    let (space, config) = (*space, gizmos.config_ext.clone());
    let Some(color) = config.hearing else {
        return;
    };
    let axis = ring_axis(space);
    for (hearing, transform, of, own) in &sensors {
        // Shown itself, or its agent is.
        if config.shows(own || of.is_some_and(|of| shown.contains(of.agent()))) {
            let ear = space.flatten(transform.translation());
            gizmos.linestrip(
                cone(ear, axis.any_orthonormal_vector(), axis, PI, hearing.range),
                color,
            );
        }
    }
}
