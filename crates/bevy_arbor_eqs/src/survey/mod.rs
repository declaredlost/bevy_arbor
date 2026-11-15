//! Asking for a query to be run for an agent, and what it found.
//!
//! A [`Survey<K>`] runs the [`EnvQuery<K>`](crate::query::EnvQuery) the agent
//! carries, or something it asks it of carries, and puts what it found on the
//! agent as a [`Found<K>`].

mod found;
mod request;
mod run;
mod trace;

pub use self::found::{Found, found};
pub(crate) use self::request::Pending;
pub use self::request::Survey;
pub(crate) use self::run::run;
pub use self::trace::EnvQueryTrace;
