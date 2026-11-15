//! Asking for a query to be run for an agent, and what it found.
//!
//! A [`Survey<Q>`] runs the query keyed by `Q`, added with
//! [`EnvQueryApp::add_env_query`], and puts what it found on the agent as a
//! [`Found<Q>`].

mod found;
mod registry;
mod request;
mod run;
mod trace;

pub use self::found::{Found, found};
pub use self::registry::EnvQueryApp;
pub(crate) use self::registry::{EnvQueries, Pending, Seed};
pub use self::request::Survey;
pub(crate) use self::run::run;
pub use self::trace::{EnvQueryTrace, Weighed};
