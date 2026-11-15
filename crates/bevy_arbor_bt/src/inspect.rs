//! How a node, decorator or condition shows itself in a
//! [`BehaviorTreeTrace`](crate::tree::BehaviorTreeTrace).
//!
//! Each shows its name and parameters, what's noted above it, and its children.
//!
//! It's apart from what nodes, decorators and conditions do: [`Inspect`] is
//! how one shows, [`Inspection`] where it writes it, as `Debug` and its
//! `Formatter` are for Rust's `{:?}`. `#[derive(Inspect)]` shows a type as its
//! name, with the fields marked `#[inspect(param)]` as its parameters and
//! those marked `#[inspect(child)]` or `#[inspect(children)]` under it. One
//! wrapping a condition or decorator shows it with [`label`].
//!
//! ```
//! # use bevy_arbor_bt::prelude::*;
//! use bevy_arbor_bt::inspect::{Inspect, Inspection};
//!
//! /// Its child, `times` times.
//! struct Repeat {
//!     times: u32,
//!     child: Behavior,
//! }
//!
//! impl Inspect for Repeat {
//!     fn inspect(&self, inspection: &mut Inspection<'_>) {
//!         // Reads `Repeat(3)`, its child under it.
//!         inspection
//!             .name("Repeat")
//!             .param(self.times)
//!             .child(&self.child);
//!     }
//! }
//! ```

use std::fmt::{Display, Write};
use std::time::Duration;

pub use bevy_arbor_macros::Inspect;

use crate::behavior::Behavior;
use crate::tree::{TraceNode, write_node};

/// How something in a tree shows in a trace: a node, a decorator, or a
/// condition.
///
/// What every [`Behave`](crate::behavior::Behave),
/// [`Decorate`](crate::decorator::Decorate) and
/// [`Condition`](crate::condition::Condition) is too.
///
/// Derive it to show as the type's name, or write it for more. Built only for
/// a trace, never as a tree ticks.
pub trait Inspect {
    /// Writes how it shows into `inspection`.
    fn inspect(&self, inspection: &mut Inspection<'_>);
}

/// Where a node, decorator or condition writes how it shows in a trace.
///
/// Its name, its parameters, and, for a node, its children, each with what's
/// noted above it.
///
/// It reads as `Name(param, param)`, its children under it.
pub struct Inspection<'a> {
    /// The trace's nodes, and which one of them is being written.
    ///
    /// `None` while only a label is wanted, as of a decorator.
    nodes: Option<(&'a mut Vec<TraceNode>, usize)>,
    name: String,
    params: Vec<String>,
}

impl<'a> Inspection<'a> {
    /// An empty one, writing what's under it into `nodes`, under the one at its
    /// index.
    ///
    /// Or with no `nodes`, nothing, for a label.
    pub(crate) fn new(nodes: Option<(&'a mut Vec<TraceNode>, usize)>) -> Self {
        Self {
            nodes,
            name: String::new(),
            params: Vec::new(),
        }
    }

    /// What it's called: `Select`, a task's name, a condition's function.
    pub fn name(&mut self, name: impl Display) -> &mut Self {
        self.name.clear();
        let _ = write!(self.name, "{name}");
        self
    }

    /// One of its parameters, in brackets after its name: `Wait(1s)`.
    pub fn param(&mut self, param: impl Display) -> &mut Self {
        self.params.push(param.to_string());
        self
    }

    /// One of its children, under it.
    pub fn child(&mut self, child: &Behavior) -> &mut Self {
        self.add(None, child)
    }

    /// One of its children, under it, with `note` written above it.
    ///
    /// A note is a branch's decorator, say, or a score.
    pub fn noted(&mut self, note: impl Display, child: &Behavior) -> &mut Self {
        self.add(Some(note.to_string()), child)
    }

    /// Adds `child` under it, noted with `note`, then the child's own.
    fn add(&mut self, note: Option<String>, child: &Behavior) -> &mut Self {
        if let Some((nodes, at)) = &mut self.nodes {
            let parent = Some(*at);
            write_node(nodes, parent, note, child);
        }
        self
    }

    /// How it reads: its name, then its parameters in brackets.
    pub(crate) fn finish(self) -> String {
        let Self {
            mut name, params, ..
        } = self;
        if !params.is_empty() {
            let _ = write!(name, "({})", params.join(", "));
        }
        name
    }
}

/// How `item` reads in a trace, without what's under it: a one-line label.
///
/// A decorator's, in a branch's note, or a condition's in another's, as `!`
/// shows what it turns.
#[must_use]
pub fn label(item: &(impl Inspect + ?Sized)) -> String {
    let mut inspection = Inspection::new(None);
    item.inspect(&mut inspection);
    inspection.finish()
}

/// `length` as a parameter reads it: `1.5s`.
pub(crate) fn seconds(length: Duration) -> impl Display {
    format!("{}s", length.as_secs_f32())
}
