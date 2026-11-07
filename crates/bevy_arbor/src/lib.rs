#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
// The README is the crate's documentation. Its example needs `bt`, so it's
// doctested only with it, through `ReadmeDoctests`.
#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

use bevy::app::plugin_group;

mod order;

pub use self::order::{ArborSystems, ArborSystemsPlugin};

/// Reactive behaviour trees: the tree on an agent, its plugin, and what it's
/// built of.
#[cfg(feature = "bt")]
#[cfg_attr(docsrs, doc(cfg(feature = "bt")))]
pub mod bt {
    // Listed by name, so the documentation shows what each part brings.
    pub use bevy_arbor_bt::tree::{
        BehaviourTree, TreeFinished, TreeOutline, TreePaused, TreeTrace,
    };
    pub use bevy_arbor_bt::{TreePlugin, TreeSystems, behaviour_tree};

    pub mod behaviour {
        //! What trees are built of: nodes, branches, guards, and what a node
        //! of your own implements.

        pub use bevy_arbor_bt::behaviour::{
            Behave, Behaviour, Branch, Context, Guard, GuardParam, IntoGuard, Outcome,
            ScoredBranch, Status,
        };
    }

    pub mod task {
        //! Task entities: what a tree spawns to do its work, and reads back.

        pub use bevy_arbor_bt::task::{AgentTasks, Graceful, Task, TaskEnded, TaskOf, Tasks};
    }
}

#[cfg(feature = "bt")]
#[cfg_attr(docsrs, doc(cfg(feature = "bt")))]
pub use bevy_arbor_bt::behaviour_tree;

/// Perception: what an agent senses, by each sense, written as
/// [`Perceived<S>`](perception::perceived::Perceived) on it; sensors; and the
/// stages and senses of your own that can join.
#[cfg(feature = "perception")]
#[cfg_attr(docsrs, doc(cfg(feature = "perception")))]
pub mod perception {
    // Listed by name, so the documentation shows what each part brings.
    pub use bevy_arbor_perception::{PerceptionPlugin, PerceptionSystems};

    pub mod sense {
        //! The senses: what agents sense with, what they sense from, and how a
        //! sense of your own joins them.

        pub use bevy_arbor_perception::sense::{
            Facing, Hearing, Listeners, Noise, Perceivable, Sense, SensePlugin, SensorOf, Sensors,
            Share, Shared, Sight, Touch, Touched,
        };
    }

    pub mod perceived {
        //! What an agent perceives and remembers by each sense, how it stands
        //! towards it, and what's triggered as that changes.

        pub use bevy_arbor_perception::perceived::{
            Attitude, Candidate, Candidates, Forgotten, Lost, Noticed, Perceived, Percept, Team,
            Trace,
        };
    }

    pub mod space {
        //! The space perception works in.

        pub use bevy_arbor_perception::space::Space;
    }

    #[cfg(feature = "gizmos")]
    #[cfg_attr(docsrs, doc(cfg(feature = "gizmos")))]
    pub mod gizmos {
        //! Drawing what agents perceive. Not in `ArborPlugins`, as it needs a
        //! renderer: add `PerceptionGizmosPlugin` where there is one.

        pub use bevy_arbor_perception::gizmos::{
            PerceptionGizmos, PerceptionGizmosPlugin, ShowPerception,
        };
    }
}

pub mod prelude {
    //! What using the enabled parts takes: `use bevy_arbor::prelude::*`.

    #[cfg(feature = "bt")]
    #[doc(hidden)]
    pub use bevy_arbor_bt::prelude::*;
    #[cfg(feature = "perception")]
    #[doc(hidden)]
    pub use bevy_arbor_perception::prelude::*;

    #[doc(hidden)]
    pub use crate::{ArborPlugins, ArborSystems};
}

plugin_group! {
    /// The plugin of every part its features enable.
    ///
    /// Each keeps its defaults; change one with
    /// `ArborPlugins.set(TreePlugin::new(Update))`.
    pub struct ArborPlugins {
        #[cfg(feature = "bt")]
        bt:::TreePlugin,
        #[cfg(feature = "perception")]
        perception:::PerceptionPlugin,
        :ArborSystemsPlugin,
    }
}

/// The README's example, compiled as a doctest with `bt` enabled.
#[cfg(all(doctest, feature = "bt"))]
#[doc = include_str!("../README.md")]
pub struct ReadmeDoctests;

/// The book's examples, one item per chapter, compiled as doctests with every
/// feature enabled. The book isn't published with the crate, so this only
/// builds in the repository.
#[cfg(all(doctest, feature = "bt", feature = "perception", feature = "gizmos"))]
mod book {
    macro_rules! chapters {
        ($($name:ident: $file:literal),* $(,)?) => {
            $(
                #[doc = include_str!(concat!("../../../book/src/", $file))]
                struct $name;
            )*
        };
    }

    chapters! {
        Introduction: "introduction.md",
        Installation: "installation.md",
        FirstAgent: "first-agent.md",
        Concepts: "concepts.md",
        Trees: "trees.md",
        Guards: "guards.md",
        Tasks: "tasks.md",
        Interruption: "interruption.md",
        Sleeping: "sleeping.md",
        CustomNodes: "custom-nodes.md",
        Perception: "perception.md",
        Sight: "sight.md",
        HearingTouchSharing: "hearing-touch-sharing.md",
        Sensors: "sensors.md",
        MemoryEvents: "memory-events.md",
        OcclusionAttitude: "occlusion-attitude.md",
        CustomSenses: "custom-senses.md",
        Gizmos: "gizmos.md",
        PerceptionAndTrees: "perception-and-trees.md",
        Testing: "testing.md",
        Features: "features.md",
        Performance: "performance.md",
    }
}
