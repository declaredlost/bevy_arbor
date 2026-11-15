use std::marker::PhantomData;

use bevy::prelude::*;
use bevy::utils::prelude::ShortName;
use variadics_please::all_tuples;

use super::{Condition, IntoCondition};
use crate::behavior::Context;
use crate::inspect::{Inspect, Inspection};

/// What a typed condition can take: `&T` or `Option<&T>` of a component on the
/// agent.
pub trait ConditionParam: 'static {
    /// What the function is given.
    type Item<'a>;

    /// It, off `agent`: `None` if a component it needs isn't there.
    #[doc(hidden)]
    fn fetch(agent: EntityRef<'_>) -> Option<Self::Item<'_>>;

    /// Asks for the tree to be woken once it changes.
    #[doc(hidden)]
    fn watch(cx: &mut Context);
}

impl<T: Component> ConditionParam for &'static T {
    type Item<'a> = &'a T;

    fn fetch(agent: EntityRef<'_>) -> Option<&T> {
        agent.get::<T>()
    }

    fn watch(cx: &mut Context) {
        let agent = cx.agent();
        cx.watch::<T>(agent);
    }
}

impl<T: Component> ConditionParam for Option<&'static T> {
    type Item<'a> = Option<&'a T>;

    fn fetch(agent: EntityRef<'_>) -> Option<Option<&T>> {
        Some(agent.get::<T>())
    }

    fn watch(cx: &mut Context) {
        let agent = cx.agent();
        cx.watch::<T>(agent);
    }
}

/// A function of the agent's components, taking `P`: what a typed condition
/// calls.
#[doc(hidden)]
pub trait AgentFn<P>: Send + Sync + 'static {
    /// It, called with what it takes off `agent`: `None` if a component it
    /// needs isn't there.
    fn call(&self, agent: EntityRef<'_>) -> Option<bool>;

    /// Asks for the tree to be woken once what it takes changes.
    fn watch(cx: &mut Context);
}

/// [`AgentFn`] for functions of `params` components.
macro_rules! agent_fn {
    ($($param:ident),*) => {
        impl<F, $($param),*> AgentFn<($($param,)*)> for F
        where
            F: Fn($($param),+) -> bool
                + for<'a> Fn($($param::Item<'a>),+) -> bool
                + Send
                + Sync
                + 'static,
            $($param: ConditionParam),+
        {
            fn call(&self, agent: EntityRef<'_>) -> Option<bool> {
                let ($(Some($param),)+) = ($($param::fetch(agent),)+) else {
                    return None;
                };
                Some(self($($param),+))
            }

            fn watch(cx: &mut Context) {
                $($param::watch(cx);)+
            }
        }
    };
}

all_tuples!(agent_fn, 1, 4, P);

/// A function of the agent's components, as a condition: asked once they
/// change.
pub struct TypedCondition<F, P> {
    function: F,
    params: PhantomData<fn() -> P>,
}

impl<F: AgentFn<P>, P: 'static> Condition for TypedCondition<F, P> {
    fn passes(&mut self, cx: &mut Context) -> bool {
        F::watch(cx);
        let agent = cx.world().get_entity(cx.agent()).ok();
        agent
            .and_then(|agent| self.function.call(agent))
            .unwrap_or(false)
    }
}

impl<F, P> Inspect for TypedCondition<F, P> {
    fn inspect(&self, inspection: &mut Inspection<'_>) {
        inspection.name(ShortName(std::any::type_name::<F>()));
    }
}

impl<F: AgentFn<P>, P: 'static> IntoCondition<fn(P) -> bool> for F {
    type Condition = TypedCondition<F, P>;

    fn into_condition(self) -> TypedCondition<F, P> {
        TypedCondition {
            function: self,
            params: PhantomData,
        }
    }
}
