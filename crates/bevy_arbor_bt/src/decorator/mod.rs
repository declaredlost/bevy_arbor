//! Decorators: what decides whether a branch may run, and hears as it starts
//! and ends. Conditions on the agent's components or asked of a system,
//! [`Cooldown`], or your own [`Decorate`], put together with `!`, `&&` and
//! `||`.

mod cooldown;
mod decorate;

use std::ops::Not;

use bevy::ecs::system::ReadOnlySystem;
use bevy::prelude::*;
use bevy::utils::prelude::ShortName;

pub use self::cooldown::{Cooldown, Cooling, SharedCooldown, cooldown, cooldown_of};
pub use self::decorate::Decorate;
use crate::behavior::system::Check;
use crate::behavior::{Context, Outcome};

/// A decorator on a branch: one, or several put together with `!`, `&&` and
/// `||`. Asked again while the branch could matter, and told as it starts and
/// ends.
///
/// The macro writes one from `when(has_target && !calm)`,
/// `when(within, 5.0)`, or `when(has_target && cooldown(8.0))`.
pub struct Decorator {
    test: Test,
}

enum Test {
    Decorator(Box<dyn Decorate>),
    Not(Box<Decorator>),
    And(Box<Decorator>, Box<Decorator>),
    Or(Box<Decorator>, Box<Decorator>),
}

impl Not for Decorator {
    type Output = Self;

    /// Passes when this doesn't.
    fn not(self) -> Self {
        Self {
            test: Test::Not(Box::new(self)),
        }
    }
}

/// A typed condition: its function and what it reads put together.
struct Typed {
    /// Its function's type's name.
    name: &'static str,
    read: Box<dyn for<'c> Fn(&mut Context<'c>) -> bool + Send + Sync>,
}

impl Decorate for Typed {
    fn passes(&mut self, cx: &mut Context) -> bool {
        (self.read)(cx)
    }

    fn label(&self) -> String {
        ShortName(self.name).to_string()
    }
}

/// A system condition. It says nothing it waits on, as it can read anything:
/// it's asked every tick.
struct Asked(Check<bool>);

impl Decorate for Asked {
    fn passes(&mut self, cx: &mut Context) -> bool {
        self.0.run(cx).unwrap_or(false)
    }

    fn label(&self) -> String {
        self.0.name().to_owned()
    }
}

/// What a [`Decorator`] can be made from: a decorator, a [`Decorate`] of your
/// own, or a condition: a function of the agent's components, or a read-only
/// system on the agent.
///
/// A function of the agent's components is a typed condition: it takes `&T` or
/// `Option<&T>` of up to four components, and returns `bool`. It's asked again
/// only once one of them changes, comes or goes, so a tree under typed
/// conditions sleeps until then. One taking `&T` the agent hasn't got doesn't
/// pass.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_arbor_bt::prelude::*;
/// #[derive(Component)]
/// struct Target(Option<Entity>);
///
/// fn has_target(target: &Target) -> bool {
///     target.0.is_some()
/// }
///
/// let decorator = has_target.into_decorator();
/// ```
///
/// Anything else, a typed condition can't read: another entity, a resource,
/// time. For that, a read-only system taking the agent and returning `bool`, or
/// a `Result<bool>` for one using `?`: an error doesn't pass, and goes to
/// Bevy's error handler. It can read anything, so it's asked every tick, and
/// its tree never sleeps while it could matter. Better still, a system of your
/// own keeps what it reads on the agent, for a typed condition.
///
/// Anything taking a decorator takes any of these, so `has_target` and
/// `Decorator::with(within, 5.0).or(has_target)` go in alike.
///
/// A system condition:
///
/// Read-only as a Bevy run condition is: no `ResMut` or `&mut` in a query.
/// Like a run condition, it can still take `Commands`, which are applied as it
/// returns; what changes things belongs in an `act`.
///
/// A system is cached by its type, so every condition, tree and agent using it
/// shares one, `Local`s and change detection too: a `Local` sees what other
/// agents left in it, and `Changed` or `Added` filters what changed since it
/// was last asked about any agent, not this one. Keep what's per agent on the
/// agent. Being cached, it can't capture anything; one that does doesn't
/// compile. What varies goes in as a parameter, with [`Decorator::with`]:
///
/// ```compile_fail
/// # use bevy::prelude::*;
/// # use bevy_arbor_bt::prelude::*;
/// let reach = 5.0;
/// let decorator = (move |In(_): In<Entity>| reach > 1.0).into_decorator();
/// ```
pub trait IntoDecorator<M> {
    /// The decorator it is.
    fn into_decorator(self) -> Decorator;
}

