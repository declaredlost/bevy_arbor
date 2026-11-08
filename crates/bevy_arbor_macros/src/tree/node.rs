//! The nodes the macro knows: their names, and how each is written.

/// A node the macro knows, by its name in a tree.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Node {
    Forever,
    Repeat,
    Select,
    Fallback,
    Sequence,
    All,
    Any,
    Race,
    Scored,
    Random,
    Invert,
    Succeed,
    Commit,
    RepeatWhile,
    Timeout,
    State,
    Socket,
    Wait,
    Task,
    Act,
}

impl Node {
    /// Every node, in the order an unknown name lists them.
    pub(super) const ALL: [Self; 20] = [
        Self::Forever,
        Self::Repeat,
        Self::Select,
        Self::Fallback,
        Self::Sequence,
        Self::All,
        Self::Any,
        Self::Race,
        Self::Scored,
        Self::Random,
        Self::Invert,
        Self::Succeed,
        Self::Commit,
        Self::RepeatWhile,
        Self::Timeout,
        Self::State,
        Self::Socket,
        Self::Wait,
        Self::Task,
        Self::Act,
    ];

    /// The node called `name`.
    pub(super) fn named(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|node| node.name() == name)
    }

    /// Its name in a tree.
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Forever => "Forever",
            Self::Repeat => "Repeat",
            Self::Select => "Select",
            Self::Fallback => "Fallback",
            Self::Sequence => "Sequence",
            Self::All => "All",
            Self::Any => "Any",
            Self::Race => "Race",
            Self::Scored => "Scored",
            Self::Random => "Random",
            Self::Invert => "Invert",
            Self::Succeed => "Succeed",
            Self::Commit => "Commit",
            Self::RepeatWhile => "RepeatWhile",
            Self::Timeout => "Timeout",
            Self::State => "State",
            Self::Socket => "Socket",
            Self::Wait => "Wait",
            Self::Task => "task",
            Self::Act => "act",
        }
    }

    /// What it takes in brackets, if anything: `length`.
    fn params(self) -> Option<&'static str> {
        match self {
            Self::Random => Some("roll"),
            Self::Repeat => Some("times"),
            Self::RepeatWhile => Some("decorator"),
            Self::Timeout | Self::Wait => Some("length"),
            Self::State | Self::Task => Some("bundle"),
            Self::Socket => Some("key"),
            Self::Act => Some("system"),
            _ => None,
        }
    }

    /// Whether it takes children.
    fn has_children(self) -> bool {
        !matches!(self, Self::Wait | Self::Socket | Self::Task | Self::Act)
    }

    /// How it's written, as an unknown name lists it: `Timeout(length)`.
    fn short(self) -> String {
        match self.params() {
            Some(params) => format!("{}({params})", self.name()),
            None => self.name().to_owned(),
        }
    }

    /// How it's written in full, as using it wrongly says:
    /// `` `Timeout(length) => { .. }` ``.
    pub(super) fn usage(self) -> String {
        match self {
            Self::Task => "`task(bundle)` or `task(name, bundle)`".to_owned(),
            Self::Act => "`act(system)` or `act(system, param)`".to_owned(),
            _ if self.has_children() => format!("`{} => {{ .. }}`", self.short()),
            _ => format!("`{}`", self.short()),
        }
    }

    /// What an unknown name gets: every node there is.
    pub(super) fn unknown(name: &str) -> String {
        let nodes: Vec<String> = Self::ALL.iter().map(|node| node.short()).collect();
        format!(
            "no node called `{name}`: there's {}, or @ expr",
            nodes.join(", ")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::Node;

    #[test]
    fn every_node_is_found_by_its_name() {
        for node in Node::ALL {
            assert_eq!(Node::named(node.name()), Some(node));
        }
    }

    #[test]
    fn an_unknown_name_says_what_there_is() {
        assert_eq!(
            Node::unknown("Parallel"),
            "no node called `Parallel`: there's Forever, Repeat(times), Select, Fallback, Sequence, All, Any, \
             Race, Scored, Random(roll), Invert, Succeed, Commit, RepeatWhile(decorator), \
             Timeout(length), State(bundle), Socket(key), Wait(length), task(bundle), act(system), or @ expr"
        );
    }
}
