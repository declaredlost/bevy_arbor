//! The space perception works in, and the grid it finds things through.

mod grid;

use bevy::prelude::*;

pub(crate) use self::grid::Grid;

/// The space perception works in: what's ahead of an agent, and what distance
/// is. One code path serves both; only these differ.
#[derive(Resource, Reflect, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[reflect(Resource, Debug, Default, PartialEq)]
pub enum Space {
    /// 2D, in the XY plane: z, which 2D uses for layering, is ignored. Ahead
    /// is the agent's +X, so a sprite flipped by a negative x scale looks the
    /// other way.
    Planar,
    /// 3D. Ahead is the agent's -Z, Bevy's forward.
    #[default]
    Volume,
}

impl Space {
    /// `point` in this space: in the plane, z dropped. What a sense of your
    /// own senses from, so it measures as the built-in ones do.
    #[must_use]
    pub fn flatten(self, point: Vec3) -> Vec3 {
        match self {
            Self::Planar => point.with_z(0.0),
            Self::Volume => point,
        }
    }

    /// Which way `transform` looks, unit length, or zero if it can't tell.
    pub(crate) fn ahead(self, transform: &GlobalTransform) -> Vec3 {
        let ahead = match self {
            Self::Planar => transform.affine().matrix3 * Vec3::X,
            Self::Volume => transform.affine().matrix3 * Vec3::NEG_Z,
        };
        self.flatten(ahead).normalize_or_zero()
    }

    /// The axes a cone turns around, to be drawn: in 2D the plane's normal; in
    /// 3D the agent's up and right, a fan each way.
    #[cfg(feature = "gizmos")]
    pub(crate) fn turns(self, transform: &GlobalTransform) -> Vec<Vec3> {
        match self {
            Self::Planar => vec![Vec3::Z],
            Self::Volume => {
                let axes = transform.affine().matrix3;
                vec![
                    (axes * Vec3::Y).try_normalize().unwrap_or(Vec3::Y),
                    (axes * Vec3::X).try_normalize().unwrap_or(Vec3::X),
                ]
            }
        }
    }

    /// The axis a range is drawn around: the plane's normal in 2D, up in 3D.
    #[cfg(feature = "gizmos")]
    pub(crate) fn ring_axis(self) -> Vec3 {
        match self {
            Self::Planar => Vec3::Z,
            Self::Volume => Vec3::Y,
        }
    }

    /// The grid cell `point` is in, for cells `size` across.
    pub(crate) fn cell(self, point: Vec3, size: f32) -> IVec3 {
        let cell = (point / size).floor().as_ivec3();
        match self {
            Self::Planar => cell.with_z(0),
            Self::Volume => cell,
        }
    }

    /// How many cells deep the grid is either side of a cell, `reach` across.
    pub(crate) fn depth(self, reach: i32) -> i32 {
        match self {
            Self::Planar => 0,
            Self::Volume => reach,
        }
    }
}
