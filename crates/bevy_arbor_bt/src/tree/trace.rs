use std::fmt;

use bevy::prelude::*;

use crate::behavior::Behavior;
use crate::inspect::{Inspect, Inspection};

/// Keeps its agent's tree as it last ticked: each node, its note, what's
/// running.
///
/// For an inspector, or a test. Each node's index is its id, the same tick to
/// tick while the tree keeps its shape; its parent comes before it. Written
/// out, it's the tree's outline, with `>` on what's running:
///
/// ```text
/// > Select
///     when has_target
///   Chase
/// >   Wander
/// ```
///
/// Kept each tick its tree ticks, and changed only when that's changed: it's
/// a string a node, so only for debugging.
#[derive(Component, Reflect, Debug, Clone, Default, PartialEq, Eq)]
#[reflect(Component, Debug, Default, PartialEq)]
pub struct BehaviorTreeTrace {
    nodes: Vec<TraceNode>,
}

impl BehaviorTreeTrace {
    /// `root`'s tree, as it is now.
    pub(crate) fn of(root: &Behavior) -> Self {
        Self {
            nodes: {
                let mut nodes = Vec::new();
                write_node(&mut nodes, None, None, root);
                nodes
            },
        }
    }

    /// Every node, the root first, each after its parent.
    #[must_use]
    pub fn nodes(&self) -> &[TraceNode] {
        &self.nodes
    }

    /// The node with id `id`, if there is one.
    #[must_use]
    pub fn get(&self, id: usize) -> Option<&TraceNode> {
        self.nodes.get(id)
    }

    /// The nodes running, with their ids: the root down to what's at work.
    pub fn running(&self) -> impl Iterator<Item = (usize, &TraceNode)> {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node.running)
    }
}

/// Writes `node` into `nodes`, under `parent`, noted with `above`, then what's
/// under it.
pub(crate) fn write_node(
    nodes: &mut Vec<TraceNode>,
    parent: Option<usize>,
    above: Option<String>,
    node: &Behavior,
) {
    let at = nodes.len();
    nodes.push(TraceNode {
        parent,
        label: String::new(),
        note: above,
        running: node.is_running(),
        held: node.is_holding(),
    });
    let mut inspection = Inspection::new(Some((&mut *nodes, at)));
    node.inspect(&mut inspection);
    let label = inspection.finish();
    nodes[at].label = label;
}

impl fmt::Display for BehaviorTreeTrace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // How deep each is written: a child with a note goes a step deeper,
        // its note in between.
        let mut depths = Vec::with_capacity(self.nodes.len());
        for node in &self.nodes {
            let noted = usize::from(node.note.is_some());
            let depth = node.parent.map_or(0, |parent| depths[parent] + 1 + noted);
            if let Some(note) = &node.note {
                writeln!(f, "  {:indent$}{note}", "", indent = (depth - 1) * 2)?;
            }
            let mark = if node.running { ">" } else { " " };
            writeln!(f, "{mark} {:indent$}{}", "", node.label, indent = depth * 2)?;
            depths.push(depth);
        }
        Ok(())
    }
}

/// One node of a [`BehaviorTreeTrace`]: its label, note and parent, and
/// whether it runs.
#[derive(Reflect, Debug, Clone, PartialEq, Eq)]
#[reflect(Debug, PartialEq)]
pub struct TraceNode {
    pub(crate) parent: Option<usize>,
    pub(crate) label: String,
    pub(crate) note: Option<String>,
    pub(crate) running: bool,
    pub(crate) held: bool,
}

impl TraceNode {
    /// Its parent's id; `None` for the root.
    #[must_use]
    pub fn parent(&self) -> Option<usize> {
        self.parent
    }

