//! What running queries takes: `use bevy_arbor_eqs::prelude::*`.
//!
//! Writing a generator, test or anchor of your own takes
//! [`Generate`](crate::generate::Generate), [`Test`](crate::test::Test) or
//! [`Locate`](crate::anchor::Locate), and
//! [`QueryContext`](crate::query::QueryContext), too.

#[doc(no_inline)]
pub use crate::anchor::{Agent, At, Hostiles, Thing};
#[doc(no_inline)]
pub use crate::generate::{Donut, Fan, Grid, Nearby, Percepts};
#[doc(no_inline)]
pub use crate::query::{EnvQuery, Filter, Item, Pick, Score, Scored};
#[doc(no_inline)]
pub use crate::survey::{EnvQueryApp, Found, Survey, SurveyTrace, found};
#[doc(no_inline)]
pub use crate::test::{Batch, Distance, Dot, Measure, Perceives, Random};
#[doc(no_inline)]
pub use crate::{EqsBudget, EqsPlugin, EqsSystems};
