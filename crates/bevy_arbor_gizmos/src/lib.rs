#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

#[cfg(feature = "eqs")]
#[cfg_attr(docsrs, doc(cfg(feature = "eqs")))]
pub mod eqs;
#[cfg(feature = "nav")]
#[cfg_attr(docsrs, doc(cfg(feature = "nav")))]
pub mod nav;
pub mod perception;
mod plugin;
pub mod prelude;
#[cfg(feature = "smart")]
#[cfg_attr(docsrs, doc(cfg(feature = "smart")))]
pub mod smart;
#[cfg(test)]
mod testing;

pub use self::plugin::ArborGizmosPlugin;
