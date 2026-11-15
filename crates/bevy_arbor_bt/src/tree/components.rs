use std::fmt;
use std::sync::atomic::Ordering;

use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;

use super::systems::Ticking;
use super::trace::BehaviorTreeTrace;
use super::wake::{NODE, Owner, Plan, Wakes};
use crate::behavior::{Behavior, Context, Outcome};

/// An agent's behavior tree: what it runs, and how it ended, once it has.
///
/// Taking it away, however that happens, cancels whatever it's running. If
/// something it runs takes it away or replaces it, nothing more of it runs.
///
/// Between ticks it sleeps, until what its last tick waited on comes: a
/// decorator that has to be asked every tick, a timer, or a change to a
/// component a typed condition reads or to a task's report.
#[derive(Component)]
#[component(on_insert = inserted, on_discard = discard)]
pub struct BehaviorTree {
    /// Out of here only while it ticks.
    pub(super) root: Option<Behavior>,
    pub(super) state: TreeState,
    /// The run of the trees it was last ticked in.
    pub(super) ran: u64,
    /// How many times it's been ticked.
    pub(super) ticks: u64,
    /// The last id given one of its decorator parts.
    pub(super) ids: Owner,
}

/// Where a tree is.
#[derive(Default)]
pub(super) enum TreeState {
    /// Never ticked yet, or restarted: ticked whatever it waited on.
    #[default]
    Fresh,
    /// Running, asleep until what its last tick waited on comes.
    Asleep(Plan),
    /// Ended, as it went.
    Finished(Outcome),
}

impl BehaviorTree {
    /// Runs `root` on the entity this goes on.
    #[must_use]
    pub fn new(root: Behavior) -> Self {
        Self {
            root: Some(root),
            state: TreeState::Fresh,
            ran: 0,
            ticks: 0,
            ids: NODE,
        }
    }

    /// How the tree ended, once it has. One under `Forever` never does.
    #[must_use]
    pub fn outcome(&self) -> Option<Outcome> {
        match self.state {
            TreeState::Finished(outcome) => Some(outcome),
            TreeState::Fresh | TreeState::Asleep(_) => None,
        }
    }

    /// Starts a finished tree again from the top, on its next tick.
    ///
    /// One still running is left as it is.
    pub fn restart(&mut self) {
        if let TreeState::Finished(_) = self.state {
            self.state = TreeState::Fresh;
        }
    }

    /// Whether it's yet to be ticked, new or restarted.
    pub(super) fn is_fresh(&self) -> bool {
        matches!(self.state, TreeState::Fresh)
    }
}

impl fmt::Display for BehaviorTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.root {
            Some(root) => BehaviorTreeTrace::of(root).fmt(f),
            None => Ok(()),
        }
    }
}

/// Holds an agent's tree where it is, until it's taken away again.
///
/// No decorators asked, nothing started, its task not ticked by
/// [`TaskQuery`](crate::task::TaskQuery). Time goes on, so a `Wait` or
/// `Timeout` may be over when it's taken away. One put on by a tree ticked
/// before it, in the same tick, already holds it.
#[derive(Component, Reflect, Debug, Clone, Copy, Default)]
#[reflect(Component, Debug, Default)]
pub struct BehaviorTreePaused;

/// Ticks a tree new on its agent next tick.
fn inserted(mut world: DeferredWorld, context: HookContext) {
    if let Some(mut wakes) = world.get_resource_mut::<Wakes>() {
        wakes.wake(context.entity);
    }
}

/// Looks at an unpaused tree next tick: it sees what changed while it was
/// paused.
pub(crate) fn unpaused(removed: On<Remove<BehaviorTreePaused>>, mut wakes: ResMut<Wakes>) {
    wakes.wake(removed.entity);
}

