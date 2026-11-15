//! Decorators: conditions and cooldowns, deciding whether a branch may run.
//!
//! [`Condition`]s only answer: functions of the agent's components, read-only
//! systems on the agent, or your own, put together with `!`, `&&` and `||`.
//! Decorators that act as their branch starts and ends, as [`Cooldown`] or a
//! claim, are [`Decorate`]s: they go after a comma, never under `!` or `||`, so
//! whenever their branch runs, they passed, and they're told it started and
//! ended.
//!
//! ```
//! # use bevy::prelude::*;
//! # use bevy_arbor_bt::prelude::*;
//! # #[derive(Component)] struct Target;
//! # #[derive(Component)] struct Calm;
//! # #[derive(Component, Clone)] struct Throw;
//! fn has_target(_: &Target) -> bool {
//!     true
//! }
//! fn calm(_: &Calm) -> bool {
//!     true
//! }
//!
//! let tree = behavior_tree! {
//!     Select => {
//!         when(has_target && !calm, cooldown(8.0)) => task(Throw),
//!     }
//! };
//! ```
//!
//! A cooldown under `!` or `||` doesn't compile: there, it may not have been
//! asked, or may not have passed, when its branch starts.
//!
//! ```compile_fail
//! # use bevy::prelude::*;
//! # use bevy_arbor_bt::prelude::*;
//! # #[derive(Component)] struct Target;
//! # #[derive(Component, Clone)] struct Throw;
//! fn has_target(_: &Target) -> bool { true }
//!
//! let tree = behavior_tree! {
//!     Select => { when(has_target || cooldown(8.0)) => task(Throw) }
//! };
//! ```

mod cooldown;
mod decorate;

use variadics_please::all_tuples;

pub use self::cooldown::{Cooldown, Cooling, SharedCooldown, cooldown, shared_cooldown};
pub use self::decorate::Decorate;
use crate::behavior::{Context, Outcome};
use crate::condition::{Condition, IntoCondition, poll_if_silent};
use crate::inspect::{Inspect, Inspection, label};
use crate::tree::{NODE, Owner};

/// A branch's decorators, all of which must pass: what's in its `when(..)`.
///
/// Its conditions, put together with `!`, `&&` and `||`, and after commas, any
/// that act as it starts and ends.
///
/// The macro writes one from `when(has_target && !calm)`, or
/// `when(has_target, cooldown(8.0))`.
pub struct Decorator {
    parts: Vec<Part>,
}

/// One part of a [`Decorator`], and what it last answered.
///
/// A part is asked again only once what it waits on comes: its answer can't
/// change before, so it stands, and what it waits on is carried over. That's
/// what Unreal calls observer aborts: a tree woken by one decorator's watch
/// asks that one again, not every decorator in it.
struct Part {
    decorate: Box<dyn Decorate>,
    /// Whether it acts as its branch starts and ends.
    ///
    /// Then that may change its answer too.
    scoped: bool,
    /// Its id in its tree, once it has one.
    id: Owner,
    /// What it last answered, unless that's not to be trusted.
    answer: Option<bool>,
    /// The tree's tick it last answered, or stood, in.
    checked: u64,
}

impl Part {
    fn new(decorate: Box<dyn Decorate>, scoped: bool) -> Self {
        Self {
            decorate,
            scoped,
            id: NODE,
            answer: None,
            checked: 0,
        }
    }

    /// Whether it passes: as it last answered, or asked again.
    ///
    /// It answers as it last did if nothing it waits on came and it was asked
    /// the tree's tick before.
    fn passes(&mut self, cx: &mut Context) -> bool {
        if self.id == NODE {
            self.id = cx.new_id();
        }
        let tick = cx.tick_count();
        // Asked already this tick: as it said then. What's changed since,
        // it's waiting on, and wakes the tree next tick.
        if let Some(answer) = self.answer
            && self.checked == tick
        {
            return answer;
        }
        // Asked the tick before, so what it waits on was waited on since.
        if let Some(answer) = self.answer
            && self.checked + 1 == tick
            && !cx.woken(self.id)
        {
            cx.keep(self.id);
            self.checked = tick;
            return answer;
        }
        let decorate = &mut self.decorate;
        let answer = cx.asking(self.id, |cx| poll_if_silent(cx, |cx| decorate.passes(cx)));
        self.answer = Some(answer);
        self.checked = tick;
        answer
    }

    /// Its branch starts: a scoped one's answer may change with it.
    fn start(&mut self, cx: &mut Context) {
        self.decorate.start(cx);
        if self.scoped {
            self.answer = None;
        }
    }

    /// Its branch ends: a scoped one's answer may change with it.
    fn end(&mut self, cx: &mut Context, outcome: Option<Outcome>) {
        self.decorate.end(cx, outcome);
        if self.scoped {
            self.answer = None;
        }
    }
}

impl Decorator {
    /// A decorator of `part`.
    pub fn new<M>(part: impl IntoDecorator<M>) -> Self {
        part.into_decorator()
    }

