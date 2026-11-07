//! What trees are built of: nodes, branches, guards, and custom nodes.
//!
//! Build trees from [`Behaviour`], its branches and the guards on them; for a
//! node of your own, [`Behave`] and what it runs with.

mod act;
mod behave;
mod commit;
mod fallback;
mod forever;
mod guard;
mod invert;
mod ordered;
mod parallel;
mod random;
mod repeat_while;
mod scored;
mod select;
mod sequence;
mod spawn;
mod state;
mod succeed;
mod timeout;
mod wait;

use std::borrow::Cow;
use std::fmt::Debug;
use std::time::Duration;

use bevy::prelude::*;

use self::act::Act;
pub use self::behave::{Behave, Context, Outcome, Status};
use self::commit::Commit;
use self::fallback::Fallback;
use self::forever::Forever;
pub(crate) use self::guard::{Check, Systems};
pub use self::guard::{Guard, GuardParam, IntoGuard};
use self::invert::Invert;
use self::parallel::{Mode, Parallel};
use self::random::Random;
use self::repeat_while::RepeatWhile;
use self::scored::Scored;
pub use self::scored::ScoredBranch;
pub use self::select::Branch;
use self::select::Select;
use self::sequence::Sequence;
use self::spawn::Spawn;
use self::state::WithState;
use self::succeed::Succeed;
use self::timeout::Timeout;
use self::wait::Wait;
use crate::tree::TreeOutline;

/// A part of a tree: built by the macro, these functions, or any `Behave`.
///
/// Built by [`behaviour_tree!`](crate::behaviour_tree), by these functions, or
/// from any [`Behave`].
pub struct Behaviour {
    inner: Box<dyn Behave>,
    /// Whether it was running, and holding, as of its last tick.
    running: bool,
    held: bool,
}

impl<B: Behave> From<B> for Behaviour {
    fn from(behaviour: B) -> Self {
        Self {
            inner: Box::new(behaviour),
            running: false,
            held: false,
        }
    }
}

impl Behaviour {
    /// Runs `child` again each time it ends. Never ends itself. What ends is
    /// started again the same tick, at most once, so a child that ends every
    /// time can't hang the tick.
    #[must_use]
    pub fn forever(child: Behaviour) -> Self {
        Forever::new(child).into()
    }

    /// Runs the first of `branches` whose guard passes, asking again each tick:
    /// a guard passing cancels what's running below it, and one stopping
    /// cancels its own branch, unless what's running [holds](Context::hold). A
    /// branch without a guard always passes.
    ///
    /// A branch that fails falls through to the next, and stays out until its
    /// guard stops passing or the `Select` restarts, so it can't restart itself
    /// in a loop. A branch that succeeds ends the `Select`; none left to try
    /// fails it.
    #[must_use]
    pub fn select(branches: Vec<Branch>) -> Self {
        Select::new(branches).into()
    }

    /// Runs the first of `branches` whose guard passes, asking each guard only
    /// as it gets there: what's started isn't interrupted.
    #[must_use]
    pub fn fallback(branches: Vec<Branch>) -> Self {
        Fallback::new(branches).into()
    }

    /// Runs `children` in turn, until one fails.
    #[must_use]
    pub fn sequence(children: Vec<Behaviour>) -> Self {
        Sequence::new(children).into()
    }

    /// `children` as one node: the only one, or a sequence. What the macro
    /// builds a single-child node's body with.
    #[doc(hidden)]
    #[must_use]
    pub fn group(mut children: Vec<Behaviour>) -> Self {
        if children.len() == 1 {
            return children.remove(0);
        }
        Self::sequence(children)
    }

    /// Runs `children` together: fails at the first failure, cancelling the
    /// rest, and succeeds once all have.
    #[must_use]
    pub fn all(children: Vec<Behaviour>) -> Self {
        Parallel::new(Mode::All, children).into()
    }

    /// Runs `children` together: succeeds at the first success, cancelling the
    /// rest, and fails once all have.
    #[must_use]
    pub fn any(children: Vec<Behaviour>) -> Self {
        Parallel::new(Mode::Any, children).into()
    }

