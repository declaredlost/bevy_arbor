/// How a node's doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Status {
    /// Not done yet: ticked again next time.
    Running,
    /// Done, and it worked.
    Success,
    /// Done, and it didn't.
    Failure,
}

impl Status {
    /// How it ended, once it has.
    #[must_use]
    pub fn outcome(self) -> Option<Outcome> {
        match self {
            Self::Running => None,
            Self::Success => Some(Outcome::Success),
            Self::Failure => Some(Outcome::Failure),
        }
    }
}

/// How something ended: a node, a tree, or a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Outcome {
    /// It worked.
    Success,
    /// It didn't.
    Failure,
}

impl Outcome {
    /// Whether it worked.
    #[must_use]
    pub fn is_success(self) -> bool {
        self == Self::Success
    }
}

impl From<bool> for Outcome {
    /// [`Success`](Self::Success) for `true`, [`Failure`](Self::Failure)
    /// for `false`.
    fn from(success: bool) -> Self {
        if success {
            Self::Success
        } else {
            Self::Failure
        }
    }
}

impl From<Outcome> for Status {
    fn from(outcome: Outcome) -> Self {
        match outcome {
            Outcome::Success => Self::Success,
            Outcome::Failure => Self::Failure,
        }
    }
}
