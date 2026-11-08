//! What using perception takes: `use bevy_arbor_perception::prelude::*`.

#[doc(no_inline)]
pub use crate::Space;
#[doc(no_inline)]
pub use crate::perceived::{
    Attitude, Candidates, Forgotten, Lost, Noticed, Perceived, Percept, Team, Trace,
};
#[doc(no_inline)]
pub use crate::sense::{
    Facing, Hearing, Listeners, Noise, Perceivable, Sense, SensePlugin, SensorOf, Sensors, Share,
    Shared, Sight, Touch, Touched,
};
#[doc(no_inline)]
pub use crate::{PerceptionPlugin, PerceptionSystems};
