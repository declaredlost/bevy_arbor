//! The shapes generators lay points out in: round a center, in steps.

use std::f32::consts::TAU;

use bevy::prelude::*;
use bevy_arbor_perception::Space;

/// The point `radius` from `center` at `angle` round it, in the plane things
/// stand on.
///
/// XY in 2D, XZ in 3D, where 0 is +X.
pub(crate) fn round(space: Space, center: Vec3, angle: f32, radius: f32) -> Vec3 {
    let (sin, cos) = angle.sin_cos();
    match space {
        Space::Planar => center + Vec3::new(cos, sin, 0.0) * radius,
        Space::Volume => center + Vec3::new(cos, 0.0, -sin) * radius,
    }
}

/// How many points a grid or row has each way of its anchor, at most.
///
/// One asking for more is spread further apart, so a tiny spacing can't hang
/// the game.
pub(crate) const MOST_STEPS: i32 = 50;

/// How many points each way, and how far apart, out to `extent`.
///
/// Wanted `spacing` apart, by the generator `name`. More than [`MOST_STEPS`]
/// are spread further apart instead, with a warning.
pub(crate) fn steps(name: &str, extent: f32, spacing: f32) -> (i32, f32) {
    let wanted = (extent / spacing).floor();
    #[expect(clippy::cast_precision_loss, reason = "a small step count")]
    let most = MOST_STEPS as f32;
    if wanted > most {
        let spread = extent / most;
        warn_once!(
            "a {name} {extent} out, {spacing} apart, is too many points: spread {spread} apart instead"
        );
        return (MOST_STEPS, spread);
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "at most MOST_STEPS, and never negative"
    )]
    (wanted as i32, spacing)
}

/// `count` angles evenly round a circle.
pub(crate) fn angles(count: u32) -> impl Iterator<Item = f32> {
    #[expect(
        clippy::cast_precision_loss,
        reason = "a count of points, far below where f32 loses whole numbers"
    )]
    let step = TAU / count.max(1) as f32;
    #[expect(clippy::cast_precision_loss, reason = "as above")]
    (0..count).map(move |index| index as f32 * step)
}
