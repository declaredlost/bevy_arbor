//! What `Scored` and `Random` share: trying their nodes in an order settled as
//! they start, until one succeeds.

use crate::behavior::{Behavior, Context, Status};

/// An order to try nodes in, and how far along it is.
#[derive(Default)]
pub(super) struct Order {
    order: Vec<usize>,
    at: usize,
}

impl Order {
    /// Settles it, from the start.
    pub(super) fn start(&mut self, order: Vec<usize>) {
        self.order = order;
        self.at = 0;
    }

    /// Ticks `items` in order, from where it got to, until one succeeds; `node`
    /// finds each one's node.
    pub(super) fn tick<T>(
        &mut self,
        items: &mut [T],
        node: fn(&mut T) -> &mut Behavior,
        cx: &mut Context,
    ) -> Status {
        while let Some(index) = self.order.get(self.at).copied() {
            match node(&mut items[index]).tick(cx) {
                Status::Running => return Status::Running,
                Status::Success => return Status::Success,
                // Asked to stop, it doesn't start the next.
                Status::Failure if cx.interrupted() => break,
                Status::Failure => self.at += 1,
            }
        }
        Status::Failure
    }
}