    /// Runs `children` together: the first to end, however it ends, settles it,
    /// cancelling the rest.
    #[must_use]
    pub fn race(children: Vec<Behaviour>) -> Self {
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
    pub fn random<S, M>(roll: S, children: Vec<Behaviour>) -> Self
    where
        S: IntoSystem<In<Entity>, f32, M> + Copy + Send + Sync + 'static,
    {
        Random::new(Check::new(roll), children).into()
    }

    /// `child`, its result reversed.
    #[must_use]
    pub fn invert(child: Behaviour) -> Self {
        Invert::new(child).into()
    }

    /// `child`, then success whatever it did.
    #[must_use]
    pub fn succeed(child: Behaviour) -> Self {
        Succeed::new(child).into()
    }

    /// `child`, not to be interrupted once started.
    ///
    /// Not even told it's wanted to stop, so a
    /// [`Graceful`](crate::task::Graceful) task in it doesn't wind down.
    /// Only force still stops it: a `Timeout`, an `All`, `Any` or `Race`
    /// settling, anything above being cancelled, or the tree going.
    #[must_use]
    pub fn commit(child: Behaviour) -> Self {
        Commit::new(child).into()
    }

    /// Runs `child` over and over while `guard` passes, cancelling it when it
    /// stops (unless it [holds](Context::hold)): succeeds then, and fails if
    /// `child` does. `While(guard)` in the macro.
    pub fn repeat_while<M>(guard: impl IntoGuard<M>, child: Behaviour) -> Self {
        RepeatWhile::new(guard.into_guard(), child).into()
    }

    /// Succeeds after `length`, by the clock the tree ticks on.
    #[must_use]
    pub fn wait(length: Duration) -> Self {
        Wait::new(length).into()
    }

    /// `child`, cancelled and failed if it takes more than `length`, even if it
    /// holds.
    #[must_use]
    pub fn timeout(length: Duration, child: Behaviour) -> Self {
        Timeout::new(length, child).into()
    }

    /// `child`, with `bundle` on the agent while it runs: inserted as it
    /// starts, removed as it ends or is cancelled. `State(bundle)` in the
    /// macro.
    ///
    /// For systems that work only while a branch runs, as `With<Hunting>`. A
    /// branch of one task needn't: its task entity is already that.
    #[must_use]
    pub fn state<B: Bundle + Clone>(bundle: B, child: Behaviour) -> Self {
        WithState::new(bundle, child).into()
    }

    /// Spawns a task entity named `name` with `bundle`, and waits for it to
    /// report.
    pub fn task<B: Bundle + Clone>(name: impl Into<Cow<'static, str>>, bundle: B) -> Self {
        Spawn::new(name.into(), bundle).into()
    }

    /// Runs `system` once on the agent: what it returns is the result. It can
    /// return a `Result<bool>` to use `?`: an error fails, and goes to Bevy's
    /// error handler. Cached like a guard, its `Local`s are every agent's.
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
        let (status, held) = cx.holding(|cx| self.inner.tick(cx));
        self.running = status == Status::Running;
        if self.running && self.inner.polls() {
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
    /// holding when it has, starting nothing new. What `Select` and `While` do.
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

    /// Stops it at once, ready to start again.
    pub fn cancel(&mut self, cx: &mut Context) {
        self.running = false;
        self.held = false;
        self.inner.cancel(cx);
    }

    /// Whether it asked, on its last tick, not to be stopped: see
    /// [`Context::hold`].
    #[must_use]
    pub fn holds(&self) -> bool {
        self.held
    }

    /// Writes it into `out`.
    pub(crate) fn outline(&self, out: &mut TreeOutline) {
        out.mark(self.running);
        self.inner.outline(out);
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::prelude::*;

    use crate::behaviour::{Behave, Context};
    use crate::prelude::*;
    use crate::testing::*;

    /// A custom node: what a user would write.
    struct Twice {
        done: bool,
        child: Behaviour,
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

        fn cancel(&mut self, cx: &mut Context) {
            self.done = false;
            self.child.cancel(cx);
        }
    }

    #[test]
    fn a_custom_node_goes_in_a_tree_like_any_other() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behaviour_tree! {
                Sequence => {
                    @ Twice { done: false, child: Behaviour::act(count) },
                }
            },
        );
        app.tick();
        assert_eq!(acts(&app), 2);
        assert_eq!(outcome(&app, npc), Some(true));
    }

    #[test]
    fn subtrees_and_lists_go_in_with_at() {
        fn part() -> Behaviour {
            behaviour_tree! { act(count) }
        }
        let mut app = app();
        let list = vec![part(), part()];
        let npc = agent(
            &mut app,
            behaviour_tree! {
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
        child: Behaviour,
    }

    impl Behave for Plain {
        fn tick(&mut self, cx: &mut Context) -> Status {
            self.child.tick(cx)
        }

        fn cancel(&mut self, cx: &mut Context) {
            self.child.cancel(cx);
        }
    }

    #[test]
    fn a_commit_holds_through_a_custom_node_and_up_a_nested_select() {
        let mut app = app();
        let inner = behaviour_tree! { Select => { Commit => { task("Work", Work) } } };
        let npc = agent(
            &mut app,
            behaviour_tree! {
                Select => {
                    when(high) => task("Other", Other),
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
            behaviour_tree! {
                All => {
                    Commit => { task("Work", Work) },
                    Select => {
                        when(high) => task("Other", Other),
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
            BehaviourTree::new(behaviour_tree! {
                @ Plain { child: Behaviour::wait(Duration::from_secs(1)) }
            }),
        ));
        app.tick();
        let trace = &app.world().get::<TreeTrace>(npc).unwrap().0;
        assert_eq!(trace.trim(), "> Plain", "{trace}");
    }

    /// A custom node ticking its child every other tick.
    struct EveryOther {
        now: bool,
        child: Behaviour,
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

        fn cancel(&mut self, cx: &mut Context) {
            self.now = false;
            self.child.cancel(cx);
        }
    }

    #[test]
    fn a_skipped_child_keeps_holding() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behaviour_tree! {
                Select => {
                    when(high) => task("Other", Other),
                    @ EveryOther { now: false, child: behaviour_tree! { Commit => { task("Work", Work) } } },
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
}
