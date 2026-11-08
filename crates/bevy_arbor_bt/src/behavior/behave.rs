use bevy::utils::prelude::ShortName;

use crate::behavior::{Child, Context, Outcome, Status};

/// A kind of node: what the built-in nodes implement, and custom ones do too.
///
/// The built-in ones are written against this and nothing else, so a custom one
/// can do whatever they do. Put one in a tree with `@ MyNode::new(..)`, or
/// `Behavior::from(..)`.
///
/// A node runs from [`start`](Self::start), through each
/// [`tick`](Self::tick), to [`end`](Self::end): the tree calls them, so a node
/// only says what it does at each. It may run again in the same tick it ends.
///
/// Its children are [`Behavior`](crate::behavior::Behavior)s. A child's hold
/// passes up on its own; on a tick a running child isn't ticked, call
/// [`skip`](crate::behavior::Behavior::skip) on it, and to switch away from
/// one that [holds](crate::behavior::Behavior::holds), use
/// [`tick_interrupted`](crate::behavior::Behavior::tick_interrupted).
///
/// Trees sleep until something wakes them: say what you wait on as you tick,
/// a time ([`Context::wake_at`]) or a component ([`Context::watch`]). What your
/// children wait on they say themselves; a running node whose tick, its
/// children's included, said nothing is ticked again next tick.
///
/// ```
/// use bevy_arbor_bt::behavior::{Behave, Child, Context};
/// use bevy_arbor_bt::prelude::*;
///
/// /// Runs its child `times` times, unless it fails.
/// struct Repeat {
///     times: u32,
///     done: u32,
///     child: Behavior,
/// }
///
/// impl Behave for Repeat {
///     fn start(&mut self, _cx: &mut Context) {
///         self.done = 0;
///     }
///
///     fn tick(&mut self, cx: &mut Context) -> Status {
///         match self.child.tick(cx) {
///             Status::Success => {
///                 self.done += 1;
///                 if self.done < self.times {
///                     return Status::Running;
///                 }
///                 Status::Success
///             }
///             status => status,
///         }
///     }
///
///     fn end(&mut self, cx: &mut Context, _outcome: Option<Outcome>) {
///         self.child.cancel(cx);
///     }
///
///     fn label(&self) -> String {
///         format!("Repeat({})", self.times)
///     }
///
///     fn children(&self) -> Vec<Child<'_>> {
///         vec![Child::new(&self.child)]
///     }
/// }
/// ```
pub trait Behave: Send + Sync + 'static {
    /// Starts it, before its first tick each time it runs.
    fn start(&mut self, _cx: &mut Context) {}

    /// Runs it a tick: [`Running`](Status::Running) until it's done.
    fn tick(&mut self, cx: &mut Context) -> Status;

    /// Ends it, as it's done, with how it went, or as it's cancelled, with
    /// `None`. It has to cancel its children
    /// ([`Behavior::cancel`](crate::behavior::Behavior::cancel), which leaves
    /// one that isn't running as it is), or their tasks are left running.
    fn end(&mut self, cx: &mut Context, outcome: Option<Outcome>);

    /// How it reads in a trace: its type's short name, unless it says
    /// otherwise. Built only for a trace.
    fn label(&self) -> String {
        ShortName::of::<Self>().to_string()
    }

    /// Its children, each with what's written above it: for a trace, and
    /// anything else walking the tree. None, unless it says otherwise.
    ///
    /// Leaving one out only leaves it out of those: cancelling them is
    /// [`end`](Self::end)'s.
    fn children(&self) -> Vec<Child<'_>> {
        Vec::new()
    }
}
