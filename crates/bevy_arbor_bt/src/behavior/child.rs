use crate::behavior::Behavior;

/// One of a node's children, as
/// [`Behave::children`](crate::behavior::Behave::children) lists them: the
/// child, and what's written above it in a trace, if anything (a branch's
/// decorator, a score).
pub struct Child<'a> {
    pub(crate) node: &'a Behavior,
    pub(crate) note: Option<String>,
}

impl<'a> Child<'a> {
    /// `node`, with nothing above it.
    #[must_use]
    pub fn new(node: &'a Behavior) -> Self {
        Self { node, note: None }
    }

    /// `node`, with `above` written above it.
    #[must_use]
    pub fn noted(node: &'a Behavior, above: impl Into<String>) -> Self {
        Self {
            node,
            note: Some(above.into()),
        }
    }
}
