//! What a node is: [`Behave`], the trait built-in and custom nodes alike
//! implement, and what it runs with.

mod context;
mod status;

pub use self::context::Context;
pub use self::status::{Outcome, Status};
use crate::tree::TreeOutline;

/// A kind of node: what the built-in nodes implement, and custom ones do too.
///
/// The built-in ones are written against this and nothing else, so a custom one
/// can do whatever they do. Put one in a tree with `@ MyNode::new(..)`, or
/// `Behaviour::from(..)`.
///
/// Its children are [`Behaviour`](crate::behaviour::Behaviour)s. A child's hold
/// passes up on its own; on a tick a running child isn't ticked, call
/// [`skip`](crate::behaviour::Behaviour::skip) on it, and to switch away from
/// one that [holds](crate::behaviour::Behaviour::holds), use
/// [`tick_interrupted`](crate::behaviour::Behaviour::tick_interrupted).
///
/// ```
/// use bevy_arbor_bt::behaviour::{Behave, Context};
/// use bevy_arbor_bt::prelude::*;
/// use bevy_arbor_bt::tree::TreeOutline;
///
/// /// Runs its child `times` times, unless it fails.
/// struct Repeat {
///     times: u32,
///     done: u32,
///     child: Behaviour,
/// }
///
/// impl Behave for Repeat {
///     fn tick(&mut self, cx: &mut Context) -> Status {
///         match self.child.tick(cx) {
///             Status::Success => {
///                 self.done += 1;
///                 if self.done < self.times {
///                     return Status::Running;
///                 }
///                 self.done = 0;
///                 Status::Success
///             }
///             Status::Failure => {
///                 self.done = 0;
///                 Status::Failure
///             }
///             Status::Running => Status::Running,
///         }
///     }
///
///     fn cancel(&mut self, cx: &mut Context) {
///         self.done = 0;
///         self.child.cancel(cx);
///     }
///
///     fn outline(&self, out: &mut TreeOutline) {
///         out.line(format!("Repeat({})", self.times));
///         out.child(&self.child);
///     }
/// }
/// ```
pub trait Behave: Send + Sync + 'static {
    /// Runs it a tick. It may run again in the same tick if it ends.
    fn tick(&mut self, cx: &mut Context) -> Status;

    /// Stops it at once, children too, ready to start again. It has to pass
    /// this on to its children, or their tasks are left running.
    fn cancel(&mut self, cx: &mut Context);

    /// Whether it has to be ticked every tick while it runs: yes, unless it
    /// says otherwise.
    ///
    /// Trees sleep until something wakes them. A node saying no tells its
    /// context what it waits on instead, when it runs: a time
    /// ([`Context::wake_at`]), or a component ([`Context::watch`]). What its
    /// children wait on they say themselves. The built-in nodes all say no.
    fn polls(&self) -> bool {
        true
    }

    /// How it looks in a tree's outline: its type's name, and none of its
    /// children, unless it says otherwise.
    fn outline(&self, out: &mut TreeOutline) {
        out.line(TreeOutline::short::<Self>());
    }
}