    /// What it is: `Select`, `Wait(1s)`, a task's name.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// What's written above it, if anything: the decorator or score of the
    /// branch it is.
    #[must_use]
    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }

    /// Whether it's running.
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Whether it asked not to be stopped: see
    /// [`Context::hold`](crate::behavior::Context::hold).
    #[must_use]
    pub fn is_holding(&self) -> bool {
        self.held
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use bevy_harness::TestApp;

    use crate::behavior::{Behave, Context};
    use crate::prelude::*;
    use crate::testing::*;

    fn traced(app: &mut TestApp, tree: Behavior) -> Entity {
        app.spawn((
            Flags::default(),
            BehaviorTreeTrace::default(),
            BehaviorTree::new(tree),
        ))
    }

    fn trace(app: &TestApp, npc: Entity) -> &BehaviorTreeTrace {
        app.world().get::<BehaviorTreeTrace>(npc).unwrap()
    }

    #[test]
    fn it_has_each_node_under_its_parent() {
        let mut app = app();
        let npc = traced(
            &mut app,
            behavior_tree! {
                Select => {
                    when(high) => Sequence => { task(Work), Wait(1.0) },
                    task(Other),
                }
            },
        );
        app.tick();
        let nodes: Vec<_> = trace(&app, npc)
            .nodes()
            .iter()
            .map(|node| (node.parent(), node.label(), node.note().is_some()))
            .collect();
        assert_eq!(
            nodes,
            [
                (None, "Select", false),
                (Some(0), "Sequence", true),
                (Some(1), "Work", false),
                (Some(1), "Wait(1s)", false),
                (Some(0), "Other", false),
            ]
        );
        assert_eq!(trace(&app, npc).get(1).unwrap().note(), Some("when high"));
    }

    #[test]
    fn it_marks_what_is_running_and_what_holds() {
        let mut app = app();
        let npc = traced(
            &mut app,
            behavior_tree! {
                Select => {
                    when(high) => task(Other),
                    Commit => { task(Work) },
                }
            },
        );
        app.tick();
        let running: Vec<_> = trace(&app, npc)
            .running()
            .map(|(id, node)| (id, node.label(), node.is_holding()))
            .collect();
        assert_eq!(
            running,
            [(0, "Select", true), (2, "Commit", true), (3, "Work", false)]
        );
    }

    #[test]
    fn written_out_it_is_the_outline() {
        let mut app = app();
        let npc = traced(
            &mut app,
            behavior_tree! {
                Select => {
                    when(high) => task(Work),
                    task(Other),
                }
            },
        );
        app.tick();
        assert_eq!(
            trace(&app, npc).to_string(),
            "> Select\n    when high\n      Work\n>   Other\n"
        );
    }

    #[test]
    fn it_is_only_changed_when_the_tree_is() {
        let mut app = app();
        let npc = traced(
            &mut app,
            behavior_tree! {
                Select => {
                    when(high) => task(Work),
                    task(Other),
                }
            },
        );
        app.tick();
        let changed = |app: &TestApp| {
            app.world()
                .entity(npc)
                .get_ref::<BehaviorTreeTrace>()
                .unwrap()
                .last_changed()
        };
        let first = changed(&app);
        app.ticks(3);
        assert_eq!(changed(&app), first, "the same");
        set(&mut app, npc, true, false);
        app.tick();
        assert_ne!(changed(&app), first, "what's running changed");
    }

    /// Runs its first child, showing both, and how many times it ran.
    #[derive(Inspect)]
    struct Shown {
        #[inspect(param)]
        runs: u32,
        #[inspect(children)]
        children: Vec<Behavior>,
    }

    impl Behave for Shown {
        fn tick(&mut self, cx: &mut Context) -> Status {
            self.children[0].tick(cx)
        }

        fn end(&mut self, cx: &mut Context, _outcome: Option<Outcome>) {
            for child in &mut self.children {
                child.cancel(cx);
            }
        }
    }

    #[test]
    fn a_derived_inspect_shows_the_name_the_params_and_the_children() {
        let mut app = app();
        let npc = traced(
            &mut app,
            behavior_tree! {
                @ Shown {
                    runs: 2,
                    children: vec![Behavior::task(Work), Behavior::task(Other)],
                }
            },
        );
        app.tick();
        assert_eq!(
            trace(&app, npc).to_string(),
            "> Shown(2)\n>   Work\n    Other\n"
        );
    }
}
