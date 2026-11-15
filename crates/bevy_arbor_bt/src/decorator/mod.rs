//! Decorators: what decides whether a branch may run.
//!
//! [`Condition`]s only answer: functions of the agent's components, or your
//! own, put together with `!`, `&&` and `||`. Decorators that act as their
//! branch starts and ends, as [`Cooldown`] or a claim, are [`Decorate`]s:
//! they go after a comma, never under `!` or `||`, so whenever their branch
//! runs, they passed, and they're told it started and ended.
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

mod condition;
mod cooldown;
mod decorate;

pub(crate) use self::condition::asked;
pub use self::condition::{
    AgentFn, And, Asked, Condition, ConditionParam, IntoCondition, Not, Or, Typed, not, with,
};
pub use self::cooldown::{Cooldown, Cooling, SharedCooldown, cooldown, cooldown_of};
pub use self::decorate::Decorate;
use crate::behavior::{Context, Outcome};
use crate::tree::{NODE, Owner};

/// A branch's decorators, all of which must pass: its conditions, put
/// together with `!`, `&&` and `||`, and after commas, any that act as it
/// starts and ends.
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
    /// Whether it acts as its branch starts and ends: then that may change its
    /// answer too.
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

    /// Whether it passes: as it last answered, if nothing it waits on came
    /// and it was asked the tree's tick before, or asked again.
    fn passes(&mut self, cx: &mut Context) -> bool {
        if self.id == NODE {
            self.id = cx.new_id();
        }
        let tick = cx.tick_count();
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
        let answer = cx.asking(self.id, |cx| asked(cx, |cx| decorate.passes(cx)));
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
            .map(|part| part.decorate.label())
            .collect();
        labels.join(", ")
    }

    /// Asks it about the agent: whether every part passes, each asked only if
    /// those before it did. One that says nothing it waits on is asked again
    /// next tick; one that's not asked, as one before it failed, isn't waited
    /// on, so its answer is forgotten.
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

    fn label(&self) -> String {
        self.0.label()
    }
}

/// What a [`Decorator`] can be made from: a condition, as
/// [`IntoCondition`] takes, a [`Decorate`], or a tuple of them, all of which
/// must pass.
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
    ($($part:ident $marker:ident),+) => {
        impl<$($part: IntoDecorator<$marker>, $marker),+> IntoDecorator<Many<($($marker,)+)>>
            for ($($part,)+)
        {
            fn into_decorator(self) -> Decorator {
                #[expect(non_snake_case, reason = "named after their types")]
                let ($($part,)+) = self;
                let mut parts = Vec::new();
                $(parts.extend($part.into_decorator().parts);)+
                Decorator { parts }
            }
        }
    };
}

