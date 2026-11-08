use super::Behavior;
use crate::behavior::{Child, Context, Status};
use crate::decorator::{Decorator, IntoDecorator};

/// A branch of a `Select` or `Fallback`: a node, and the decorator it runs
/// under.
pub struct Branch {
    decorator: Option<Decorator>,
    node: Behavior,
    /// That it failed, and stays out of a `Select` until its decorator stops
    /// passing or the `Select` restarts.
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
    pub fn always(node: Behavior) -> Self {
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

    /// Ticks its node, asked to stop if `interrupted`: its decorator is told as
    /// the node starts and ends.
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
    pub(in crate::behavior) fn holds(&self) -> bool {
        self.node.holds()
    }

    /// Its node, as a child of what it's a branch of: noted with its
    /// decorator, if it has one.
    pub(in crate::behavior) fn child(&self) -> Child<'_> {
        match &self.decorator {
            Some(decorator) => {
                let failed = if self.failed { " (failed)" } else { "" };
                Child::noted(&self.node, format!("when {}{failed}", decorator.label()))
            }
            None => Child::new(&self.node),
        }
    }
}
