use std::fmt;

use bevy::prelude::*;

use crate::behavior::Behavior;

/// Keeps its agent's tree as it last ticked: each node, what's written above
/// it, and whether it's running.
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
pub struct TreeTrace {
    nodes: Vec<TraceNode>,
}

impl TreeTrace {
    /// `root`'s tree, as it is now.
    pub(crate) fn of(root: &Behavior) -> Self {
        let mut trace = Self { nodes: Vec::new() };
        trace.walk(root, None, None);
        trace
    }

    /// Adds `node` under `parent`, with `above` written above it, then its
    /// children.
    fn walk(&mut self, node: &Behavior, parent: Option<usize>, above: Option<String>) {
        let index = self.nodes.len();
        self.nodes.push(TraceNode {
            parent,
            label: node.label(),
            notes: above.into_iter().collect(),
            running: node.is_running(),
            held: node.holds(),
        });
        for child in node.children() {
            self.walk(child.node, Some(index), child.note);
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

impl fmt::Display for TreeTrace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // How deep each is written: a child with notes goes a step deeper,
        // its notes in between.
        let mut depths = Vec::with_capacity(self.nodes.len());
        for node in &self.nodes {
            let noted = usize::from(!node.notes.is_empty());
            let depth = node.parent.map_or(0, |parent| depths[parent] + 1 + noted);
            for note in &node.notes {
                writeln!(f, "  {:indent$}{note}", "", indent = (depth - 1) * 2)?;
            }
            let mark = if node.running { ">" } else { " " };
            writeln!(f, "{mark} {:indent$}{}", "", node.label, indent = depth * 2)?;
            depths.push(depth);
        }
        Ok(())
    }
}

/// One node of a [`TreeTrace`].
#[derive(Reflect, Debug, Clone, PartialEq, Eq)]
#[reflect(Debug, PartialEq)]
pub struct TraceNode {
    pub(crate) parent: Option<usize>,
    pub(crate) label: String,
    pub(crate) notes: Vec<String>,
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

    /// What's written above it: the decorator or score of the branch it is.
    #[must_use]
    pub fn notes(&self) -> &[String] {
        &self.notes
    }

    /// Whether it's running.
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Whether it asked not to be stopped: see
    /// [`Context::hold`](crate::behavior::Context::hold).
    #[must_use]
    pub fn holds(&self) -> bool {
        self.held
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use bevy_harness::TestApp;

    use crate::prelude::*;
    use crate::testing::*;

    fn traced(app: &mut TestApp, tree: Behavior) -> Entity {
        app.spawn((
            Flags::default(),
            TreeTrace::default(),
            BehaviorTree::new(tree),
        ))
    }

    fn trace(app: &TestApp, npc: Entity) -> &TreeTrace {
        app.world().get::<TreeTrace>(npc).unwrap()
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
            .map(|node| (node.parent(), node.label(), node.notes().len()))
            .collect();
        assert_eq!(
            nodes,
            [
                (None, "Select", 0),
                (Some(0), "Sequence", 1),
                (Some(1), "Work", 0),
                (Some(1), "Wait(1s)", 0),
                (Some(0), "Other", 0),
            ]
        );
        assert_eq!(trace(&app, npc).get(1).unwrap().notes(), ["when high"]);
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
            .map(|(id, node)| (id, node.label(), node.holds()))
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
                .get_ref::<TreeTrace>()
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
}
