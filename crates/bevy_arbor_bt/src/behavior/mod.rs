//! What trees are built of: nodes, branches, decorators, and custom nodes.
//!
//! Build trees from [`Behavior`], its branches and the decorators on them; for
//! a node of your own, [`Behave`] and what it runs with.

mod behave;
mod branch;
mod child;
mod composite;
mod context;
mod leaf;
mod modifier;
mod status;
pub(crate) mod system;

use std::borrow::Cow;
use std::fmt::Debug;
use std::time::Duration;

use bevy::prelude::*;

pub use self::behave::Behave;
pub use self::branch::Branch;
pub use self::child::Child;
pub use self::composite::ScoredBranch;
use self::composite::{Fallback, Mode, Parallel, Random, Scored, Select, Sequence};
pub use self::context::Context;
use self::leaf::{Act, Spawn, Wait};
use self::modifier::{
    Commit, Forever, Invert, Repeat, RepeatWhile, Socket, Succeed, Timeout, WithState,
};
pub use self::modifier::{Plugged, plugged};
pub use self::status::{Outcome, Status};
use crate::behavior::system::Check;
use crate::decorator::IntoDecorator;

/// A part of a tree: built by the macro, these functions, or any `Behave`.
///
/// Built by [`behavior_tree!`](crate::behavior_tree), by these functions, or
/// from any [`Behave`].
pub struct Behavior {
    inner: Box<dyn Behave>,
    /// Whether it was running, and holding, as of its last tick.
    running: bool,
    held: bool,
}

impl<B: Behave> From<B> for Behavior {
    fn from(behavior: B) -> Self {
        Self {
            inner: Box::new(behavior),
            running: false,
            held: false,
        }
    }
}

impl Behavior {
    /// Runs `child` again each time it ends. Never ends itself. What ends is
    /// started again the same tick, at most once, so a child that ends every
    /// time can't hang the tick.
    #[must_use]
    pub fn forever(child: Behavior) -> Self {
        Forever::new(child).into()
    }

    /// Runs `child` `times` times, until it fails: succeeds once it has run
    /// them all, and fails at its first failure. What ends is started again
    /// the same tick, as in a `Sequence` of copies. `Repeat(times)` in the
    /// macro.
    #[must_use]
    pub fn repeat(times: u32, child: Behavior) -> Self {
        Repeat::new(times, child).into()
    }

    /// Runs the first of `branches` whose decorator passes, asking again each
    /// tick: a decorator passing cancels what's running below it, and one
    /// stopping cancels its own branch, unless what's running
    /// [holds](Context::hold). A branch without a decorator always passes.
    ///
    /// A branch that fails falls through to the next, and stays out until its
    /// decorator stops passing or the `Select` restarts, so it can't restart
    /// itself in a loop. A branch that succeeds ends the `Select`; none
    /// left to try fails it.
    #[must_use]
    pub fn select(branches: Vec<Branch>) -> Self {
        Select::new(branches).into()
    }

    /// Runs the first of `branches` whose decorator passes, asking each
    /// decorator only as it gets there: what's started isn't interrupted.
    #[must_use]
    pub fn fallback(branches: Vec<Branch>) -> Self {
        Fallback::new(branches).into()
    }

    /// Runs `children` in turn, until one fails.
    #[must_use]
    pub fn sequence(children: Vec<Behavior>) -> Self {
        Sequence::new(children).into()
    }

    /// `children` as one node: the only one, or a sequence. What the macro
    /// builds a single-child node's body with.
    #[doc(hidden)]
    #[must_use]
    pub fn group(mut children: Vec<Behavior>) -> Self {
        if children.len() == 1 {
            return children.remove(0);
        }
        Self::sequence(children)
    }

    /// Runs `children` together: fails at the first failure, canceling the
    /// rest, and succeeds once all have.
    #[must_use]
    pub fn all(children: Vec<Behavior>) -> Self {
        Parallel::new(Mode::All, children).into()
    }

