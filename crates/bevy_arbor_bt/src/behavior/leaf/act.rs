use std::fmt::Debug;

use bevy::prelude::*;

use crate::behavior::system::{Check, Param, WithParam};
use crate::behavior::{Behave, Behavior, Context, Outcome, Status};
use crate::inspect::{Inspect, Inspection};

/// What an act can be made from: a system taking the agent, or one taking a
/// parameter too, made by [`with`](crate::behavior::with).
#[diagnostic::on_unimplemented(
    message = "`{Self}` isn't an act",
    label = "not an act",
    note = "an act is a system taking `In<Entity>` and returning `bool` or `Result<bool>`, or `with(system, param)` for one taking `In<(Entity, P)>`"
)]
pub trait IntoAct<M> {
    /// The node running it.
    fn into_act(self) -> Behavior;
}

impl<S, M> IntoAct<fn(M)> for S
where
    S: IntoSystem<In<Entity>, bool, M> + Copy + Send + Sync + 'static,
{
    fn into_act(self) -> Behavior {
        Act(Check::new(self)).into()
    }
}

impl<S, P, M> IntoAct<Param<M>> for WithParam<S, P>
where
    S: IntoSystem<In<(Entity, P)>, bool, M> + Copy + Send + Sync + 'static,
    P: Clone + Debug + Send + Sync + 'static,
{
    fn into_act(self) -> Behavior {
        Act(Check::with(self.system, self.param)).into()
    }
}

/// Runs a system once: what it returns is the result.
struct Act(Check<bool>);

impl Behave for Act {
    fn tick(&mut self, cx: &mut Context) -> Status {
        Outcome::from(self.0.run(cx).unwrap_or(false)).into()
    }

    fn end(&mut self, _cx: &mut Context, _outcome: Option<Outcome>) {}
}

impl Inspect for Act {
    fn inspect(&self, inspection: &mut Inspection<'_>) {
        inspection.name(format_args!("act {}", self.0.name()));
    }
}
