use std::fmt::Debug;

use bevy::prelude::*;

use super::{Condition, IntoCondition};
use crate::behavior::Context;
use crate::behavior::system::{Check, Param, WithParam};
use crate::inspect::{Inspect, Inspection};

/// A Bevy [`SystemCondition`] on the agent, as a condition: asked every tick.
///
/// It can read anything, so it says nothing it waits on.
struct ConditionSystem(Check<bool>);

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
    fn into_condition(self) -> impl Condition {
        ConditionSystem(Check::new(self))
    }
}

/// A system condition taking the agent and a parameter, made by
/// [`with`](crate::behavior::with): `when(with(within, 6.0))`.
impl<S, P, M> IntoCondition<Param<M>> for WithParam<S, P>
where
    S: SystemCondition<M, In<(Entity, P)>> + Copy + Send + Sync + 'static,
    P: Clone + Debug + Send + Sync + 'static,
{
    fn into_condition(self) -> impl Condition {
        ConditionSystem(Check::with(self.system, self.param))
    }
}