    /// Runs `children` together: succeeds at the first success, canceling the
    /// rest, and fails once all have.
    #[must_use]
    pub fn any(children: Vec<Behavior>) -> Self {
        Parallel::new(Mode::Any, children).into()
    }

    /// Runs `children` together: the first to end, however it ends, settles it,
    /// canceling the rest.
    #[must_use]
    pub fn race(children: Vec<Behavior>) -> Self {
        Parallel::new(Mode::Race, children).into()
    }

    /// Tries `branches` from the highest score down, until one succeeds,
    /// scoring them once as it starts. A score system takes the agent, and may
    /// change things. A NaN score counts as no score: it goes last.
    #[must_use]
    pub fn scored(branches: Vec<ScoredBranch>) -> Self {
        Scored::new(branches).into()
    }

    /// Tries `children` in a random order, until one succeeds. `roll` is where
    /// the randomness comes from, so it's yours: a system taking the agent and
    /// returning a number from 0 up to 1.
    pub fn random<S, M>(roll: S, children: Vec<Behavior>) -> Self
    where
        S: IntoSystem<In<Entity>, f32, M> + Copy + Send + Sync + 'static,
    {
        Random::new(Check::new(roll), children).into()
    }

    /// `child`, its result reversed.
    #[must_use]
    pub fn invert(child: Behavior) -> Self {
        Invert::new(child).into()
    }

    /// `child`, then success whatever it did.
    #[must_use]
    pub fn succeed(child: Behavior) -> Self {
        Succeed::new(child).into()
    }

    /// `child`, not to be interrupted once started.
    ///
    /// Not even told it's wanted to stop, so a
    /// [`Graceful`](crate::task::Graceful) task in it doesn't wind down.
    /// Only force still stops it: a `Timeout`, an `All`, `Any` or `Race`
    /// settling, anything above being canceled, or the tree going.
    #[must_use]
    pub fn commit(child: Behavior) -> Self {
        Commit::new(child).into()
    }

    /// Runs `child` over and over while `decorator` passes, canceling it when
    /// it stops (unless it [holds](Context::hold)): succeeds then, and
    /// fails if `child` does. `RepeatWhile(decorator)` in the macro.
    pub fn repeat_while<M>(decorator: impl IntoDecorator<M>, child: Behavior) -> Self {
        RepeatWhile::new(decorator.into_decorator(), child).into()
    }

    /// Succeeds after `length`, by the clock the tree ticks on.
    #[must_use]
    pub fn wait(length: Duration) -> Self {
        Wait::new(length).into()
    }

    /// `child`, canceled and failed if it takes more than `length`, even if it
    /// holds.
    #[must_use]
    pub fn timeout(length: Duration, child: Behavior) -> Self {
        Timeout::new(length, child).into()
    }

    /// `child`, with `bundle` on the agent while it runs: inserted as it
    /// starts, removed as it ends or is canceled. `State(bundle)` in the
    /// macro.
    ///
    /// For systems that work only while a branch runs, as `With<Hunting>`. A
    /// branch of one task needn't: its task entity is already that.
    #[must_use]
    pub fn state<B: Bundle + Clone>(bundle: B, child: Behavior) -> Self {
        WithState::new(bundle, child).into()
    }

