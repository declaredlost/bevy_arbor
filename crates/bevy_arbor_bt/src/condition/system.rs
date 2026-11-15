use std::fmt::Debug;

use bevy::prelude::*;

use super::{Condition, IntoCondition};
use crate::behavior::Context;
use crate::behavior::system::Check;
use crate::inspect::{Inspect, Inspection};

/// A Bevy [`SystemCondition`] on the agent, as a condition: asked every tick.
///
/// It can read anything, so it says nothing it waits on. Never named: it's
/// what a system condition turns into.
#[doc(hidden)]
pub struct ConditionSystem(Check<bool>);

impl Condition for ConditionSystem {
    fn passes(&mut self, cx: &mut Context) -> bool {
        self.0.run(cx).unwrap_or(false)
    }
}

impl Inspect for ConditionSystem {
    fn inspect(&self, inspection: &mut Inspection<'_>) {
        inspection.name(self.0.name());
    }
}

impl<S, M> IntoCondition<fn(M)> for S
where
    S: SystemCondition<M, In<Entity>> + Copy + Send + Sync + 'static,
{
    type Condition = ConditionSystem;

    fn into_condition(self) -> ConditionSystem {
        ConditionSystem(Check::new(self))
    }
}

/// A system condition taking the agent and `param`: `when(with(within, 6.0))`.
///
/// As many conditions as you like, from one function.
pub fn with<S, P, M>(system: S, param: P) -> impl Condition
where
    S: SystemCondition<M, In<(Entity, P)>> + Copy + Send + Sync + 'static,
    P: Clone + Debug + Send + Sync + 'static,
{
    ConditionSystem(Check::with(system, param))
}
