#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
// The README is the crate's documentation. Its example needs `bt` and
// `perception`, so it's doctested only with them, through `ReadmeDoctests`.
#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

use bevy::app::plugin_group;

mod order;

/// Perception with avian2d: line of sight through colliders, and touch
/// from contacts.
#[cfg(feature = "avian2d")]
#[cfg_attr(docsrs, doc(cfg(feature = "avian2d")))]
#[doc(inline)]
pub use bevy_arbor_avian2d as avian2d;
/// Perception with avian3d: line of sight through colliders, and touch
/// from contacts.
#[cfg(feature = "avian3d")]
#[cfg_attr(docsrs, doc(cfg(feature = "avian3d")))]
#[doc(inline)]
pub use bevy_arbor_avian3d as avian3d;
/// Reactive behavior trees: trees that sleep until something they wait on
/// changes, with tasks as entities.
#[cfg(feature = "bt")]
#[cfg_attr(docsrs, doc(cfg(feature = "bt")))]
#[doc(inline)]
pub use bevy_arbor_bt as bt;
/// Environment queries: places and things generated round an agent,
/// filtered and scored, and the best picked.
#[cfg(feature = "eqs")]
#[cfg_attr(docsrs, doc(cfg(feature = "eqs")))]
#[doc(inline)]
pub use bevy_arbor_eqs as eqs;
/// Debug drawing: what agents perceive, how queries weighed their items,
/// and slots.
#[cfg(feature = "gizmos")]
#[cfg_attr(docsrs, doc(cfg(feature = "gizmos")))]
#[doc(inline)]
pub use bevy_arbor_gizmos as gizmos;
/// Navigation: where agents are asked to go and how it goes, a `MoveTo`
/// task, and path costs for queries, over any pathfinding.
#[cfg(feature = "nav")]
#[cfg_attr(docsrs, doc(cfg(feature = "nav")))]
#[doc(inline)]
pub use bevy_arbor_nav as nav;
/// Perception: what an agent senses, by each sense, written to it as
/// `Perceived<S>`.
#[cfg(feature = "perception")]
#[cfg_attr(docsrs, doc(cfg(feature = "perception")))]
#[doc(inline)]
pub use bevy_arbor_perception as perception;
/// Smart objects: slots on things in the world that agents find, claim
/// and use.
#[cfg(feature = "smart")]
#[cfg_attr(docsrs, doc(cfg(feature = "smart")))]
#[doc(inline)]
pub use bevy_arbor_smart as smart;

pub use self::order::{ArborSystems, ArborSystemsPlugin};

pub mod prelude {
    //! What using the enabled parts takes: `use bevy_arbor::prelude::*`.

    // A game is 2D or 3D: with both on, the two share names, so neither is
    // here, and they're used through `bevy_arbor::avian2d` and `avian3d`.
    #[cfg(all(feature = "avian2d", not(feature = "avian3d")))]
    #[doc(no_inline)]
    pub use bevy_arbor_avian2d::prelude::*;
    #[cfg(all(feature = "avian3d", not(feature = "avian2d")))]
    #[doc(no_inline)]
    pub use bevy_arbor_avian3d::prelude::*;
    #[cfg(feature = "bt")]
    #[doc(no_inline)]
    pub use bevy_arbor_bt::prelude::*;
    #[cfg(feature = "eqs")]
    #[doc(no_inline)]
    pub use bevy_arbor_eqs::prelude::*;
    #[cfg(feature = "gizmos")]
    #[doc(no_inline)]
    pub use bevy_arbor_gizmos::prelude::*;
    #[cfg(feature = "nav")]
    #[doc(no_inline)]
    pub use bevy_arbor_nav::prelude::*;
    #[cfg(feature = "perception")]
    #[doc(no_inline)]
    pub use bevy_arbor_perception::prelude::*;
    #[cfg(feature = "smart")]
    #[doc(no_inline)]
    pub use bevy_arbor_smart::prelude::*;

    #[doc(no_inline)]
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
        #[cfg(feature = "eqs")]
        eqs:::EqsPlugin,
        #[cfg(feature = "smart")]
        smart:::SmartPlugin,
        #[cfg(feature = "nav")]
        nav:::NavPlugin,
        :ArborSystemsPlugin,
        #[plugin_group]
        #[cfg(feature = "avian2d")]
        avian2d:::AvianPerceptionPlugins,
        #[plugin_group]
        #[cfg(feature = "avian3d")]
        avian3d:::AvianPerceptionPlugins,
    }
}

/// The README's example, compiled as a doctest with `bt` and `perception`.
#[cfg(all(doctest, feature = "bt", feature = "perception"))]
#[doc = include_str!("../README.md")]
pub struct ReadmeDoctests;

/// The book's examples, one item per chapter, compiled as doctests with every
/// feature enabled. The book isn't published with the crate, so this only
/// builds in the repository.
#[cfg(all(
    doctest,
    feature = "bt",
    feature = "perception",
    feature = "gizmos",
    feature = "avian2d",
    feature = "eqs",
    feature = "smart",
    feature = "nav"
))]
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
        Decorators: "decorators.md",
        Tasks: "tasks.md",
        Interruption: "interruption.md",
        Sleeping: "sleeping.md",
        LargeTrees: "large-trees.md",
        CustomNodes: "custom-nodes.md",
        Senses: "senses.md",
        SensorsMemory: "sensors-memory.md",
        OcclusionAttitude: "occlusion-attitude.md",
        CustomSenses: "custom-senses.md",
        Queries: "queries.md",
        RunningQueries: "running-queries.md",
        SmartObjects: "smart-objects.md",
        Navigation: "navigation.md",
        CompleteGuard: "complete-guard.md",
        Debugging: "debugging.md",
        Testing: "testing.md",
        Performance: "performance.md",
        Features: "features.md",
    }
}