    /// What's plugged into the agent's socket `S`: its [`Plugged<S>`], built
    /// as it starts, and swapped as that is. `Socket(Weapon)` in the macro.
    ///
    /// Empty, it fails: `when(plugged::<S>)` keeps it out of the way until
    /// something's plugged in. Swapped while running, what it ran is cancelled,
    /// once it stops holding, and the new one starts; unplugged, it fails.
    pub fn socket<S: Send + Sync + 'static>(_socket: S) -> Self {
        Socket::<S>::new().into()
    }

    /// Spawns a task entity with `bundle`, and waits for it to report.
    ///
    /// The task is named after the bundle's type (its first component's, for a
    /// tuple), in its [`Name`] and the outline: `task(Chase)` in the macro. For
    /// a name of its own, [`task_named`](Self::task_named).
    pub fn task<B: Bundle + Clone>(bundle: B) -> Self {
        Spawn::new(leaf::bundle_name::<B>().into(), bundle).into()
    }

    /// Spawns a task entity named `name` with `bundle`, and waits for it to
    /// report: `task("Hunt down", Chase)` in the macro.
    pub fn task_named<B: Bundle + Clone>(name: impl Into<Cow<'static, str>>, bundle: B) -> Self {
        Spawn::new(name.into(), bundle).into()
    }

    /// Runs `system` once on the agent: what it returns is the result. It can
    /// return a `Result<bool>` to use `?`: an error fails, and goes to Bevy's
    /// error handler. Cached like a system condition, its `Local`s are every
    /// agent's.
    pub fn act<S, M>(system: S) -> Self
    where
        S: IntoSystem<In<Entity>, bool, M> + Copy + Send + Sync + 'static,
    {
        Act::new(Check::new(system)).into()
    }

    /// Runs `system` once on the agent and `param`: what it returns is the
    /// result.
    pub fn act_with<S, P, M>(system: S, param: P) -> Self
    where
        S: IntoSystem<In<(Entity, P)>, bool, M> + Copy + Send + Sync + 'static,
        P: Clone + Debug + Send + Sync + 'static,
    {
        Act::new(Check::with(system, param)).into()
    }

    /// Runs it a tick.
    ///
    /// Once its tree has been taken away or replaced, by something it ran, it
    /// doesn't run, and fails.
    pub fn tick(&mut self, cx: &mut Context) -> Status {
        if cx.gone() {
            return Status::Failure;
        }
        let said = cx.said();
        if !self.running {
            self.inner.start(cx);
        }
        let (status, held) = cx.holding(|cx| self.inner.tick(cx));
        self.running = status == Status::Running;
        if !self.running {
            self.inner.end(cx, status.outcome());
        } else if cx.said() == said {
            // Running, and nothing in it said what it waits on.
            cx.poll();
        }
        self.held = held && self.running;
        if self.held {
            cx.hold();
        }
        status
    }

    /// Runs it a tick asking it to stop, for a node about to switch away from
    /// it while it [holds](Self::holds): it winds down, and ends or stops
    /// holding when it has, starting nothing new. What `Select` and
    /// `RepeatWhile` do.
    pub fn tick_interrupted(&mut self, cx: &mut Context) -> Status {
        cx.interrupting(|cx| self.tick(cx))
    }

    /// Keeps it holding through a tick it isn't ticked: a node that doesn't
    /// tick a running child every time calls this on the ticks it skips, or the
    /// child's hold lapses for them.
    pub fn skip(&self, cx: &mut Context) {
        if self.held {
            cx.hold();
        }
    }

    /// Stops it at once, if it's running, ready to start again.
    pub fn cancel(&mut self, cx: &mut Context) {
        self.held = false;
        if std::mem::take(&mut self.running) {
            self.inner.end(cx, None);
        }
    }

    /// Whether it's running, as of its last tick.
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Whether it asked, on its last tick, not to be stopped: see
    /// [`Context::hold`].
    #[must_use]
    pub fn holds(&self) -> bool {
        self.held
    }

    /// How it reads in a trace.
    pub(crate) fn label(&self) -> String {
        self.inner.label()
    }

    /// Its children, for a trace.
    pub(crate) fn children(&self) -> Vec<Child<'_>> {
        self.inner.children()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::prelude::*;
    use bevy_harness::TestApp;

    use crate::behavior::{Behave, Child, Context};
    use crate::prelude::*;
    use crate::testing::*;

    /// A custom node: what a user would write.
    struct Twice {
        done: bool,
        child: Behavior,
    }

    impl Behave for Twice {
        fn tick(&mut self, cx: &mut Context) -> Status {
            match self.child.tick(cx) {
                Status::Success if !self.done => {
                    self.done = true;
                    self.child.tick(cx)
                }
                status => status,
            }
        }

        fn start(&mut self, _cx: &mut Context) {
            self.done = false;
        }

        fn end(&mut self, cx: &mut Context, _outcome: Option<Outcome>) {
            self.child.cancel(cx);
        }
    }

    #[test]
    fn a_custom_node_goes_in_a_tree_like_any_other() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behavior_tree! {
                Sequence => {
                    @ Twice { done: false, child: Behavior::act(count) },
                }
            },
        );
        app.tick();
        assert_eq!(acts(&app), 2);
        assert_eq!(outcome(&app, npc), Some(true));
    }

    #[test]
    fn subtrees_and_lists_go_in_with_at() {
        fn part() -> Behavior {
            behavior_tree! { act(count) }
        }
        let mut app = app();
        let list = vec![part(), part()];
        let npc = agent(
            &mut app,
            behavior_tree! {
                Sequence => {
                    @ part(),
                    @[ list ],
                    Succeed => { @ part() },
                }
            },
        );
        app.tick();
        assert_eq!(acts(&app), 4);
        assert_eq!(outcome(&app, npc), Some(true));
    }

    /// A custom node over a child, with nothing about interrupts in it.
    struct Plain {
        child: Behavior,
    }

    impl Behave for Plain {
        fn tick(&mut self, cx: &mut Context) -> Status {
            self.child.tick(cx)
        }

        fn end(&mut self, cx: &mut Context, _outcome: Option<Outcome>) {
            self.child.cancel(cx);
        }
    }

    #[test]
    fn a_commit_holds_through_a_custom_node_and_up_a_nested_select() {
        let mut app = app();
        let inner = behavior_tree! { Select => { Commit => { task(Work) } } };
        let npc = agent(
            &mut app,
            behavior_tree! {
                Select => {
                    when(high) => task(Other),
                    @ Plain { child: inner },
                }
            },
        );
        app.tick();
        set(&mut app, npc, true, false);
        app.ticks(3);
        assert_eq!(app.count::<With<Work>>(), 1, "held");
        assert_eq!(app.count::<With<Other>>(), 0);
    }

    #[test]
    fn one_child_holding_does_not_leak_into_its_sibling() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behavior_tree! {
                All => {
                    Commit => { task(Work) },
                    Select => {
                        when(high) => task(Other),
                        Wait(100.0),
                    },
                }
            },
        );
        app.tick();
        set(&mut app, npc, true, false);
        app.tick();
        assert_eq!(
            app.count::<With<Other>>(),
            1,
            "the sibling was free to switch"
        );
        assert_eq!(app.count::<With<Work>>(), 1);
    }

    #[test]
    fn a_custom_node_shows_its_type_name_in_the_outline_unless_it_says_otherwise() {
        let mut app = app();
        let npc = app.spawn((
            TreeTrace::default(),
            BehaviorTree::new(behavior_tree! {
                @ Plain { child: Behavior::wait(Duration::from_secs(1)) }
            }),
        ));
        app.tick();
        let trace = app.world().get::<TreeTrace>(npc).unwrap().to_string();
        assert_eq!(trace.trim(), "> Plain", "{trace}");
    }

    /// A custom node ticking its child every other tick.
    struct EveryOther {
        now: bool,
        child: Behavior,
    }

    impl Behave for EveryOther {
        fn tick(&mut self, cx: &mut Context) -> Status {
            self.now = !self.now;
            if self.now {
                return self.child.tick(cx);
            }
            self.child.skip(cx);
            Status::Running
        }

        fn start(&mut self, _cx: &mut Context) {
            self.now = false;
        }

        fn end(&mut self, cx: &mut Context, _outcome: Option<Outcome>) {
            self.child.cancel(cx);
        }
    }

    #[test]
    fn a_skipped_child_keeps_holding() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behavior_tree! {
                Select => {
                    when(high) => task(Other),
                    @ EveryOther { now: false, child: behavior_tree! { Commit => { task(Work) } } },
                }
            },
        );
        app.tick();
        set(&mut app, npc, true, false);
        app.ticks(4);
        assert_eq!(
            app.count::<With<Work>>(),
            1,
            "held on ticks it was skipped too"
        );
        assert_eq!(app.count::<With<Other>>(), 0);
    }

    /// What a node was told, in order.
    #[derive(Resource, Default)]
    struct Told(Vec<&'static str>);

    /// A node recording its lifecycle, done on its second tick.
    struct Recorder {
        ticks: u32,
    }

    impl Behave for Recorder {
        fn start(&mut self, cx: &mut Context) {
            self.ticks = 0;
            cx.world_mut().resource_mut::<Told>().0.push("start");
        }

        fn tick(&mut self, cx: &mut Context) -> Status {
            self.ticks += 1;
            cx.world_mut().resource_mut::<Told>().0.push("tick");
            if self.ticks == 2 {
                Status::Success
            } else {
                Status::Running
            }
        }

        fn end(&mut self, cx: &mut Context, outcome: Option<Outcome>) {
            let told = match outcome {
                Some(Outcome::Success) => "succeeded",
                Some(Outcome::Failure) => "failed",
                None => "cancelled",
            };
            cx.world_mut().resource_mut::<Told>().0.push(told);
        }
    }

    fn told(app: &TestApp) -> &[&'static str] {
        &app.world().resource::<Told>().0
    }

    #[test]
    fn a_node_starts_ticks_and_ends_in_order() {
        let mut app = app();
        app.app_mut().init_resource::<Told>();
        agent(&mut app, behavior_tree! { @ Recorder { ticks: 0 } });
        app.ticks(3);
        assert_eq!(told(&app), ["start", "tick", "tick", "succeeded"]);
    }

    #[test]
    fn a_cancelled_node_is_ended_once_and_an_idle_one_not_at_all() {
        let mut app = app();
        app.app_mut().init_resource::<Told>();
        let npc = agent(
            &mut app,
            behavior_tree! {
                Select => {
                    when(high) => task(Work),
                    @ Recorder { ticks: 0 },
                }
            },
        );
        app.tick();
        set(&mut app, npc, true, false);
        app.tick();
        app.entity_mut(npc).remove::<BehaviorTree>();
        app.tick();
        assert_eq!(told(&app), ["start", "tick", "cancelled"]);
    }

    /// A node that says nothing it waits on, counting its ticks.
    struct Silent;

    impl Behave for Silent {
        fn tick(&mut self, cx: &mut Context) -> Status {
            cx.world_mut().resource_mut::<Acts>().0 += 1;
            Status::Running
        }

        fn end(&mut self, _cx: &mut Context, _outcome: Option<Outcome>) {}
    }

    #[test]
    fn a_node_saying_nothing_is_ticked_every_tick_beside_one_that_waits() {
        let mut app = app();
        agent(
            &mut app,
            behavior_tree! {
                All => {
                    Wait(100.0),
                    @ Silent,
                }
            },
        );
        app.ticks(4);
        assert_eq!(acts(&app), 4, "its sibling's timer didn't put it to sleep");
    }

    /// A custom node saying how it reads, and what's under it.
    struct Pair {
        first: Behavior,
        second: Behavior,
    }

    impl Behave for Pair {
        fn tick(&mut self, cx: &mut Context) -> Status {
            self.first.tick(cx)
        }

        fn end(&mut self, cx: &mut Context, _outcome: Option<Outcome>) {
            self.first.cancel(cx);
            self.second.cancel(cx);
        }

        fn label(&self) -> String {
            "Pair of two".to_owned()
        }

        fn children(&self) -> Vec<Child<'_>> {
            vec![Child::new(&self.first), Child::noted(&self.second, "spare")]
        }
    }

    #[test]
    fn a_custom_node_says_how_it_reads_and_what_is_under_it() {
        let mut app = app();
        let npc = app.spawn((
            TreeTrace::default(),
            BehaviorTree::new(behavior_tree! {
                @ Pair {
                    first: Behavior::wait(Duration::from_secs(1)),
                    second: Behavior::wait(Duration::from_secs(2)),
                }
            }),
        ));
        app.tick();
        let trace = app.world().get::<TreeTrace>(npc).unwrap().to_string();
        assert_eq!(
            trace, "> Pair of two\n>   Wait(1s)\n    spare\n      Wait(2s)\n",
            "{trace}"
        );
    }
}
