//! Where 2D and 3D differ: perception's points as avian's.

use bevy::prelude::*;

use crate::avian::math::{RVector, ToRealPrecision};
use crate::avian::prelude::Collider;

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

/// The way along `toward`, if it has one.
#[cfg(dim = "2d")]
pub(crate) fn direction(toward: Vec3) -> Option<Dir2> {
    Dir2::new(toward.truncate()).ok()
}

/// The way along `toward`, if it has one.
#[cfg(dim = "3d")]
pub(crate) fn direction(toward: Vec3) -> Option<Dir3> {
    Dir3::new(toward).ok()
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

/// No turn: a shape as it's built.
#[cfg(dim = "2d")]
pub(crate) fn upright() -> Rot2 {
    Rot2::IDENTITY
}

/// No turn: a shape as it's built.
#[cfg(dim = "3d")]
pub(crate) fn upright() -> Quat {
    Quat::IDENTITY
}

/// A ball `radius` across, as avian makes one.
#[cfg(dim = "2d")]
pub(crate) fn ball(radius: f32) -> Collider {
    Collider::circle(radius.real())
}

/// A ball `radius` across, as avian makes one.
#[cfg(dim = "3d")]
pub(crate) fn ball(radius: f32) -> Collider {
    Collider::sphere(radius.real())
}
