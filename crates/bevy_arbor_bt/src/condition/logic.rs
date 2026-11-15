use super::{Condition, IntoCondition};
use crate::behavior::Context;
use crate::inspect::{Inspect, Inspection, label};

/// Passes when `condition` doesn't: what `!a` in `when(..)` is, outside it.
pub fn not<M, C: IntoCondition<M>>(condition: C) -> Not<C::Condition> {
    Not(condition.into_condition())
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

/// Two conditions that must both pass. Made by [`IntoCondition::and`].
pub struct And<A, B>(pub(super) A, pub(super) B);

impl<A: Condition, B: Condition> Condition for And<A, B> {
    fn passes(&mut self, cx: &mut Context) -> bool {
        poll_if_silent(cx, |cx| self.0.passes(cx)) && poll_if_silent(cx, |cx| self.1.passes(cx))
    }
}

impl<A: Inspect, B: Inspect> Inspect for And<A, B> {
    fn inspect(&self, inspection: &mut Inspection<'_>) {
        inspection.name(format_args!("({} && {})", label(&self.0), label(&self.1)));
    }
}

/// Two conditions, one of which must pass. Made by [`IntoCondition::or`].
pub struct Or<A, B>(pub(super) A, pub(super) B);

impl<A: Condition, B: Condition> Condition for Or<A, B> {
    fn passes(&mut self, cx: &mut Context) -> bool {
        poll_if_silent(cx, |cx| self.0.passes(cx)) || poll_if_silent(cx, |cx| self.1.passes(cx))
    }
}

impl<A: Inspect, B: Inspect> Inspect for Or<A, B> {
    fn inspect(&self, inspection: &mut Inspection<'_>) {
        inspection.name(format_args!("({} || {})", label(&self.0), label(&self.1)));
    }
}

/// A condition that mustn't pass: another, turned. Made by [`not`].
pub struct Not<A>(A);

impl<A: Condition> Condition for Not<A> {
    fn passes(&mut self, cx: &mut Context) -> bool {
        !poll_if_silent(cx, |cx| self.0.passes(cx))
    }
}

impl<A: Inspect> Inspect for Not<A> {
    fn inspect(&self, inspection: &mut Inspection<'_>) {
        inspection.name(format_args!("!{}", label(&self.0)));
    }
}
