//! The macro used where `bevy_arbor_bt` goes by another name.

use arbor::behavior::Behavior;

#[test]
fn it_builds_a_tree_through_a_renamed_dependency() {
    let tree: Behavior = arbor::behavior_tree! {
        Sequence => {
            Wait(0.5),
            Commit => { Wait(1.0) },
        }
    };
    let _ = tree;
}
