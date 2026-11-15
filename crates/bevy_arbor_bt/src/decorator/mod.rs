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
    AgentFn, And, Condition, ConditionParam, IntoCondition, Not, Or, Typed, not,
};
pub use self::cooldown::{Cooldown, Cooling, SharedCooldown, cooldown, cooldown_of};
pub use self::decorate::Decorate;
use crate::behavior::{Context, Outcome};

/// A branch's decorators, all of which must pass: its conditions, put
/// together with `!`, `&&` and `||`, and after commas, any that act as it
/// starts and ends.
///
/// The macro writes one from `when(has_target && !calm)`, or
/// `when(has_target, cooldown(8.0))`.
pub struct Decorator {
    parts: Vec<Box<dyn Decorate>>,
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
        let labels: Vec<String> = self.parts.iter().map(|part| part.label()).collect();
        labels.join(", ")
    }

    /// Asks it about the agent: whether every part passes, each asked only if
    /// those before it did. One that says nothing it waits on is asked again
    /// next tick.
    pub(crate) fn passes(&mut self, cx: &mut Context) -> bool {
        self.parts
            .iter_mut()
            .all(|part| asked(cx, |cx| part.passes(cx)))
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
            parts: vec![Box::new(self)],
        }
    }
}

/// What marks a condition's [`IntoDecorator`].
#[doc(hidden)]
pub struct Conditional<M>(std::marker::PhantomData<M>);

impl<M, C: IntoCondition<M>> IntoDecorator<Conditional<M>> for C {
    fn into_decorator(self) -> Decorator {
        Decorator {
            parts: vec![Box::new(Guard(self.into_condition()))],
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

    use bevy::prelude::*;
    use bevy_harness::TestApp;

    use crate::behavior::Context;
    use crate::decorator::{Condition, Decorate, IntoCondition};
    use crate::prelude::*;
    use crate::testing::*;

    #[derive(Component)]
    struct Level(u32);

    /// At least `least`: a typed condition taking a parameter, captured.
    fn at_least(least: u32) -> impl Condition {
        (move |level: &Level| level.0 >= least).into_condition()
    }

    fn with_flags(app: &mut TestApp, tree: Behavior, high: bool, low: bool) -> Entity {
        let npc = agent(app, tree);
        set(app, npc, high, low);
        npc
    }

    #[test]
    fn conditions_read_as_written() {
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
    fn a_condition_captures_what_varies() {
        let mut app = app();
        let npc = app.spawn((
            Level(3),
            BehaviorTree::new(behavior_tree! {
                Sequence => {
                    Select => { when(at_least(2)) => act(count) },
                    Select => { when(at_least(5)) => act(count) },
                }
            }),
        ));
        app.tick();
        assert_eq!(acts(&app), 1, "one function, two conditions");
        assert_eq!(outcome(&app, npc), Some(false));
    }

    #[test]
    fn a_decorator_built_by_hand_goes_in_too() {
        let mut app = app();
        let npc = app.spawn((
            Level(3),
            BehaviorTree::new(behavior_tree! {
                Select => {
                    when(Decorator::new(at_least(2).and(at_least(3))).and(cooldown(1.0))) => act(count),
                }
            }),
        ));
        app.tick();
        assert_eq!(outcome(&app, npc), Some(true));
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

    /// How many are in a branch.
    #[derive(Resource, Default)]
    struct Inside(i32);

    /// Counts its branch's runs in and out.
    struct Counted;

    impl Decorate for Counted {
        fn start(&mut self, cx: &mut Context) {
            cx.commands()
                .queue(|world: &mut World| world.resource_mut::<Inside>().0 += 1);
        }

        fn passes(&mut self, cx: &mut Context) -> bool {
            cx.wait_on_nothing();
            true
        }

        fn end(&mut self, cx: &mut Context, _outcome: Option<Outcome>) {
            cx.commands()
                .queue(|world: &mut World| world.resource_mut::<Inside>().0 -= 1);
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

    /// Takes the agent's tree away: an act, which may.
    fn remover(In(agent): In<Entity>, mut commands: Commands) -> bool {
        commands.entity(agent).remove::<BehaviorTree>();
        false
    }

    #[test]
    fn a_tree_taken_away_by_its_own_act_ends_each_decorator_once() {
        let mut app = app();
        app.app_mut().init_resource::<Inside>();
        agent(
            &mut app,
            behavior_tree! {
                Select => { when(Counted) => act(remover) }
            },
        );
        app.tick();
        assert_eq!(
            app.world().resource::<Inside>().0,
            0,
            "started, then ended once, not twice"
        );
    }

    struct Thrown;

    #[test]
    fn a_branch_never_run_after_its_tree_went_is_told_nothing() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behavior_tree! {
                Select => {
                    act(remover),
                    when(cooldown_of::<Thrown>(5.0)) => act(count),
                }
            },
        );
        app.tick();
        assert!(app.world().get::<BehaviorTree>(npc).is_none());
        assert!(
            app.world().get::<Cooling<Thrown>>(npc).is_none(),
            "no cooldown for what never happened"
        );
        assert_eq!(acts(&app), 0);
    }

    static ASKED: AtomicUsize = AtomicUsize::new(0);

    /// A condition of its own saying nothing it waits on.
    struct Silent;

    impl Condition for Silent {
        fn passes(&mut self, _cx: &mut Context) -> bool {
            ASKED.fetch_add(1, Ordering::Relaxed);
            false
        }
    }

    #[test]
    fn a_condition_saying_nothing_is_asked_every_tick_even_beside_one_that_waits() {
        let mut app = app();
        agent(
            &mut app,
            behavior_tree! {
                Select => {
                    when(low || Silent) => task(Work),
                    Wait(100.0),
                }
            },
        );
        app.ticks(4);
        assert_eq!(ASKED.load(Ordering::Relaxed), 4);
    }
}
