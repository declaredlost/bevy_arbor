#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

// One source for 2D and 3D: `bevy_arbor_avian2d` and `bevy_arbor_avian3d`
// build it, each setting its dimension in its build script, as `dim`.
#[cfg(not(any(dim = "2d", dim = "3d")))]
compile_error!("build `bevy_arbor_avian2d` or `bevy_arbor_avian3d`, not this source alone");

#[cfg(dim = "2d")]
use avian2d as avian;
#[cfg(dim = "3d")]
use avian3d as avian;
use bevy::app::plugin_group;

mod dim;
mod occlusion;
pub mod prelude;
mod touch;

pub use self::occlusion::{LineOfSightPlugin, Occluders, SensorOccluders, SightPoints};
pub use self::touch::ContactTouchPlugin;

plugin_group! {
    /// Line of sight for sight, and touch from contacts.
    #[derive(Default)]
    pub struct AvianPerceptionPlugins {
        occlusion:::LineOfSightPlugin,
        touch:::ContactTouchPlugin,
    }
}

#[cfg(test)]
mod testing;
