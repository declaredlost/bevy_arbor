use bevy::prelude::*;

/// Draws every part enabled: what agents perceive, and, with their features,
/// how queries weighed their items, smart objects' slots, and where agents
/// are going.
///
/// Not in `ArborPlugins`, as it needs a renderer: Bevy's `GizmoPlugin`, in
/// `DefaultPlugins`. Each part's drawing is a gizmo group of its own,
/// [`PerceptionGizmos`](crate::perception::PerceptionGizmos) and the rest:
/// set what each draws, or turn it off, through Bevy's `GizmoConfigStore`.
pub struct ArborGizmosPlugin;

impl Plugin for ArborGizmosPlugin {
    fn build(&self, app: &mut App) {
        crate::perception::plugin(app);
        #[cfg(feature = "eqs")]
        crate::eqs::plugin(app);
        #[cfg(feature = "smart")]
        crate::smart::plugin(app);
        #[cfg(feature = "nav")]
        crate::nav::plugin(app);
    }
}