many!(A MA);
many!(A MA, B MB);
many!(A MA, B MB, C MC);
many!(A MA, B MB, C MC, D MD);
many!(A MA, B MB, C MC, D MD, E ME);
many!(A MA, B MB, C MC, D MD, E ME, F MF);
many!(A MA, B MB, C MC, D MD, E ME, F MF, G MG);
many!(A MA, B MB, C MC, D MD, E ME, F MF, G MG, H MH);

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use bevy::ecs::error::{ErrorContext, FallbackErrorHandler};
    use bevy::prelude::*;
    use bevy_harness::TestApp;

    use crate::behavior::Context;
    use crate::decorator::{Condition, Decorate, IntoCondition, with};
    use crate::prelude::*;
    use crate::testing::*;

    fn at_least(In((agent, least)): In<(Entity, u32)>, levels: Query<&Level>) -> bool {
        levels.get(agent).is_ok_and(|level| level.0 >= least)
    }

    #[derive(Component)]
    struct Level(u32);

    fn with_flags(app: &mut TestApp, tree: Behavior, high: bool, low: bool) -> Entity {
        let npc = agent(app, tree);
        set(app, npc, high, low);
        npc
    }

    #[test]
    fn boolean_decorators_read_as_written() {
        let mut app = app();
        let tree = || {
            behavior_tree! {
                Select => {
                    when(high && !low) => act(count),
                }
            }
        };
        let yes = with_flags(&mut app, tree(), true, false);
        let no = with_flags(&mut app, tree(), true, true);
        app.tick();
        assert_eq!(outcome(&app, yes), Some(true));
        assert_eq!(outcome(&app, no), Some(false));
    }

    #[test]
    fn or_and_grouping() {
        let mut app = app();
        let tree = || {
            behavior_tree! {
                Select => {
                    when(!(high || low)) => act(count),
                }
            }
        };
        let yes = with_flags(&mut app, tree(), false, false);
        let no = with_flags(&mut app, tree(), false, true);
        app.tick();
        assert_eq!(outcome(&app, yes), Some(true));
        assert_eq!(outcome(&app, no), Some(false));
    }

    #[test]
    fn a_decorator_takes_a_parameter() {
        let mut app = app();
        let npc = app.spawn((
            Level(3),
            BehaviorTree::new(behavior_tree! {
                Sequence => {
                    Select => { when(with(at_least, 2)) => act(count) },
                    Select => { when(with(at_least, 5)) => act(count) },
                }
            }),
        ));
        app.tick();
        assert_eq!(acts(&app), 1, "one function, two decorators");
        assert_eq!(outcome(&app, npc), Some(false));
    }

    static ERRORS: AtomicUsize = AtomicUsize::new(0);

    fn count_errors(_: BevyError, _: ErrorContext) {
        ERRORS.fetch_add(1, Ordering::Relaxed);
    }

    fn broken(In(_): In<Entity>) -> Result<bool> {
        Err("broken".into())
    }

    #[expect(
        clippy::unnecessary_wraps,
        reason = "an act returning a `Result`, which is what's tested"
    )]
    fn fine(In(_): In<Entity>, mut acts: ResMut<Acts>) -> Result<bool> {
        acts.0 += 1;
        Ok(true)
    }

    #[test]
    fn a_decorator_or_act_can_use_question_marks_and_an_error_counts_as_no() {
        let mut app = app();
        app.app_mut()
            .insert_resource(FallbackErrorHandler(count_errors));
        let npc = agent(
            &mut app,
            behavior_tree! {
                Sequence => {
                    act(fine),
                    Select => {
                        when(broken) => act(count),
                        act(refuse),
                    },
                }
            },
        );
        app.tick();
        assert_eq!(acts(&app), 2, "`fine`, then `refuse`: `broken` said no");
        assert_eq!(outcome(&app, npc), Some(false));
        assert_eq!(ERRORS.load(Ordering::Relaxed), 1, "to Bevy's error handler");
    }

    #[test]
    fn a_decorator_built_by_hand_goes_in_too() {
        let mut app = app();
        let npc = app.spawn((
            Level(3),
            BehaviorTree::new(behavior_tree! {
                Select => {
                    when((with(at_least, 2).and(with(at_least, 3)), cooldown(1.0))) => act(count),
                }
            }),
        ));
        app.tick();
        assert_eq!(outcome(&app, npc), Some(true));
    }

    /// Says no the first time it's asked, by any agent, and yes after.
    fn asked_before(In(_): In<Entity>, mut asked: Local<bool>) -> bool {
        std::mem::replace(&mut *asked, true)
    }

    #[test]
    fn a_decorator_s_local_is_shared_by_every_agent_using_it() {
        let mut app = app();
        let tree = || behavior_tree! { Select => { when(asked_before) => act(count) } };
        agent(&mut app, tree());
        agent(&mut app, tree());
        app.tick();
        assert_eq!(acts(&app), 1, "the second saw the first's `Local`");
    }

    #[derive(Component)]
    struct Asked;

    fn noted(In(agent): In<Entity>, mut commands: Commands) -> bool {
        commands.entity(agent).insert(Asked);
        true
    }

    #[test]
    fn a_decorator_can_queue_commands_as_a_run_condition_can() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behavior_tree! { Select => { when(noted) => act(count) } },
        );
        app.tick();
        assert!(
            app.world().get::<Asked>(npc).is_some(),
            "applied as it returned"
        );
    }

    /// Whether another entity's `Level` is above 2, as a closure.
    fn above(other: Entity) -> impl Condition {
        move |cx: &mut Context| {
            cx.watch::<Level>(other);
            cx.world()
                .get::<Level>(other)
                .is_some_and(|level| level.0 > 2)
        }
    }

    #[test]
    fn a_closure_taking_the_context_is_a_condition_too() {
        let mut app = app();
        let other = app.spawn(Level(1));
        agent(
            &mut app,
            behavior_tree! {
                Select => {
                    when(above(other) && !low) => task(Work),
                    when(|_: &mut Context| true) => task(Other),
                }
            },
        );
        app.tick();
        assert_eq!(app.count::<With<Other>>(), 1);
        app.entity_mut(other).insert(Level(3));
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 1, "woken by the other's change");
    }

    #[test]
    fn a_typed_condition_can_capture_what_varies() {
        fn least(least: u32) -> impl Condition {
            (move |level: &Level| level.0 >= least).into_condition()
        }
        let mut app = app();
        let npc = app.spawn((
            Level(3),
            BehaviorTree::new(behavior_tree! {
                Sequence => {
                    Select => { when(least(2)) => act(count) },
                    Select => { when(least(5)) => act(count) },
                }
            }),
        ));
        app.tick();
        assert_eq!(acts(&app), 1);
        assert_eq!(outcome(&app, npc), Some(false));
    }

    /// Whether another entity's `Level` is above 2: a condition of its own,
    /// reading beyond the agent.
    struct Above(Entity);

    impl Condition for Above {
        fn passes(&mut self, cx: &mut Context) -> bool {
            cx.watch::<Level>(self.0);
            cx.world()
                .get::<Level>(self.0)
                .is_some_and(|level| level.0 > 2)
        }
    }

    #[test]
    fn a_condition_of_your_own_reads_beyond_the_agent_and_wakes_for_it() {
        let mut app = app();
        let other = app.spawn(Level(1));
        agent(
            &mut app,
            behavior_tree! {
                Select => {
                    when(Above(other) && !low) => task(Work),
                    task(Other),
                }
            },
        );
        app.tick();
        assert_eq!(app.count::<With<Other>>(), 1);
        app.entity_mut(other).insert(Level(3));
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 1, "woken by the other's change");
    }

    static SILENT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    /// A condition of its own saying nothing it waits on.
    struct Silent;

    impl Condition for Silent {
        fn passes(&mut self, _cx: &mut Context) -> bool {
            SILENT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            false
        }
    }

    fn typed_low(flags: &Flags) -> bool {
        flags.low
    }

    #[test]
    fn a_condition_saying_nothing_is_asked_every_tick_even_beside_one_that_waits() {
        let mut app = app();
        agent(
            &mut app,
            behavior_tree! {
                Select => {
                    when(typed_low || Silent) => task(Work),
                    Wait(100.0),
                }
            },
        );
        app.ticks(4);
        assert_eq!(SILENT.load(std::sync::atomic::Ordering::Relaxed), 4);
    }

    /// How many are in a branch.
    #[derive(Resource, Default)]
    struct Inside(i32);

    /// Counts its branch's runs in and out.
    struct Counted;

    impl Decorate for Counted {
        fn start(&mut self, cx: &mut Context) {
            cx.world_mut().resource_mut::<Inside>().0 += 1;
        }

        fn passes(&mut self, _cx: &mut Context) -> bool {
            true
        }

        fn end(&mut self, cx: &mut Context, _outcome: Option<Outcome>) {
            cx.world_mut().resource_mut::<Inside>().0 -= 1;
        }
    }

    #[test]
    fn a_decorator_after_a_comma_is_told_its_branch_starts_and_ends() {
        let mut app = app();
        app.app_mut().init_resource::<Inside>();
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
        assert_eq!(app.world().resource::<Inside>().0, 1, "started");
        set(&mut app, npc, true, false);
        app.tick();
        assert_eq!(app.world().resource::<Inside>().0, 0, "ended as cancelled");
        assert_eq!(app.count::<With<Other>>(), 1);
    }

    /// Takes the agent's tree away if it's low: a system condition queuing
    /// commands, as one may.
    fn remover(In(agent): In<Entity>, flags: Query<&Flags>, mut commands: Commands) -> bool {
        if flags.get(agent).is_ok_and(|flags| flags.low) {
            commands.entity(agent).remove::<BehaviorTree>();
        }
        true
    }

    #[test]
    fn a_tree_taken_away_by_its_own_condition_ends_each_decorator_once() {
        let mut app = app();
        app.app_mut().init_resource::<Inside>();
        let npc = agent(
            &mut app,
            behavior_tree! {
                Select => { when(remover, Counted) => task(Work) }
            },
        );
        app.tick();
        assert_eq!(app.world().resource::<Inside>().0, 1, "started");
        set(&mut app, npc, false, true);
        app.tick();
        assert_eq!(
            app.world().resource::<Inside>().0,
            0,
            "ended once, not twice"
        );
    }

    #[test]
    fn a_decorator_whose_condition_before_it_failed_is_neither_asked_nor_told() {
        let mut app = app();
        app.app_mut().init_resource::<Inside>();
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
        assert_eq!(app.world().resource::<Inside>().0, 0);
        assert_eq!(app.count::<With<Other>>(), 1);
    }

    struct Thrown;

    #[test]
    fn a_branch_never_run_after_its_tree_went_is_told_nothing() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behavior_tree! {
                Select => {
                    when(remover) => act(refuse),
                    when(cooldown_of::<Thrown>(5.0)) => act(count),
                }
            },
        );
        set(&mut app, npc, false, true);
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
