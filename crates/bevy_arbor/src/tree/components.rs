use std::fmt;

use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;

use super::outline::TreeOutline;
use crate::behaviour::{Behaviour, Context, Outcome};

/// An agent's behaviour tree: what it runs, and how it ended, once it has.
///
/// Taking it away, however that happens, cancels whatever it's running.
#[derive(Component)]
#[component(on_discard = discard)]
pub struct BehaviourTree {
    /// Out of here only while it ticks.
    pub(super) root: Option<Behaviour>,
    pub(super) outcome: Option<Outcome>,
}

impl BehaviourTree {
    /// Runs `root` on the entity this goes on.
    #[must_use]
    pub fn new(root: Behaviour) -> Self {
        Self {
            root: Some(root),
            outcome: None,
        }
    }

    /// How the tree ended, once it has. One under `Forever` never does.
    #[must_use]
    pub fn outcome(&self) -> Option<Outcome> {
        self.outcome
    }

    /// Starts a finished tree again from the top, on its next tick. One still
    /// running is left as it is.
    pub fn restart(&mut self) {
        self.outcome = None;
    }
}

impl fmt::Display for BehaviourTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.root {
            Some(root) => f.write_str(&TreeOutline::of(root)),
            None => Ok(()),
        }
    }
}

/// Holds an agent's tree where it is, until it's taken away again.
///
/// No guards asked, nothing started, its task not ticked by
/// [`Tasks`](crate::task::Tasks). Time goes on, so a `Wait` or `Timeout` may be
/// over when it's taken away.
#[derive(Component, Reflect, Debug, Clone, Copy, Default)]
#[reflect(Component, Debug, Default)]
pub struct TreePaused;

/// Keeps its agent's tree as a text outline, with `>` on what's running.
///
/// For an inspector. Costs a string a tick, so it's only for debugging.
#[derive(Component, Reflect, Debug, Clone, Default, PartialEq, Eq)]
#[reflect(Component, Debug, Default, PartialEq)]
pub struct TreeTrace(pub String);

/// Cancels what a tree that's going is running.
fn discard(mut world: DeferredWorld, context: HookContext) {
    let agent = context.entity;
    let Some(mut root) = world
        .get_mut::<BehaviourTree>(agent)
        .and_then(|mut tree| tree.root.take())
    else {
        return;
    };
    world.commands().queue(move |world: &mut World| {
        root.cancel(&mut Context::new(world, agent));
    });
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use bevy_harness::TestApp;

    use crate::prelude::*;
    use crate::testing::*;

    #[test]
    fn removing_the_tree_or_despawning_the_agent_cancels_its_task() {
        let mut app = app();
        let npc = agent(&mut app, behaviour_tree! { task("Work", Work) });
        app.tick();
        app.entity_mut(npc).remove::<BehaviourTree>();
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 0);

        let npc = agent(&mut app, behaviour_tree! { task("Work", Work) });
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 1);
        app.entity_mut(npc).despawn();
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 0);
    }

    #[test]
    fn replacing_the_tree_cancels_the_old_ones_task() {
        let mut app = app();
        let npc = agent(&mut app, behaviour_tree! { task("Work", Work) });
        app.tick();
        app.entity_mut(npc)
            .insert(BehaviourTree::new(behaviour_tree! { task("Other", Other) }));
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 0);
        assert_eq!(app.count::<With<Other>>(), 1);
    }

    fn remove_own_tree(In(agent): In<Entity>, mut commands: Commands) -> bool {
        commands.entity(agent).remove::<BehaviourTree>();
        true
    }

    #[test]
    fn an_act_taking_its_own_tree_away_leaves_no_task_behind() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behaviour_tree! {
                All => {
                    task("Work", Work),
                    act(remove_own_tree),
                }
            },
        );
        app.ticks(2);
        assert!(app.world().get::<BehaviourTree>(npc).is_none());
        assert_eq!(app.count::<With<Work>>(), 0);
    }

    #[test]
    fn a_paused_tree_stays_put_until_unpaused() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behaviour_tree! {
                Select => {
                    when(high) => task("Work", Work),
                    task("Other", Other),
                }
            },
        );
        app.tick();
        app.entity_mut(npc).insert(TreePaused);
        set(&mut app, npc, true, false);
        app.ticks(3);
        assert_eq!(app.count::<With<Other>>(), 1, "not cancelled");
        assert_eq!(app.count::<With<Work>>(), 0, "not started");

        app.entity_mut(npc).remove::<TreePaused>();
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 1);
    }

    #[test]
    fn tree_finished_is_triggered_once_on_the_agent() {
        #[derive(Resource, Default)]
        struct Seen(Vec<TreeFinished>);
        let mut app = app();
        app.app_mut().init_resource::<Seen>();
        app.app_mut()
            .add_observer(|finished: On<TreeFinished>, mut seen: ResMut<Seen>| {
                seen.0.push(*finished);
            });
        let npc = agent(&mut app, behaviour_tree! { act(refuse) });
        app.ticks(3);
        let seen = &app.world().resource::<Seen>().0;
        assert_eq!(
            seen,
            &[TreeFinished {
                entity: npc,
                outcome: Outcome::Failure
            }]
        );
    }

    #[test]
    fn a_trace_shows_what_is_running() {
        let mut app = app();
        let npc = app.spawn((
            Flags::default(),
            TreeTrace::default(),
            BehaviourTree::new(behaviour_tree! {
                Select => {
                    when(high) => task("Work", Work),
                    task("Other", Other),
                }
            }),
        ));
        app.tick();
        let trace = &app.world().get::<TreeTrace>(npc).unwrap().0;
        assert!(trace.contains("when high"), "{trace}");
        assert!(trace.contains(">   Other"), "{trace}");
        assert!(trace.contains("Work"), "{trace}");
        assert!(!trace.contains("> Work"), "{trace}");
    }

    #[test]
    fn a_trace_is_only_changed_when_what_is_running_is() {
        let mut app = app();
        let npc = app.spawn((
            Flags::default(),
            TreeTrace::default(),
            BehaviourTree::new(behaviour_tree! {
                Select => {
                    when(high) => task("Work", Work),
                    task("Other", Other),
                }
            }),
        ));
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
        assert_eq!(changed(&app), first, "the same outline");
        set(&mut app, npc, true, false);
        app.tick();
        assert_ne!(changed(&app), first, "what's running changed");
    }

    #[test]
    fn a_finished_tree_restarts_from_the_top() {
        let mut app = app();
        let npc = agent(&mut app, behaviour_tree! { act(count) });
        app.ticks(2);
        assert_eq!(acts(&app), 1, "a finished tree isn't ticked");
        app.world_mut()
            .get_mut::<BehaviourTree>(npc)
            .unwrap()
            .restart();
        assert_eq!(outcome(&app, npc), None);
        app.tick();
        assert_eq!(acts(&app), 2);
        assert_eq!(outcome(&app, npc), Some(true));
    }
}
