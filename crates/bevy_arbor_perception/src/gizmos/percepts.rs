use bevy::prelude::*;

use super::{PerceptionGizmos, ShowPerception};
use crate::perceived::{Attitude, Perceived};
use crate::sense::Sense;
use crate::space::Space;

/// For the sense `S`: a line from each agent to what's active for it (in its
/// view), coloured by attitude, and a cross where what it only remembers was
/// last sensed.
pub(super) fn draw<S: Sense>(
    mut gizmos: Gizmos<PerceptionGizmos>,
    space: Res<Space>,
    places: Query<&GlobalTransform>,
    agents: Query<(&Perceived<S>, &GlobalTransform, Has<ShowPerception>)>,
) {
    let (space, config) = (*space, gizmos.config_ext.clone());
    for (perceived, transform, shown) in &agents {
        if !config.shows(shown) {
            continue;
        }
        let at = space.flatten(transform.translation());
        for percept in perceived.iter() {
            if !percept.is_active() {
                if let (Some(colour), Some(trace)) = (config.remembered, percept.trace()) {
                    gizmos.cross(
                        Isometry3d::from_translation(trace.position),
                        config.cross_size,
                        colour,
                    );
                }
                continue;
            }
            let colour = match percept.attitude() {
                Attitude::Hostile => config.hostile,
                Attitude::Friendly => config.friendly,
                Attitude::Neutral => config.neutral,
            };
            if let (Some(colour), Ok(target)) = (colour, places.get(percept.entity())) {
                gizmos.line(at, space.flatten(target.translation()), colour);
            }
        }
    }
}
