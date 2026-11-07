use std::f32::consts::PI;

use bevy::prelude::*;

use super::shape::cone;
use super::{PerceptionGizmos, ShowPerception};
use crate::sense::{Hearing, SensorOf};
use crate::space::Space;

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
    let Some(colour) = config.hearing else {
        return;
    };
    let axis = space.ring_axis();
    for (hearing, transform, of, own) in &sensors {
        // Shown itself, or its agent is.
        if config.shows(own || of.is_some_and(|of| shown.contains(of.agent()))) {
            let ear = space.flatten(transform.translation());
            gizmos.linestrip(
                cone(ear, axis.any_orthonormal_vector(), axis, PI, hearing.range),
                colour,
            );
        }
    }
}
