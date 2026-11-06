//! What `Scored` and `Random` share: trying their nodes in an order settled
//! as they start, until one succeeds.

use super::Behaviour;
use crate::behave::{Context, Status};

/// An order to try nodes in, and how far along it is.
#[derive(Default)]
pub(super) struct Order {
    order: Vec<usize>,
    at: usize,
}

impl Order {
    /// Whether it's settled: it is from the start until it ends.
    pub(super) fn started(&self) -> bool {
        !self.order.is_empty()
    }

    /// Settles it.
    pub(super) fn start(&mut self, order: Vec<usize>) {
        self.order = order;
        self.at = 0;
    }

    /// Ready to start again.
    pub(super) fn reset(&mut self) {
        self.order.clear();
        self.at = 0;
    }

    /// Ticks `items` in order, from where it got to, until one succeeds;
    /// `node` finds each one's node.
    pub(super) fn tick<T>(
        &mut self,
        items: &mut [T],
        node: fn(&mut T) -> &mut Behaviour,
        cx: &mut Context,
    ) -> Status {
        while let Some(index) = self.order.get(self.at).copied() {
            match node(&mut items[index]).tick(cx) {
                Status::Running => return Status::Running,
                Status::Success => {
                    self.reset();
                    return Status::Success;
                }
                Status::Failure => self.at += 1,
            }
        }
        self.reset();
        Status::Failure
    }
}
