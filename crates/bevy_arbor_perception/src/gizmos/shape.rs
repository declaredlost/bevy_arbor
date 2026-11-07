//! The shapes perception is drawn with, in either space.

use std::f32::consts::{PI, TAU};

use bevy::prelude::*;

/// How many segments a full circle is drawn with.
const SEGMENTS: u16 = 48;

/// A cone's outline from `apex`: `half_angle` either side of `ahead`, turning
/// around `axis`, out to `radius`. A full turn is a closed ring without the
/// apex.
pub(super) fn cone(apex: Vec3, ahead: Vec3, axis: Vec3, half_angle: f32, radius: f32) -> Vec<Vec3> {
    let full = half_angle >= PI;
    let sweep = if full { TAU } else { 2.0 * half_angle };
    let segments = (f32::from(SEGMENTS) * sweep / TAU).ceil().max(1.0);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "at least one, and at most `SEGMENTS`"
    )]
    let steps = segments as u16;
    let arc = (0..=steps).map(|step| {
        let angle = -sweep / 2.0 + sweep * f32::from(step) / f32::from(steps);
        apex + Quat::from_axis_angle(axis, angle) * ahead * radius
    });
    if full {
        arc.collect()
    } else {
        std::iter::once(apex)
            .chain(arc)
            .chain(std::iter::once(apex))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_PI_4;

    use bevy::prelude::*;

    use super::cone;
    use crate::testing::*;

    #[test]
    fn a_cone_reaches_its_range_at_either_edge_of_its_angle() {
        for space in SPACES {
            let (ahead, axis) = match space {
                crate::space::Space::Planar => (Vec3::X, Vec3::Z),
                crate::space::Space::Volume => (Vec3::NEG_Z, Vec3::Y),
            };
            let points = cone(Vec3::ZERO, ahead, axis, FRAC_PI_4, 10.0);
            assert_eq!(
                points.first(),
                Some(&Vec3::ZERO),
                "{space:?}: from the apex"
            );
            assert_eq!(points.last(), Some(&Vec3::ZERO), "{space:?}: back to it");
            let edges = [points[1], points[points.len() - 2]];
            for edge in edges {
                assert!((edge.length() - 10.0).abs() < 1e-4, "{space:?}: {edge}");
                let off = edge.normalize().angle_between(ahead);
                assert!((off - FRAC_PI_4).abs() < 1e-4, "{space:?}: {off}");
            }
        }
    }

    #[test]
    fn a_full_turn_is_a_closed_ring() {
        let points = cone(Vec3::ONE, Vec3::X, Vec3::Z, std::f32::consts::PI, 5.0);
        assert!(points.first().unwrap().distance(*points.last().unwrap()) < 1e-4);
        assert!(
            points
                .iter()
                .all(|point| (point.distance(Vec3::ONE) - 5.0).abs() < 1e-4)
        );
    }
}
