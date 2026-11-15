//! Smart objects in environment queries: finding free slots, and measuring.
//!
//! [`FreeSlots`] is a generator of the slots an agent could claim, and
//! [`ClaimedSlots`] an anchor at the ones it holds.

mod claimed;
mod free;

pub use self::claimed::ClaimedSlots;
pub use self::free::FreeSlots;
