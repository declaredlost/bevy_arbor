use bevy::app::plugin_group;
use bevy::prelude::*;

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

/// The schedule perception runs in, if it's there.
pub(crate) fn schedule(app: &App) -> Option<bevy::ecs::schedule::InternedScheduleLabel> {
    app.get_added_plugins::<bevy_arbor_perception::PerceptionPlugin>()
        .first()
        .map(|plugin| plugin.schedule())
}
