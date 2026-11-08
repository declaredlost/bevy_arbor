//! Sockets: a node running what's plugged into the agent.

mod plugged;

use std::marker::PhantomData;

use bevy::utils::prelude::ShortName;

pub use self::plugged::{Plugged, plugged};
use crate::behavior::{Behave, Behavior, Child, Context, Outcome, Status};

/// What's plugged into it on the agent, its [`Plugged<S>`]: swapped as that
/// is.
pub(in crate::behavior) struct Socket<S: Send + Sync + 'static> {
    /// What it's running, and what it was built from.
    child: Option<(Behavior, Plugged<S>)>,
    socket: PhantomData<fn() -> S>,
}

impl<S: Send + Sync + 'static> Socket<S> {
    pub(in crate::behavior) fn new() -> Self {
        Self {
            child: None,
            socket: PhantomData,
        }
    }

    /// What's plugged into it on the agent now.
    fn plugged(cx: &Context) -> Option<Plugged<S>> {
        cx.world().get::<Plugged<S>>(cx.agent()).cloned()
    }

    /// Whether what it runs was built from what's plugged in now.
    fn current(&self, plugged: Option<&Plugged<S>>) -> bool {
        match (&self.child, plugged) {
            (Some((_, built)), Some(plugged)) => built.same(plugged),
            (None, None) => true,
            _ => false,
        }
    }

    /// Builds what's plugged in, if anything is.
    fn build(&mut self, plugged: Option<Plugged<S>>) {
        self.child = plugged.map(|plugged| (plugged.build(), plugged));
    }
}

