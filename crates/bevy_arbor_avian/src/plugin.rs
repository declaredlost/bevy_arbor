use bevy::app::plugin_group;
use bevy::prelude::*;
use bevy_arbor_perception::Space;

use crate::dim;

plugin_group! {
    /// Sight finding what's near through avian's colliders, line of sight,
    /// and touch from contacts.
    #[derive(Default)]
    pub struct AvianPerceptionPlugins {
        crate::sight:::ColliderSightPlugin,
        crate::sight:::LineOfSightPlugin,
        crate::touch:::ContactTouchPlugin,
    }
}

/// Set once perception's space has been checked against this crate's
/// dimension, so it's warned about once, however many plugins check.
#[derive(Resource)]
struct SpaceChecked;

/// Warns, once, if perception works in another space than this crate's
/// dimension: 3D perception over 2D physics finds nothing where it should.
pub(crate) fn check_space(app: &mut App) {
    if app.world().contains_resource::<SpaceChecked>() {
        return;
    }
    app.insert_resource(SpaceChecked);
    if let Some(space) = mismatched(app) {
        warn!(
            "`{}` works in `Space::{:?}`, but perception is in `Space::{space:?}`: \
             set it with `PerceptionPlugin::default().with_space(Space::{:?})`",
            env!("CARGO_PKG_NAME"),
            dim::SPACE,
            dim::SPACE,
        );
    }
}

/// Perception's space, if it isn't this crate's dimension's.
fn mismatched(app: &App) -> Option<Space> {
    app.world()
        .get_resource::<Space>()
        .copied()
        .filter(|&space| space != dim::SPACE)
}

/// The schedule perception runs in, if it's there.
pub(crate) fn schedule(app: &App) -> Option<bevy::ecs::schedule::InternedScheduleLabel> {
    app.get_added_plugins::<bevy_arbor_perception::PerceptionPlugin>()
        .first()
        .map(|plugin| plugin.schedule())
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use bevy_arbor_perception::prelude::*;

    use super::mismatched;
    use crate::testing::SPACE;

    fn perception_in(space: Space) -> App {
        let mut app = App::new();
        app.add_plugins(PerceptionPlugin::default().with_space(space));
        app
    }

    #[test]
    fn perception_in_the_other_space_is_found() {
        let other = match SPACE {
            Space::Planar => Space::Volume,
            Space::Volume => Space::Planar,
        };
        assert_eq!(mismatched(&perception_in(other)), Some(other));
        assert_eq!(mismatched(&perception_in(SPACE)), None);
    }
}
