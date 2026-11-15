use super::Behavior;
use crate::behavior::{Context, Status};
use crate::decorator::{Decorator, IntoDecorator};
use crate::inspect::Inspection;

/// A branch of a `Select` or `Fallback`: a node, and the decorator it runs
/// under.
pub struct Branch {
    decorator: Option<Decorator>,
    node: Behavior,
    /// That it failed, and stays out of the `Select` it's in for the time
    /// being.
    ///
    /// It stays out until its decorator stops passing or the `Select` restarts.
    pub(in crate::behavior) failed: bool,
}

impl Branch {
    /// Runs `node` while `decorator` passes.
    pub fn when<M>(decorator: impl IntoDecorator<M>, node: Behavior) -> Self {
        Self {
            decorator: Some(decorator.into_decorator()),
            node,
            failed: false,
        }
    }

    /// Runs `node` unless a branch before it can.
    #[must_use]
    pub fn otherwise(node: Behavior) -> Self {
        Self {
            decorator: None,
            node,
            failed: false,
        }
    }

    /// Whether its decorator passes; one without a decorator always does.
    pub(in crate::behavior) fn passes(&mut self, cx: &mut Context) -> bool {
        match &mut self.decorator {
            Some(decorator) => decorator.passes(cx),
            None => true,
        }
    }

    /// Ticks its node, asked to stop if `interrupted`, and tells its decorator.
    ///
    /// The decorator is told as the node starts and ends.
    pub(in crate::behavior) fn tick(&mut self, cx: &mut Context, interrupted: bool) -> Status {
        // Its tree gone, nothing more of it runs, and nothing's told: the
        // tree's cancel ends what was running, once.
        if cx.gone() {
            return Status::Failure;
        }
        if !self.node.is_running()
            && let Some(decorator) = &mut self.decorator
        {
            decorator.start(cx);
        }
        let status = if interrupted {
            self.node.tick_interrupted(cx)
        } else {
            self.node.tick(cx)
        };
        if status != Status::Running
            && let Some(decorator) = &mut self.decorator
        {
            decorator.end(cx, status.outcome());
        }
        status
    }

    /// Cancels its node, if it's running: its decorator is told.
    pub(in crate::behavior) fn cancel(&mut self, cx: &mut Context) {
        if !self.node.is_running() {
            return;
        }
        self.node.cancel(cx);
        if let Some(decorator) = &mut self.decorator {
            decorator.end(cx, None);
        }
    }

    /// Whether its node holds.
    pub(in crate::behavior) fn is_holding(&self) -> bool {
        self.node.is_holding()
    }
}

impl Inspection<'_> {
    /// `branches`, under it: each noted with its decorator, if it has one.
    pub(crate) fn branches(&mut self, branches: &[Branch]) -> &mut Self {
        for branch in branches {
            match &branch.decorator {
                Some(decorator) => {
                    let failed = if branch.failed { " (failed)" } else { "" };
                    self.noted(
                        format_args!("when {}{failed}", decorator.label()),
                        &branch.node,
                    );
                }
                None => {
                    self.child(&branch.node);
                }
            }
        }
        self
    }
}
