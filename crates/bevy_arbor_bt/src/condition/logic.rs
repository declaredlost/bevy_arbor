use variadics_please::all_tuples_enumerated;

use super::{Condition, IntoCondition};
use crate::behavior::Context;
use crate::inspect::{Inspect, Inspection, label};

/// Passes when every one of `conditions` does: `all((has_target, armed))`.
///
/// `a && b` in `when(..)`. Each, of a tuple of up to eight, is asked only if
/// those before it passed.
pub fn all<M, C: IntoConditions<M>>(conditions: C) -> impl Condition {
    All(conditions.into_conditions())
}

/// Passes when any one of `conditions` does: `any((afraid, hurt))`.
///
/// `a || b` in `when(..)`. Each, of a tuple of up to eight, is asked only if
/// those before it didn't.
pub fn any<M, C: IntoConditions<M>>(conditions: C) -> impl Condition {
    Any(conditions.into_conditions())
}

/// Passes when `condition` doesn't: `unless(calm)`, or `!calm` in `when(..)`.
pub fn unless<M, C: IntoCondition<M>>(condition: C) -> impl Condition {
    Unless(condition.into_condition())
}

/// Asks `passes` of something in a condition, polling it if it says nothing.
///
/// One that says nothing it waits on is asked again next tick.
pub(crate) fn poll_if_silent(cx: &mut Context, passes: impl FnOnce(&mut Context) -> bool) -> bool {
    let said = cx.said();
    let passes = passes(cx);
    if cx.said() == said {
        cx.poll();
    }
    passes
}

/// What [`all`] and [`any`] take: a tuple of what [`IntoCondition`] takes.
#[diagnostic::on_unimplemented(
    message = "`{Self}` isn't a tuple of conditions",
    label = "not a tuple of conditions",
    note = "`all` and `any` take a tuple of up to eight conditions: `all((has_target, armed))`"
)]
pub trait IntoConditions<M> {
    /// Them, as conditions.
    #[doc(hidden)]
    fn into_conditions(self) -> impl Conditions;
}

/// Conditions asked in order, as [`all`] and [`any`] ask them.
#[doc(hidden)]
pub trait Conditions: Send + Sync + 'static {
    /// Whether every one passes, each asked only if those before it did.
    fn all(&mut self, cx: &mut Context) -> bool;

    /// Whether any one passes, each asked only if those before it didn't.
    fn any(&mut self, cx: &mut Context) -> bool;

    /// How each reads, in order.
    fn labels(&self) -> Vec<String>;
}

/// [`IntoConditions`] and [`Conditions`] for tuples of `conditions`.
macro_rules! conditions {
    ($(($index:tt, $condition:ident, $marker:ident)),*) => {
        impl<$($condition: IntoCondition<$marker>, $marker),*> IntoConditions<($($marker,)*)>
            for ($($condition,)*)
        {
            fn into_conditions(self) -> impl Conditions {
                ($(self.$index.into_condition(),)*)
            }
        }

        impl<$($condition: Condition),*> Conditions for ($($condition,)*) {
            fn all(&mut self, cx: &mut Context) -> bool {
                $(poll_if_silent(cx, |cx| self.$index.passes(cx)))&&*
            }

            fn any(&mut self, cx: &mut Context) -> bool {
                $(poll_if_silent(cx, |cx| self.$index.passes(cx)))||*
            }

            fn labels(&self) -> Vec<String> {
                vec![$(label(&self.$index)),*]
            }
        }
    };
}

all_tuples_enumerated!(conditions, 1, 8, C, M);

/// Conditions that must all pass. Made by [`all`].
struct All<C>(C);

impl<C: Conditions> Condition for All<C> {
    fn passes(&mut self, cx: &mut Context) -> bool {
        self.0.all(cx)
    }
}

impl<C: Conditions> Inspect for All<C> {
    fn inspect(&self, inspection: &mut Inspection<'_>) {
        inspection.name(format_args!("({})", self.0.labels().join(" && ")));
    }
}

/// Conditions, one of which must pass. Made by [`any`].
struct Any<C>(C);

impl<C: Conditions> Condition for Any<C> {
    fn passes(&mut self, cx: &mut Context) -> bool {
        self.0.any(cx)
    }
}

impl<C: Conditions> Inspect for Any<C> {
    fn inspect(&self, inspection: &mut Inspection<'_>) {
        inspection.name(format_args!("({})", self.0.labels().join(" || ")));
    }
}

/// A condition that mustn't pass. Made by [`unless`].
struct Unless<C>(C);

impl<C: Condition> Condition for Unless<C> {
    fn passes(&mut self, cx: &mut Context) -> bool {
        !poll_if_silent(cx, |cx| self.0.passes(cx))
    }
}

impl<C: Condition> Inspect for Unless<C> {
    fn inspect(&self, inspection: &mut Inspection<'_>) {
        inspection.name(format_args!("!{}", label(&self.0)));
    }
}
