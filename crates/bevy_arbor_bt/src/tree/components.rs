use std::fmt;
use std::sync::atomic::Ordering;
use std::time::Duration;

use bevy::ecs::change_detection::Tick;
use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;

use super::outline::TreeOutline;
use super::systems::Ticking;
use super::wake::Plan;
use crate::behaviour::{Behaviour, Context, Outcome};

/// An agent's behaviour tree: what it runs, and how it ended, once it has.
///
/// Taking it away, however that happens, cancels whatever it's running. If
/// something it runs takes it away or replaces it, nothing more of it runs.
///
/// Between ticks it sleeps, until what its last tick waited on comes: a guard
/// that has to be asked every tick, a timer, or a change to a component a
/// typed guard reads or to a task's report.
#[derive(Component)]
#[component(on_discard = discard)]
pub struct BehaviourTree {
    /// Out of here only while it ticks.
    pub(super) root: Option<Behaviour>,
    pub(super) outcome: Option<Outcome>,
    /// What its last tick waited on.
    pub(super) plan: Plan,
    /// Never ticked yet, or restarted: ticked whatever it waited on.
    pub(super) fresh: bool,
}

impl BehaviourTree {
    /// Runs `root` on the entity this goes on.
    #[must_use]
    pub fn new(root: Behaviour) -> Self {
        Self {
            root: Some(root),
            outcome: None,
            plan: Plan::default(),
            fresh: true,
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
        if self.outcome.take().is_some() {
            self.fresh = true;
        }
    }

    /// Whether it's to be ticked: still going, and woken, as of the tree's
    /// `clock` and the change tick `now`.
    pub(super) fn due(&self, world: &World, clock: Option<Duration>, now: Tick) -> bool {
        self.outcome.is_none() && (self.fresh || self.plan.due(world, clock, now))
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
/// over when it's taken away. One put on by a tree ticked before it, in the
/// same tick, already holds it.
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
///
/// One going as it ticks is cancelled once its tick stops, which this tells
/// it to.
fn discard(mut world: DeferredWorld, context: HookContext) {
    let agent = context.entity;
    let Some(mut tree) = world.get_mut::<BehaviourTree>(agent) else {
        return;
    };
    let Some(mut root) = tree.root.take() else {
        // Its root is only out while it ticks.
        if let Some(ticking) = world.get_resource::<Ticking>() {
            ticking.0.store(true, Ordering::Relaxed);
        }
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

    fn despawn_self(In(agent): In<Entity>, mut commands: Commands) -> bool {
        commands.entity(agent).despawn();
        true
    }

    fn replace_own_tree(In(agent): In<Entity>, mut commands: Commands) -> bool {
        let tree = Behaviour::task("Other", Other);
        commands.entity(agent).insert(BehaviourTree::new(tree));
        true
    }

    #[test]
    fn an_act_taking_its_own_tree_away_stops_the_rest_of_it() {
        let mut app = app();
        agent(
            &mut app,
            behaviour_tree! {
                Sequence => {
                    act(remove_own_tree),
                    act(count),
                    task("Work", Work),
                }
            },
        );
        app.tick();
        assert_eq!(acts(&app), 0, "nothing after it ran");
        assert_eq!(works_started(&app), 0);
    }

    #[test]
    fn an_act_despawning_its_agent_stops_the_rest_of_it() {
        let mut app = app();
        agent(
            &mut app,
            behaviour_tree! {
                Sequence => {
                    act(despawn_self),
                    act(count),
                    task("Work", Work),
                }
            },
        );
        app.tick();
        assert_eq!(acts(&app), 0, "nothing after it ran");
        assert_eq!(works_started(&app), 0);
    }

    #[test]
    fn an_act_replacing_its_tree_stops_the_old_one() {
        let mut app = app();
        agent(
            &mut app,
            behaviour_tree! {
                Sequence => {
                    act(replace_own_tree),
                    act(count),
                }
            },
        );
        app.tick();
        assert_eq!(acts(&app), 0, "the old one stopped");
        app.tick();
        assert_eq!(app.count::<With<Other>>(), 1, "the new one runs");
    }

    /// On an agent: the one its tree pauses.
    #[derive(Component)]
    struct Pauses(Entity);

    fn pause_other(In(agent): In<Entity>, pauses: Query<&Pauses>, mut commands: Commands) -> bool {
        if let Ok(other) = pauses.get(agent) {
            commands.entity(other.0).insert(TreePaused);
        }
        true
    }

    #[test]
    fn a_tree_paused_by_one_ticked_before_it_is_not_ticked() {
        let mut app = app();
        let tree = || behaviour_tree! { Sequence => { act(pause_other), act(count) } };
        let first = agent(&mut app, tree());
        let second = agent(&mut app, tree());
        app.entity_mut(first).insert(Pauses(second));
        app.entity_mut(second).insert(Pauses(first));
        app.tick();
        assert_eq!(acts(&app), 1, "whichever went first paused the other");
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
