//! Where 2D and 3D differ: perception's points as avian's.

use bevy::prelude::*;

use crate::avian::math::{RVector, ToRealPrecision};

/// `point`, from perception, as avian takes it.
#[cfg(dim = "2d")]
pub(crate) fn point(point: Vec3) -> RVector {
    point.truncate().real()
}

/// `point`, from perception, as avian takes it.
#[cfg(dim = "3d")]
pub(crate) fn point(point: Vec3) -> RVector {
    point.real()
}

/// The way along `towards`, if it has one.
#[cfg(dim = "2d")]
pub(crate) fn direction(towards: Vec3) -> Option<Dir2> {
    Dir2::new(towards.truncate()).ok()
}

/// The way along `towards`, if it has one.
#[cfg(dim = "3d")]
pub(crate) fn direction(towards: Vec3) -> Option<Dir3> {
    Dir3::new(towards).ok()
}

/// How far it is from `from` to `to`, in avian's plane or space.
#[cfg(dim = "2d")]
pub(crate) fn distance(from: Vec3, to: Vec3) -> f32 {
    from.truncate().distance(to.truncate())
}

/// How far it is from `from` to `to`, in avian's plane or space.
#[cfg(dim = "3d")]
pub(crate) fn distance(from: Vec3, to: Vec3) -> f32 {
    from.distance(to)
}
