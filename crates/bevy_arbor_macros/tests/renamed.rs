//! The macro used where `bevy_arbor` goes by another name.

use arbor::behaviour::Behaviour;

#[test]
fn it_builds_a_tree_through_a_renamed_dependency() {
    let tree: Behaviour = arbor::behaviour_tree! {
        Sequence => {
            Wait(0.5),
            Commit => { Wait(1.0) },
        }
    };
    let _ = tree;
}