impl IntoDecorator<()> for Decorator {
    fn into_decorator(self) -> Decorator {
        self
    }
}

impl<S, M> IntoDecorator<fn(M)> for S
where
    S: IntoSystem<In<Entity>, bool, M> + Copy + Send + Sync + 'static,
    S::System: ReadOnlySystem,
{
    fn into_decorator(self) -> Decorator {
        Decorator::new(Asked(Check::new(self)))
    }
}

/// What marks a [`Decorate`]r's [`IntoDecorator`].
#[doc(hidden)]
pub struct Decorated;

impl<D: Decorate> IntoDecorator<Decorated> for D {
    fn into_decorator(self) -> Decorator {
        Decorator::new(self)
    }
}

/// What a typed condition can take: `&T` or `Option<&T>` of a component on the
/// agent.
pub trait ConditionParam: 'static {
    /// What the condition's function is given.
    type Item<'a>;

    /// It, off `agent`: `None` if a component it needs isn't there.
    #[doc(hidden)]
    fn fetch(agent: EntityRef<'_>) -> Option<Self::Item<'_>>;

    /// Asks for the tree to be woken once it changes.
    #[doc(hidden)]
    fn watch(cx: &mut Context);
}

impl<T: Component> ConditionParam for &'static T {
    type Item<'a> = &'a T;

    fn fetch(agent: EntityRef<'_>) -> Option<&T> {
        agent.get::<T>()
    }

    fn watch(cx: &mut Context) {
        let agent = cx.agent();
        cx.watch::<T>(agent);
    }
}

impl<T: Component> ConditionParam for Option<&'static T> {
    type Item<'a> = Option<&'a T>;

    fn fetch(agent: EntityRef<'_>) -> Option<Option<&T>> {
        Some(agent.get::<T>())
    }

    fn watch(cx: &mut Context) {
        let agent = cx.agent();
        cx.watch::<T>(agent);
    }
}

/// A typed condition from a function of `params` components.
macro_rules! typed_condition {
    ($($param:ident),+) => {
        impl<F, $($param),+> IntoDecorator<fn($($param),+) -> bool> for F
        where
            F: Fn($($param),+) -> bool
                + for<'a> Fn($($param::Item<'a>),+) -> bool
                + Send
                + Sync
                + 'static,
            $($param: ConditionParam),+
        {
            fn into_decorator(self) -> Decorator {
                let read = move |cx: &mut Context| {
                    $($param::watch(cx);)+
                    let Ok(agent) = cx.world().get_entity(cx.agent()) else {
                        return false;
                    };
                    #[expect(non_snake_case, reason = "named after their types")]
                    let ($(Some($param),)+) = ($($param::fetch(agent),)+) else {
                        return false;
                    };
                    self($($param),+)
                };
                Decorator::new(Typed {
                    name: std::any::type_name::<F>(),
                    read: Box::new(read),
                })
            }
        }
    };
}

typed_condition!(A);
typed_condition!(A, B);
typed_condition!(A, B, C);
typed_condition!(A, B, C, D);

impl Decorator {
    /// A decorator of your own, as one.
    pub fn new(decorator: impl Decorate) -> Self {
        Self {
            test: Test::Decorator(Box::new(decorator)),
        }
    }

    /// A system condition taking the agent and `param`: as many conditions as
    /// you like from one function.
    pub fn with<S, P, M>(system: S, param: P) -> Self
    where
        S: IntoSystem<In<(Entity, P)>, bool, M> + Copy + Send + Sync + 'static,
        S::System: ReadOnlySystem,
        P: Clone + std::fmt::Debug + Send + Sync + 'static,
    {
        Self::new(Asked(Check::with(system, param)))
    }

