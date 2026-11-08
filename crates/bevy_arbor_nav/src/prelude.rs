//! What using navigation takes: `use bevy_arbor_nav::prelude::*`.

#[doc(no_inline)]
pub use crate::move_to::MoveTo;
#[cfg(feature = "eqs")]
#[doc(no_inline)]
pub use crate::path::{PathCostsApp, PathDistance};
#[doc(no_inline)]
pub use crate::travel::{Destination, Goal, Route, Travel};
#[doc(no_inline)]
pub use crate::{NavPlugin, NavSystems};
