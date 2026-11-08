use bevy::ecs::schedule::InternedScheduleLabel;
use bevy::prelude::*;

/// Every part's systems, in each schedule they run in: perception, then trees,
/// then the environment queries trees asked for, then navigation's `MoveTo`
/// tasks, where they run in the same one.
///
/// Put what keeps facts on agents before it, and task systems after it.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArborSystems;

/// Puts every part's systems in [`ArborSystems`], in order, in each schedule
/// they were given. In [`ArborPlugins`](crate::ArborPlugins); add it yourself
/// if you add the parts' plugins one by one.
///
/// It looks at the parts' plugins once all plugins are added, so they can be
/// added in any order.
#[derive(Default)]
pub struct ArborSystemsPlugin;

impl Plugin for ArborSystemsPlugin {
    fn build(&self, _: &mut App) {}

    fn finish(&self, app: &mut App) {
        let perception = perception(app);
        trees(app, &perception);
        queries(app);
        navigation(app);
    }
}

/// Puts perception in [`ArborSystems`]: the schedules it runs in.
fn perception(app: &mut App) -> Vec<InternedScheduleLabel> {
    #[cfg(feature = "perception")]
    {
        use bevy_arbor_perception::{PerceptionPlugin, PerceptionSystems};

        let schedules: Vec<_> = app
            .get_added_plugins::<PerceptionPlugin>()
            .iter()
            .map(|plugin| plugin.schedule())
            .collect();
        for &schedule in &schedules {
            app.configure_sets(
                schedule,
                (
                    PerceptionSystems::Gather,
                    PerceptionSystems::Occlude,
                    PerceptionSystems::Classify,
                    PerceptionSystems::Commit,
                )
                    .in_set(ArborSystems),
            );
        }
        schedules
    }
    #[cfg(not(feature = "perception"))]
    {
        let _ = app;
        Vec::new()
    }
}

/// Puts trees in [`ArborSystems`], after perception where it runs in the same
/// schedule, one of `perception`.
fn trees(app: &mut App, perception: &[InternedScheduleLabel]) {
    #[cfg(feature = "bt")]
    {
        use bevy_arbor_bt::{TreePlugin, TreeSystems};

        let schedules: Vec<_> = app
            .get_added_plugins::<TreePlugin>()
            .iter()
            .map(|plugin| plugin.schedule())
            .collect();
        for schedule in schedules {
            app.configure_sets(schedule, TreeSystems.in_set(ArborSystems));
            #[cfg(feature = "perception")]
            if perception.contains(&schedule) {
                app.configure_sets(
                    schedule,
                    bevy_arbor_perception::PerceptionSystems::Commit.before(TreeSystems),
                );
            }
        }
    }
    let _ = (app, perception);
}

/// Puts environment queries in [`ArborSystems`]: after trees, as their plugin
/// says, so a query a tree asks for runs the same tick.
fn queries(app: &mut App) {
    #[cfg(feature = "eqs")]
    {
        use bevy_arbor_eqs::{EqsPlugin, EqsSystems};

        let schedules: Vec<_> = app
            .get_added_plugins::<EqsPlugin>()
            .iter()
            .map(|plugin| plugin.schedule())
            .collect();
        for schedule in schedules {
            app.configure_sets(schedule, EqsSystems.in_set(ArborSystems));
        }
    }
    let _ = app;
}

/// Puts navigation in [`ArborSystems`]: after trees and queries, as its
/// plugin says.
fn navigation(app: &mut App) {
    #[cfg(feature = "nav")]
    {
        use bevy_arbor_nav::{NavPlugin, NavSystems};

        let schedules: Vec<_> = app
            .get_added_plugins::<NavPlugin>()
            .iter()
            .map(|plugin| plugin.schedule())
            .collect();
        for schedule in schedules {
            app.configure_sets(schedule, NavSystems.in_set(ArborSystems));
        }
    }
    let _ = app;
}