    /// This, and `other` too: both must pass.
    #[must_use]
    pub fn and<M>(mut self, other: impl IntoDecorator<M>) -> Self {
        self.parts.extend(other.into_decorator().parts);
        self
    }

    /// How it reads in a trace: its parts' labels. Built only for a trace.
    pub(crate) fn label(&self) -> String {
        let labels: Vec<String> = self
            .parts
            .iter()
            .map(|part| label(&*part.decorate))
            .collect();
        labels.join(", ")
    }

    /// Asks it about the agent: whether every part passes, each only if those
    /// before did.
    ///
    /// One that says nothing it waits on is asked again next tick; one that's
    /// not asked, as one before it failed, isn't waited on, so its answer is
    /// forgotten.
    pub(crate) fn passes(&mut self, cx: &mut Context) -> bool {
        let mut passes = true;
        for part in &mut self.parts {
            if passes {
                passes = part.passes(cx);
            } else {
                part.answer = None;
            }
        }
        passes
    }

    /// Tells every part its branch is starting: each passed, to let it in.
    pub(crate) fn start(&mut self, cx: &mut Context) {
        for part in &mut self.parts {
            part.start(cx);
        }
    }

    /// Tells every part its branch has ended, with how it went.
    pub(crate) fn end(&mut self, cx: &mut Context, outcome: Option<Outcome>) {
        for part in &mut self.parts {
            part.end(cx, outcome);
        }
    }
}

/// A condition, as a part of a [`Decorator`]: nothing to do as its branch
/// starts or ends.
struct Guard<C>(C);

impl<C: Condition> Decorate for Guard<C> {
    fn passes(&mut self, cx: &mut Context) -> bool {
        self.0.passes(cx)
    }
}

impl<C: Inspect> Inspect for Guard<C> {
    fn inspect(&self, inspection: &mut Inspection<'_>) {
        self.0.inspect(inspection);
    }
}

/// What a [`Decorator`] can be made from: conditions, decorators, or a tuple.
///
/// A condition, as [`IntoCondition`] takes, a [`Decorate`], or a tuple of
/// them, all of which must pass.
#[diagnostic::on_unimplemented(
    message = "`{Self}` isn't a decorator",
    label = "not a decorator",
    note = "a decorator is a condition (`fn(&Target) -> bool`, or a `Condition`), a `Decorate` such as a cooldown, or a tuple of them"
)]
pub trait IntoDecorator<M> {
    /// The decorator it is.
    fn into_decorator(self) -> Decorator;
}

impl IntoDecorator<()> for Decorator {
    fn into_decorator(self) -> Decorator {
        self
    }
}

/// What marks a [`Decorate`]'s [`IntoDecorator`].
#[doc(hidden)]
pub struct Scoped;

impl<D: Decorate> IntoDecorator<Scoped> for D {
    fn into_decorator(self) -> Decorator {
        Decorator {
            parts: vec![Part::new(Box::new(self), true)],
        }
    }
}

/// What marks a condition's [`IntoDecorator`].
#[doc(hidden)]
pub struct Conditional<M>(std::marker::PhantomData<M>);

impl<M, C: IntoCondition<M>> IntoDecorator<Conditional<M>> for C {
    fn into_decorator(self) -> Decorator {
        Decorator {
            parts: vec![Part::new(Box::new(Guard(self.into_condition())), false)],
        }
    }
}

/// What marks a tuple's [`IntoDecorator`].
#[doc(hidden)]
pub struct Many<M>(std::marker::PhantomData<M>);

/// [`IntoDecorator`] for a tuple of `parts`.
macro_rules! many {
    ($(($part:ident, $marker:ident)),*) => {
        impl<$($part: IntoDecorator<$marker>, $marker),*> IntoDecorator<Many<($($marker,)*)>>
            for ($($part,)*)
        {
            fn into_decorator(self) -> Decorator {
                let ($($part,)*) = self;
                let mut parts = Vec::new();
                $(parts.extend($part.into_decorator().parts);)*
                Decorator { parts }
            }
        }
    };
}