impl<S: Send + Sync + 'static> Behave for Socket<S> {
    fn start(&mut self, cx: &mut Context) {
        // What it built last run is kept, state and all, unless what's
        // plugged in has changed since.
        let plugged = Self::plugged(cx);
        if !self.current(plugged.as_ref()) {
            self.build(plugged);
        }
    }

    fn tick(&mut self, cx: &mut Context) -> Status {
        let agent = cx.agent();
        // Filling, swapping or emptying it wakes the tree.
        cx.watch::<Plugged<S>>(agent);
        let plugged = Self::plugged(cx);
        if !self.current(plugged.as_ref()) {
            let mut ended = None;
            if let Some((child, _)) = &mut self.child {
                // One holding goes on until it can stop, as in a `Select`.
                if child.holds() {
                    let status = child.tick_interrupted(cx);
                    if status == Status::Running && child.holds() {
                        return Status::Running;
                    }
                    ended = status.outcome().map(Status::from);
                }
                child.cancel(cx);
            }
            // Asked to stop, it starts nothing new: it ends as what ran did.
            if cx.interrupted() {
                return ended.unwrap_or(Status::Failure);
            }
            self.build(plugged);
        }
        match &mut self.child {
            Some((child, _)) => child.tick(cx),
            None => Status::Failure,
        }
    }

    fn end(&mut self, cx: &mut Context, _outcome: Option<Outcome>) {
        // Kept for its next run, unless what's plugged in changes meanwhile.
        if let Some((child, _)) = &mut self.child {
            child.cancel(cx);
        }
    }

    fn label(&self) -> String {
        format!("Socket({})", ShortName::of::<S>())
    }

    fn children(&self) -> Vec<Child<'_>> {
        self.child
            .iter()
            .map(|(child, _)| Child::new(child))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use bevy_harness::TestApp;

    use crate::prelude::*;
    use crate::testing::*;

    struct Weapon;

    fn work() -> Behavior {
        behavior_tree! { task(Work) }
    }

    fn other() -> Behavior {
        behavior_tree! { task(Other) }
    }

    fn committed() -> Behavior {
        behavior_tree! { Commit => { task(Work) } }
    }

    fn plug(app: &mut TestApp, npc: Entity, build: fn() -> Behavior) {
        app.entity_mut(npc).insert(Plugged::new(Weapon, build));
    }

    fn socketed(app: &mut TestApp) -> Entity {
        agent(
            app,
            behavior_tree! {
                Select => {
                    when(plugged::<Weapon>) => Socket(Weapon),
                    Wait(100.0),
                }
            },
        )
    }

    #[test]
    fn it_runs_what_is_plugged_in() {
        let mut app = app();
        let npc = socketed(&mut app);
        plug(&mut app, npc, work);
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 1);
    }

    #[test]
    fn an_empty_socket_is_passed_over_until_something_is_plugged_in() {
        let mut app = app();
        let npc = socketed(&mut app);
        app.ticks(2);
        assert_eq!(app.count::<With<Work>>(), 0, "empty");
        plug(&mut app, npc, work);
        app.tick();
        assert_eq!(
            app.count::<With<Work>>(),
            1,
            "plugged: the tree woke for it"
        );
    }

    #[test]
    fn an_empty_socket_reached_fails() {
        let mut app = app();
        let npc = agent(&mut app, behavior_tree! { Socket(Weapon) });
        app.tick();
        assert_eq!(outcome(&app, npc), Some(false));
    }

    #[test]
    fn swapping_it_while_running_starts_the_new_one() {
        let mut app = app();
        let npc = socketed(&mut app);
        plug(&mut app, npc, work);
        app.tick();
        plug(&mut app, npc, other);
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 0, "the old one cancelled");
        assert_eq!(app.count::<With<Other>>(), 1, "the new one started");
    }

    #[test]
    fn a_swap_waits_for_what_holds() {
        let mut app = app();
        let npc = socketed(&mut app);
        plug(&mut app, npc, committed);
        app.tick();
        plug(&mut app, npc, other);
        app.ticks(2);
        assert_eq!(app.count::<With<Work>>(), 1, "committed: still working");
        assert_eq!(app.count::<With<Other>>(), 0);
        let mut works = app.world_mut().query_filtered::<&mut Task, With<Work>>();
        works.single_mut(app.world_mut()).unwrap().succeed();
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 0);
        assert_eq!(app.count::<With<Other>>(), 1, "then swapped");
    }

    #[test]
    fn emptying_it_while_running_cancels_it() {
        let mut app = app();
        let npc = agent(&mut app, behavior_tree! { Socket(Weapon) });
        plug(&mut app, npc, work);
        app.tick();
        app.entity_mut(npc).remove::<Plugged<Weapon>>();
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 0);
        assert_eq!(outcome(&app, npc), Some(false));
    }

    #[test]
    fn the_same_subtree_inserted_again_is_not_a_swap() {
        let mut app = app();
        let npc = socketed(&mut app);
        plug(&mut app, npc, work);
        app.tick();
        let again = app.world().get::<Plugged<Weapon>>(npc).unwrap().clone();
        app.entity_mut(npc).insert(again);
        app.tick();
        assert_eq!(works_started(&app), 1, "still the first");
    }

    #[test]
    fn its_trace_shows_what_is_plugged_in() {
        let mut app = app();
        let npc = app.spawn((
            TreeTrace::default(),
            BehaviorTree::new(behavior_tree! { Socket(Weapon) }),
        ));
        plug(&mut app, npc, work);
        app.tick();
        let trace = app.world().get::<TreeTrace>(npc).unwrap().to_string();
        assert_eq!(trace, "> Socket(Weapon)\n>   Work\n");
    }

    fn cooling() -> Behavior {
        behavior_tree! {
            Select => {
                when(cooldown(5.0)) => act(count),
                Wait(0.0),
            }
        }
    }

    #[test]
    fn what_is_plugged_in_keeps_its_state_from_run_to_run() {
        let mut app = app();
        let npc = agent(&mut app, behavior_tree! { Forever => { Socket(Weapon) } });
        plug(&mut app, npc, cooling);
        app.ticks(3);
        assert_eq!(acts(&app), 1, "cooling across runs, not built afresh each");
    }

    /// A task only the new subtree starts.
    #[derive(Component, Clone)]
    struct Fresh;

    fn fresh() -> Behavior {
        behavior_tree! { task(Fresh) }
    }

    #[test]
    fn swapped_while_asked_to_stop_it_starts_nothing_new() {
        #[derive(Resource, Default)]
        struct Started(u32);
        let mut app = app();
        app.app_mut()
            .init_resource::<Started>()
            .add_observer(|_: On<Add<Fresh>>, mut started: ResMut<Started>| started.0 += 1);
        let npc = agent(
            &mut app,
            behavior_tree! {
                Select => {
                    when(high) => task(Other),
                    Socket(Weapon),
                }
            },
        );
        plug(&mut app, npc, committed);
        app.tick();
        set(&mut app, npc, true, false);
        plug(&mut app, npc, fresh);
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 1, "committed: still working");
        let mut works = app.world_mut().query_filtered::<&mut Task, With<Work>>();
        works.single_mut(app.world_mut()).unwrap().succeed();
        app.tick();
        assert_eq!(
            app.count::<With<Other>>(),
            1,
            "on to what wanted it stopped"
        );
        assert_eq!(
            app.world().resource::<Started>().0,
            0,
            "the new one never started"
        );
    }
}
