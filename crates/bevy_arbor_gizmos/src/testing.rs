//! What this crate's tests share.

use bevy::prelude::*;
use bevy_arbor_perception::prelude::*;
use bevy_harness::TestApp;

/// Both spaces: each perception test runs in 2D and in 3D.
pub(crate) const SPACES: [Space; 2] = [Space::Planar, Space::Volume];

/// `distance` ahead of an agent at the origin looking its own way.
pub(crate) fn ahead(space: Space, distance: f32) -> Vec3 {
    match space {
        Space::Planar => Vec3::new(distance, 0.0, 0.0),
        Space::Volume => Vec3::new(0.0, 0.0, -distance),
    }
}

/// Something perceivable at `at`.
pub(crate) fn thing(app: &mut TestApp, at: Vec3) -> Entity {
    app.spawn((Perceivable, GlobalTransform::from_translation(at)))
}
