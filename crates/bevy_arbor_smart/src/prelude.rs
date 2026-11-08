//! What using smart objects takes: `use bevy_arbor_smart::prelude::*`.

#[doc(no_inline)]
pub use crate::SmartPlugin;
#[doc(no_inline)]
pub use crate::claim::{Claim, ClaimedBy, Claims, claim};
#[doc(no_inline)]
pub use crate::query::{ClaimedSlots, FreeSlots};
#[doc(no_inline)]
pub use crate::slot::{SlotDisabled, SlotOf, Slots, Using};