all_tuples!(many, 1, 8, P, M);

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use bevy::prelude::*;

    use crate::prelude::*;
    use crate::testing::*;

    #[test]
    fn a_decorator_after_a_comma_is_told_its_branch_starts_and_ends() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behavior_tree! {
                Select => {
                    when(!high, Counted) => task(Work),
                    task(Other),
                }
            },
        );
        app.tick();
        assert_eq!(inside(&app), 1, "started");
        set(&mut app, npc, true, false);
        app.tick();
        assert_eq!(inside(&app), 0, "ended as cancelled");
        assert_eq!(app.count::<With<Other>>(), 1);
    }

    /// Takes the agent's tree away if it's low: a system condition queuing
    /// commands.
    ///
    /// One may queue them.
    fn remover(In(agent): In<Entity>, flags: Query<&Flags>, mut commands: Commands) -> bool {
        if flags.get(agent).is_ok_and(|flags| flags.low) {
            commands.entity(agent).remove::<BehaviorTree>();
        }
        true
    }

    #[test]
    fn a_tree_taken_away_by_its_own_condition_ends_each_decorator_once() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behavior_tree! {
                Select => { when(remover, Counted) => task(Work) }
            },
        );
        app.tick();
        assert_eq!(inside(&app), 1, "started");
        set(&mut app, npc, false, true);
        app.tick();
        assert_eq!(inside(&app), 0, "ended once, not twice");
    }

    #[test]
    fn a_decorator_whose_condition_before_it_failed_is_neither_asked_nor_told() {
        let mut app = app();
        agent(
            &mut app,
            behavior_tree! {
                Select => {
                    when(high, Counted) => task(Work),
                    task(Other),
                }
            },
        );
        app.ticks(3);
        assert_eq!(inside(&app), 0);
        assert_eq!(app.count::<With<Other>>(), 1);
    }

    struct Thrown;

    #[test]
    fn a_branch_never_run_after_its_tree_went_is_told_nothing() {
        let mut app = app();
        let npc = with_flags(
            &mut app,
            behavior_tree! {
                Select => {
                    when(remover) => act(refuse),
                    when(shared_cooldown::<Thrown>(5.0)) => act(count),
                }
            },
            false,
            true,
        );
        app.tick();
        assert!(
            app.world().get::<Cooling<Thrown>>(npc).is_none(),
            "no cooldown for what never happened"
        );
    }

    /// Counts each time a typed condition below is asked.
    static ASKED_A: AtomicUsize = AtomicUsize::new(0);
    static ASKED_B: AtomicUsize = AtomicUsize::new(0);

    #[derive(Component)]
    struct A(bool);

    #[derive(Component)]
    struct B(bool);

    fn a(a: &A) -> bool {
        ASKED_A.fetch_add(1, Ordering::Relaxed);
        a.0
    }

    fn b(b: &B) -> bool {
        ASKED_B.fetch_add(1, Ordering::Relaxed);
        b.0
    }

    #[test]
    fn a_tree_woken_by_one_condition_asks_only_that_one_again() {
        let mut app = app();
        let npc = app.spawn((
            A(false),
            B(false),
            BehaviorTree::new(behavior_tree! {
                Select => {
                    when(a) => task(Work),
                    when(b) => task(Other),
                    Wait(100.0),
                }
            }),
        ));
        app.tick();
        let (before_a, before_b) = (
            ASKED_A.load(Ordering::Relaxed),
            ASKED_B.load(Ordering::Relaxed),
        );
        app.entity_mut(npc).insert(B(true));
        app.tick();
        assert_eq!(app.count::<With<Other>>(), 1, "switched to b's branch");
        assert_eq!(
            ASKED_A.load(Ordering::Relaxed),
            before_a,
            "a's answer stood"
        );
        assert_eq!(
            ASKED_B.load(Ordering::Relaxed),
            before_b + 1,
            "b asked again"
        );
    }

    static ASKED_C: AtomicUsize = AtomicUsize::new(0);

    #[derive(Component)]
    struct C(bool);

    fn c(c: &C) -> bool {
        ASKED_C.fetch_add(1, Ordering::Relaxed);
        c.0
    }

    #[test]
    fn a_task_reporting_asks_no_condition_again() {
        let mut app = app();
        app.spawn((
            C(true),
            BehaviorTree::new(behavior_tree! {
                Forever => {
                    Select => {
                        when(c) => Sequence => { task((Work, Report::Succeed)), Wait(100.0) },
                    }
                }
            }),
        ));
        app.tick();
        let before = ASKED_C.load(Ordering::Relaxed);
        app.ticks(2);
        assert_eq!(app.count::<With<Work>>(), 0, "reported, and the wait begun");
        assert_eq!(
            ASKED_C.load(Ordering::Relaxed),
            before,
            "woken by the task alone"
        );
    }

    #[derive(Component)]
    struct Low(bool);

    fn lower(low: &Low) -> bool {
        low.0
    }

    #[test]
    fn a_condition_not_asked_while_one_above_ran_answers_afresh() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behavior_tree! {
                Select => {
                    when(high) => task(Work),
                    when(lower) => task(Other),
                    Wait(100.0),
                }
            },
        );
        app.entity_mut(npc).insert(Low(false));
        set(&mut app, npc, true, false);
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 1);
        // Changed while it wasn't asked, nor watched.
        app.entity_mut(npc).insert(Low(true));
        app.tick();
        set(&mut app, npc, false, false);
        app.tick();
        assert_eq!(app.count::<With<Other>>(), 1, "asked again, not as it was");
    }

    #[test]
    fn a_part_after_one_that_failed_answers_afresh() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behavior_tree! {
                Select => {
                    when(high, lower) => task(Work),
                    Wait(100.0),
                }
            },
        );
        app.entity_mut(npc).insert(Low(false));
        set(&mut app, npc, true, false);
        app.tick();
        set(&mut app, npc, false, false);
        app.tick();
        // `lower` changes while `high` fails before it.
        app.entity_mut(npc).insert(Low(true));
        app.tick();
        set(&mut app, npc, true, false);
        app.tick();
        assert_eq!(
            app.count::<With<Work>>(),
            1,
            "lower asked again, not as it was"
        );
    }
}