    /// Passes when this and `other` both do; `other` only asked if this does.
    #[must_use]
    pub fn and<M>(self, other: impl IntoDecorator<M>) -> Self {
        let other = other.into_decorator();
        Self {
            test: Test::And(Box::new(self), Box::new(other)),
        }
    }

    /// Passes when this or `other` does; `other` only asked if this doesn't.
    #[must_use]
    pub fn or<M>(self, other: impl IntoDecorator<M>) -> Self {
        let other = other.into_decorator();
        Self {
            test: Test::Or(Box::new(self), Box::new(other)),
        }
    }

    /// How it reads in a trace: its decorators' labels put together. Built
    /// only for a trace.
    pub(crate) fn label(&self) -> String {
        match &self.test {
            Test::Decorator(decorator) => decorator.label(),
            Test::Not(decorator) => format!("!{}", decorator.label()),
            Test::And(a, b) => format!("({} && {})", a.label(), b.label()),
            Test::Or(a, b) => format!("({} || {})", a.label(), b.label()),
        }
    }

    /// Asks it about the agent. A decorator that says nothing it waits on is
    /// asked again next tick.
    pub(crate) fn passes(&mut self, cx: &mut Context) -> bool {
        match &mut self.test {
            Test::Decorator(decorator) => {
                let said = cx.said();
                let passes = decorator.passes(cx);
                if cx.said() == said {
                    cx.poll();
                }
                passes
            }
            Test::Not(decorator) => !decorator.passes(cx),
            Test::And(a, b) => a.passes(cx) && b.passes(cx),
            Test::Or(a, b) => a.passes(cx) || b.passes(cx),
        }
    }

    /// Tells every decorator in it its branch is starting: those `&&` and `||`
    /// didn't ask too.
    pub(crate) fn start(&mut self, cx: &mut Context) {
        match &mut self.test {
            Test::Decorator(decorator) => decorator.start(cx),
            Test::Not(decorator) => decorator.start(cx),
            Test::And(a, b) | Test::Or(a, b) => {
                a.start(cx);
                b.start(cx);
            }
        }
    }

    /// Tells every decorator in it its branch has ended, with how it went.
    pub(crate) fn end(&mut self, cx: &mut Context, outcome: Option<Outcome>) {
        match &mut self.test {
            Test::Decorator(decorator) => decorator.end(cx, outcome),
            Test::Not(decorator) => decorator.end(cx, outcome),
            Test::And(a, b) | Test::Or(a, b) => {
                a.end(cx, outcome);
                b.end(cx, outcome);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use bevy::ecs::error::{ErrorContext, FallbackErrorHandler};
    use bevy::prelude::*;
    use bevy_harness::TestApp;

    use crate::behavior::Context;
    use crate::decorator::Decorate;
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
                    Select => { when(at_least, 2) => act(count) },
                    Select => { when(at_least, 5) => act(count) },
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
                    when(Decorator::with(at_least, 2).and(Decorator::with(at_least, 3))) => act(count),
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

    #[test]
    fn a_decorator_is_told_its_branch_ended_even_if_or_did_not_ask_it() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behavior_tree! {
                Forever => {
                    Select => {
                        when(high || cooldown(1.0)) => act(count),
                        Wait(100.0),
                    }
                }
            },
        );
        set(&mut app, npc, true, false);
        app.tick();
        let high = acts(&app);
        assert!(high > 0, "high: cooldown not asked");
        set(&mut app, npc, false, false);
        app.ticks(3);
        assert_eq!(acts(&app), high, "but cooling all the same");
        app.secs(1.1);
        assert!(acts(&app) > high, "cooled");
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
    fn a_decorator_of_your_own_goes_in_a_decorator_and_is_told_start_and_end() {
        let mut app = app();
        app.app_mut().init_resource::<Inside>();
        let npc = agent(
            &mut app,
            behavior_tree! {
                Select => {
                    when(!high && Counted) => task(Work),
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
                Select => { when(remover && Counted) => task(Work) }
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
}
