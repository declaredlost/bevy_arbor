//! Conditions: whether a branch may run, read off the world, and nothing else.

use std::marker::PhantomData;

use bevy::prelude::*;
use bevy::utils::prelude::ShortName;

use crate::behavior::Context;

/// Whether a branch may run, read off the world, and nothing else: no start,
/// no end. Conditions combine with `!`, `&&` and `||` into another one, in
/// `when(..)`: `when(has_target && !calm)`.
///
/// Most are functions of the agent's components, typed conditions (see
/// [`IntoCondition`]). One of your own reads anything, through
/// [`Context::world`]: another entity, a resource, time. Say what you wait on
/// as you're asked, with [`Context::watch`] or [`Context::wake_at`]: one that
/// says nothing is asked again every tick.
///
/// ```
/// use bevy::prelude::*;
/// use bevy_arbor_bt::behavior::Context;
/// use bevy_arbor_bt::decorator::Condition;
///
/// #[derive(Component)]
/// struct Target(Entity);
///
/// #[derive(Component)]
/// struct Health(f32);
///
/// /// Whether the agent's target is still standing.
/// struct TargetStanding;
///
/// impl Condition for TargetStanding {
///     fn passes(&mut self, cx: &mut Context) -> bool {
///         let Some(target) =
///             cx.world().get::<Target>(cx.agent()).map(|t| t.0)
///         else {
///             cx.watch::<Target>(cx.agent());
///             return false;
///         };
///         cx.watch::<Target>(cx.agent());
///         cx.watch::<Health>(target);
///         cx.world().get::<Health>(target).is_some_and(|h| h.0 > 0.0)
///     }
/// }
/// ```
pub trait Condition: Send + Sync + 'static {
    /// Whether its branch may run. Asked before it starts, and by a `Select`
    /// or `RepeatWhile` again each time their tree wakes, while it runs.
    fn passes(&mut self, cx: &mut Context) -> bool;

    /// How it reads in a trace: its type's short name, unless it says
    /// otherwise. Built only for a trace.
    fn label(&self) -> String {
        ShortName::of::<Self>().to_string()
    }
}

/// What a [`Condition`] can be made from: a condition, or a function of the
/// agent's components.
///
/// A function of the agent's components is a typed condition: it takes `&T` or
/// `Option<&T>` of up to four components, and returns `bool`. It's asked again
/// only once one of them changes, comes or goes, so a tree under typed
/// conditions sleeps until then. One taking `&T` the agent hasn't got doesn't
/// pass. A closure is one too, so what varies is captured: a function making
/// one returns `impl Condition`, built where the closure's type is known.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_arbor_bt::decorator::{Condition, IntoCondition};
/// #[derive(Component)]
/// struct Target(Option<Entity>);
///
/// #[derive(Component)]
/// struct Ammo(u32);
///
/// fn has_target(target: &Target) -> bool {
///     target.0.is_some()
/// }
///
/// /// At least `least` rounds left.
/// fn ammo(least: u32) -> impl Condition {
///     (move |ammo: &Ammo| ammo.0 >= least).into_condition()
/// }
///
/// let armed = has_target.and(ammo(3));
/// ```
///
/// What a typed condition can't read, the agent's other systems can put on
/// it, as components: what perception does.
#[diagnostic::on_unimplemented(
    message = "`{Self}` isn't a condition",
    label = "not a condition",
    note = "a condition is a function of the agent's components (`fn(&Target) -> bool`), or a `Condition` of your own",
    note = "one acting as its branch starts or ends, as a cooldown or a claim, goes after a comma: `when(has_target, cooldown(8.0))`"
)]
pub trait IntoCondition<M>: Sized {
    /// The condition it is.
    type Condition: Condition;

    /// It, as a condition.
    fn into_condition(self) -> Self::Condition;

    /// Passes when this and `other` both do; `other` only asked if this does.
    /// `a && b` in `when(..)`.
    fn and<N, C: IntoCondition<N>>(self, other: C) -> And<Self::Condition, C::Condition> {
        And(self.into_condition(), other.into_condition())
    }

    /// Passes when this or `other` does; `other` only asked if this doesn't.
    /// `a || b` in `when(..)`.
    fn or<N, C: IntoCondition<N>>(self, other: C) -> Or<Self::Condition, C::Condition> {
        Or(self.into_condition(), other.into_condition())
    }
}

impl<C: Condition> IntoCondition<()> for C {
    type Condition = C;

    fn into_condition(self) -> C {
        self
    }
}

