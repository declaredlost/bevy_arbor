use bevy::prelude::*;
use bevy_arbor_perception::Space;
use bevy_arbor_perception::perceived::{Attitude, Perceived};
use bevy_arbor_perception::sense::Sense;

use super::{PerceptionGizmos, ShowPerception};

/// For the sense `S`: a line from each agent to what's active for it (in its
/// view), colored by attitude, and a cross where what it only remembers was
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
                if let (Some(color), Some(trace)) = (config.remembered, percept.trace()) {
                    gizmos.cross(
                        Isometry3d::from_translation(trace.position),
                        config.cross_size,
                        color,
                    );
                }
                continue;
            }
            let color = match percept.attitude() {
                Attitude::Hostile => config.hostile,
                Attitude::Friendly => config.friendly,
                Attitude::Neutral => config.neutral,
            };
            if let (Some(color), Ok(target)) = (color, places.get(percept.entity())) {
                gizmos.line(at, space.flatten(target.translation()), color);
            }
        }
    }
}