/// Cancels what a tree that's going is running, and forgets what it waited on.
///
/// One going as it ticks is canceled once its tick stops, which this tells
/// it to.
fn discard(mut world: DeferredWorld, context: HookContext) {
    let agent = context.entity;
    let Some(mut tree) = world.get_mut::<BehaviorTree>(agent) else {
        return;
    };
    let state = std::mem::take(&mut tree.state);
    let root = tree.root.take();
    if let TreeState::Asleep(plan) = &state
        && let Some(mut wakes) = world.get_resource_mut::<Wakes>()
    {
        wakes.forget(agent, plan);
    }
    let Some(mut root) = root else {
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

    use crate::prelude::*;
    use crate::testing::*;

    #[test]
    fn removing_the_tree_or_despawning_the_agent_cancels_its_task() {
        let mut app = app();
        let npc = agent(&mut app, behavior_tree! { task(Work) });
        app.tick();
        app.entity_mut(npc).remove::<BehaviorTree>();
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 0);

        let npc = agent(&mut app, behavior_tree! { task(Work) });
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 1);
        app.entity_mut(npc).despawn();
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 0);
    }

    #[test]
    fn replacing_the_tree_cancels_the_old_ones_task() {
        let mut app = app();
        let npc = agent(&mut app, behavior_tree! { task(Work) });
        app.tick();
        app.entity_mut(npc)
            .insert(BehaviorTree::new(behavior_tree! { task(Other) }));
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 0);
        assert_eq!(app.count::<With<Other>>(), 1);
    }

    fn remove_own_tree(In(agent): In<Entity>, mut commands: Commands) -> bool {
        commands.entity(agent).remove::<BehaviorTree>();
        true
    }

    #[test]
    fn an_act_taking_its_own_tree_away_leaves_no_task_behind() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behavior_tree! {
                All => {
                    task(Work),
                    act(remove_own_tree),
                }
            },
        );
        app.ticks(2);
        assert!(app.world().get::<BehaviorTree>(npc).is_none());
        assert_eq!(app.count::<With<Work>>(), 0);
    }

    fn despawn_self(In(agent): In<Entity>, mut commands: Commands) -> bool {
        commands.entity(agent).despawn();
        true
    }

    fn replace_own_tree(In(agent): In<Entity>, mut commands: Commands) -> bool {
        let tree = Behavior::task(Other);
        commands.entity(agent).insert(BehaviorTree::new(tree));
        true
    }

    #[test]
    fn an_act_taking_its_own_tree_away_stops_the_rest_of_it() {
        let mut app = app();
        agent(
            &mut app,
            behavior_tree! {
                Sequence => {
                    act(remove_own_tree),
                    act(count),
                    task(Work),
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
            behavior_tree! {
                Sequence => {
                    act(despawn_self),
                    act(count),
                    task(Work),
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
            behavior_tree! {
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
            commands.entity(other.0).insert(BehaviorTreePaused);
        }
        true
    }

    #[test]
    fn a_tree_paused_by_one_ticked_before_it_is_not_ticked() {
        let mut app = app();
        let tree = || behavior_tree! { Sequence => { act(pause_other), act(count) } };
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
            behavior_tree! {
                Select => {
                    when(high) => task(Work),
                    task(Other),
                }
            },
        );
        app.tick();
        app.entity_mut(npc).insert(BehaviorTreePaused);
        set(&mut app, npc, true, false);
        app.ticks(3);
        assert_eq!(app.count::<With<Other>>(), 1, "not canceled");
        assert_eq!(app.count::<With<Work>>(), 0, "not started");

        app.entity_mut(npc).remove::<BehaviorTreePaused>();
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 1);
    }

    #[test]
    fn tree_finished_is_triggered_once_on_the_agent() {
        #[derive(Resource, Default)]
        struct Seen(Vec<BehaviorTreeFinished>);
        let mut app = app();
        app.app_mut().init_resource::<Seen>();
        app.app_mut().add_observer(
            |finished: On<BehaviorTreeFinished>, mut seen: ResMut<Seen>| {
                seen.0.push(*finished);
            },
        );
        let npc = agent(&mut app, behavior_tree! { act(refuse) });
        app.ticks(3);
        let seen = &app.world().resource::<Seen>().0;
        assert_eq!(
            seen,
            &[BehaviorTreeFinished {
                entity: npc,
                outcome: Outcome::Failure
            }]
        );
    }

    #[test]
    fn a_finished_tree_restarts_from_the_top() {
        let mut app = app();
        let npc = agent(&mut app, behavior_tree! { act(count) });
        app.ticks(2);
        assert_eq!(acts(&app), 1, "a finished tree isn't ticked");
        app.world_mut()
            .get_mut::<BehaviorTree>(npc)
            .unwrap()
            .restart();
        assert_eq!(outcome(&app, npc), None);
        app.tick();
        assert_eq!(acts(&app), 2);
        assert_eq!(outcome(&app, npc), Some(true));
    }
}