/// Passes when `condition` doesn't. `!a` in `when(..)`.
pub fn not<M, C: IntoCondition<M>>(condition: C) -> Not<C::Condition> {
    Not(condition.into_condition())
}

/// Asks `passes` of something in a condition: one that says nothing it waits
/// on is asked again next tick.
pub(crate) fn asked(cx: &mut Context, passes: impl FnOnce(&mut Context) -> bool) -> bool {
    let said = cx.said();
    let passes = passes(cx);
    if cx.said() == said {
        cx.poll();
    }
    passes
}

/// Two conditions that must both pass. Made by [`IntoCondition::and`].
pub struct And<A, B>(A, B);

impl<A: Condition, B: Condition> Condition for And<A, B> {
    fn passes(&mut self, cx: &mut Context) -> bool {
        asked(cx, |cx| self.0.passes(cx)) && asked(cx, |cx| self.1.passes(cx))
    }

    fn label(&self) -> String {
        format!("({} && {})", self.0.label(), self.1.label())
    }
}

/// Two conditions, one of which must pass. Made by [`IntoCondition::or`].
pub struct Or<A, B>(A, B);

impl<A: Condition, B: Condition> Condition for Or<A, B> {
    fn passes(&mut self, cx: &mut Context) -> bool {
        asked(cx, |cx| self.0.passes(cx)) || asked(cx, |cx| self.1.passes(cx))
    }

    fn label(&self) -> String {
        format!("({} || {})", self.0.label(), self.1.label())
    }
}

/// A condition that mustn't pass. Made by [`not`].
pub struct Not<A>(A);

impl<A: Condition> Condition for Not<A> {
    fn passes(&mut self, cx: &mut Context) -> bool {
        !asked(cx, |cx| self.0.passes(cx))
    }

    fn label(&self) -> String {
        format!("!{}", self.0.label())
    }
}

/// What a typed condition, or a typed score, can take: `&T` or `Option<&T>` of
/// a component on the agent.
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

/// A function of the agent's components, taking `P` and returning `O`: what a
/// typed condition or score calls.
#[doc(hidden)]
pub trait AgentFn<P, O>: Send + Sync + 'static {
    /// It, called with what it takes off `agent`: `None` if a component it
    /// needs isn't there.
    fn call(&self, agent: EntityRef<'_>) -> Option<O>;

    /// Asks for the tree to be woken once what it takes changes.
    fn watch(cx: &mut Context);
}

/// [`AgentFn`] for functions of `params` components.
macro_rules! agent_fn {
    ($($param:ident),+) => {
        impl<F, O, $($param),+> AgentFn<($($param,)+), O> for F
        where
            F: Fn($($param),+) -> O
                + for<'a> Fn($($param::Item<'a>),+) -> O
                + Send
                + Sync
                + 'static,
            $($param: ConditionParam),+
        {
            fn call(&self, agent: EntityRef<'_>) -> Option<O> {
                #[expect(non_snake_case, reason = "named after their types")]
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

agent_fn!(A);
agent_fn!(A, B);
agent_fn!(A, B, C);
agent_fn!(A, B, C, D);

/// A function of the agent's components, as a typed condition or score.
pub struct Typed<F, P> {
    function: F,
    /// Its type's name.
    name: &'static str,
    params: PhantomData<fn() -> P>,
}

impl<F, P> Typed<F, P> {
    /// `function`, typed.
    pub(crate) fn new(function: F) -> Self {
        Self {
            function,
            name: std::any::type_name::<F>(),
            params: PhantomData,
        }
    }

    /// What it says of the agent, without waking for it: `None` if a
    /// component it needs isn't there.
    pub(crate) fn read<O>(&self, cx: &Context) -> Option<O>
    where
        F: AgentFn<P, O>,
    {
        let agent = cx.world().get_entity(cx.agent()).ok()?;
        self.function.call(agent)
    }

    /// How it reads in a trace.
    pub(crate) fn name(&self) -> String {
        ShortName(self.name).to_string()
    }
}

impl<F: AgentFn<P, bool>, P: 'static> Condition for Typed<F, P> {
    fn passes(&mut self, cx: &mut Context) -> bool {
        F::watch(cx);
        self.read(cx).unwrap_or(false)
    }

    fn label(&self) -> String {
        self.name()
    }
}

impl<F: AgentFn<P, bool>, P: 'static> IntoCondition<fn(P) -> bool> for F {
    type Condition = Typed<F, P>;

    fn into_condition(self) -> Typed<F, P> {
        Typed::new(self)
    }
}
