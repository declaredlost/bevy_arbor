//! Guards: functions of the agent's components, or read-only systems asked
//! about an agent.

mod check;

use std::ops::Not;

use bevy::ecs::system::ReadOnlySystem;
use bevy::prelude::*;

pub(crate) use self::check::{Check, Systems};
use crate::behaviour::Context;
use crate::tree::TreeOutline;

/// A condition on an agent, asked again while its branch could matter.
///
/// The macro writes one from `when(has_target && !calm)` or
/// `when(within, 5.0)`.
pub struct Guard {
    name: String,
    test: Test,
}

/// A typed guard, its function and what it reads put together.
type Read = Box<dyn for<'c> Fn(&mut Context<'c>) -> bool + Send + Sync>;

enum Test {
    Check(Check<bool>),
    Read(Read),
    Not(Box<Guard>),
    And(Box<Guard>, Box<Guard>),
    Or(Box<Guard>, Box<Guard>),
}

impl Not for Guard {
    type Output = Self;

    /// Passes when this doesn't.
    fn not(self) -> Self {
        Self {
            name: format!("!{}", self.name),
            test: Test::Not(Box::new(self)),
        }
    }
}

/// What a [`Guard`] can be made from: a guard, a function of the agent's
/// components, or a read-only system on the agent.
///
/// A function of the agent's components is a typed guard: it takes `&T` or
/// `Option<&T>` of up to four components, and returns `bool`. It's asked again
/// only once one of them changes, comes or goes, so a tree under typed guards
/// sleeps until then. One taking `&T` the agent hasn't got doesn't pass.
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
/// let guard = has_target.into_guard();
/// ```
///
/// Anything else, a typed guard can't read: another entity, a resource, time.
/// For that, a read-only system taking the agent and returning `bool`, or a
/// `Result<bool>` for one using `?`: an error doesn't pass, and goes to Bevy's
/// error handler. It can read anything, so it's asked every tick, and its tree
/// never sleeps while it could matter. Better still, a system of your own
/// keeps what it reads on the agent, for a typed guard.
///
/// Anything taking a guard takes one of these, so `has_target` and
/// `Guard::with(within, 5.0).or(has_target)` go in alike.
///
/// A system guard:
///
/// Read-only as a Bevy run condition is: no `ResMut` or `&mut` in a query.
/// Like a run condition, it can still take `Commands`, which are applied as it
/// returns; what changes things belongs in an `act`.
///
/// A system is cached by its type, so every guard, tree and agent using it
/// shares one, `Local`s and change detection too: a `Local` sees what other
/// agents left in it, and `Changed` or `Added` filters what changed since it
/// was last asked about any agent, not this one. Keep what's per agent on the
/// agent. Being cached, it can't capture anything; one that does doesn't
/// compile. What varies goes in as a parameter, with [`Guard::with`]:
///
/// ```compile_fail
/// # use bevy::prelude::*;
/// # use bevy_arbor_bt::prelude::*;
/// let reach = 5.0;
/// let guard = (move |In(_): In<Entity>| reach > 1.0).into_guard();
/// ```
pub trait IntoGuard<M> {
    /// The guard it is.
    fn into_guard(self) -> Guard;
}

impl IntoGuard<()> for Guard {
    fn into_guard(self) -> Guard {
        self
    }
}

impl<S, M> IntoGuard<fn(M)> for S
where
    S: IntoSystem<In<Entity>, bool, M> + Copy + Send + Sync + 'static,
    S::System: ReadOnlySystem,
{
    fn into_guard(self) -> Guard {
        Guard::checking(Check::new(self))
    }
}

/// What a typed guard can take: `&T` or `Option<&T>` of a component on the
/// agent.
pub trait GuardParam: 'static {
    /// What the guard's function is given.
    type Item<'a>;

    /// It, off `agent`: `None` if a component it needs isn't there.
    #[doc(hidden)]
    fn fetch(agent: EntityRef<'_>) -> Option<Self::Item<'_>>;

    /// Asks for the tree to be woken once it changes.
    #[doc(hidden)]
    fn watch(cx: &mut Context);
}

impl<T: Component> GuardParam for &'static T {
    type Item<'a> = &'a T;

    fn fetch(agent: EntityRef<'_>) -> Option<&T> {
        agent.get::<T>()
    }

    fn watch(cx: &mut Context) {
        let agent = cx.agent();
        cx.watch::<T>(agent);
    }
}

impl<T: Component> GuardParam for Option<&'static T> {
    type Item<'a> = Option<&'a T>;

    fn fetch(agent: EntityRef<'_>) -> Option<Option<&T>> {
        Some(agent.get::<T>())
    }

    fn watch(cx: &mut Context) {
        let agent = cx.agent();
        cx.watch::<T>(agent);
    }
}

/// A typed guard from a function of `params` components.
macro_rules! typed_guard {
    ($($param:ident),+) => {
        impl<F, $($param),+> IntoGuard<fn($($param),+) -> bool> for F
        where
            F: Fn($($param),+) -> bool
                + for<'a> Fn($($param::Item<'a>),+) -> bool
                + Send
                + Sync
                + 'static,
            $($param: GuardParam),+
        {
            fn into_guard(self) -> Guard {
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
                Guard {
                    name: TreeOutline::short::<F>().to_owned(),
                    test: Test::Read(Box::new(read)),
                }
            }
        }
    };
}

typed_guard!(A);
typed_guard!(A, B);
typed_guard!(A, B, C);
typed_guard!(A, B, C, D);

impl Guard {
    /// A read-only system taking the agent and `param`: as many guards as you
    /// like from one function.
    pub fn with<S, P, M>(system: S, param: P) -> Self
    where
        S: IntoSystem<In<(Entity, P)>, bool, M> + Copy + Send + Sync + 'static,
        S::System: ReadOnlySystem,
        P: Clone + std::fmt::Debug + Send + Sync + 'static,
    {
        Self::checking(Check::with(system, param))
    }

    fn checking(check: Check<bool>) -> Self {
        Self {
            name: check.name().to_owned(),
            test: Test::Check(check),
        }
    }

    /// Passes when this and `other` both do; `other` only asked if this does.
    #[must_use]
    pub fn and<M>(self, other: impl IntoGuard<M>) -> Self {
        let other = other.into_guard();
        Self {
            name: format!("({} && {})", self.name, other.name),
            test: Test::And(Box::new(self), Box::new(other)),
        }
    }

    /// Passes when this or `other` does; `other` only asked if this doesn't.
    #[must_use]
    pub fn or<M>(self, other: impl IntoGuard<M>) -> Self {
        let other = other.into_guard();
        Self {
            name: format!("({} || {})", self.name, other.name),
            test: Test::Or(Box::new(self), Box::new(other)),
        }
    }

    /// What it's called in the outline.
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    /// Asks it about the agent. One that can't run says no.
    pub(crate) fn passes(&mut self, cx: &mut Context) -> bool {
        match &mut self.test {
            Test::Read(read) => read(cx),
            Test::Check(check) => {
                // A system can read anything, so it's asked every tick.
                cx.poll();
                check.run(cx).unwrap_or(false)
            }
            Test::Not(guard) => !guard.passes(cx),
            Test::And(a, b) => a.passes(cx) && b.passes(cx),
            Test::Or(a, b) => a.passes(cx) || b.passes(cx),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use bevy::ecs::error::{ErrorContext, FallbackErrorHandler};
    use bevy::prelude::*;
    use bevy_harness::TestApp;

    use crate::prelude::*;
    use crate::testing::*;

    fn at_least(In((agent, least)): In<(Entity, u32)>, levels: Query<&Level>) -> bool {
        levels.get(agent).is_ok_and(|level| level.0 >= least)
    }

    #[derive(Component)]
    struct Level(u32);

    fn with_flags(app: &mut TestApp, tree: Behaviour, high: bool, low: bool) -> Entity {
        let npc = agent(app, tree);
        set(app, npc, high, low);
        npc
    }

    #[test]
    fn boolean_guards_read_as_written() {
        let mut app = app();
        let tree = || {
            behaviour_tree! {
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
            behaviour_tree! {
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
    fn a_guard_takes_a_parameter() {
        let mut app = app();
        let npc = app.spawn((
            Level(3),
            BehaviourTree::new(behaviour_tree! {
                Sequence => {
                    Select => { when(at_least, 2) => act(count) },
                    Select => { when(at_least, 5) => act(count) },
                }
            }),
        ));
        app.tick();
        assert_eq!(acts(&app), 1, "one function, two guards");
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
    fn a_guard_or_act_can_use_question_marks_and_an_error_counts_as_no() {
        let mut app = app();
        app.app_mut()
            .insert_resource(FallbackErrorHandler(count_errors));
        let npc = agent(
            &mut app,
            behaviour_tree! {
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
    fn a_guard_built_by_hand_goes_in_too() {
        let mut app = app();
        let npc = app.spawn((
            Level(3),
            BehaviourTree::new(behaviour_tree! {
                Select => {
                    when(Guard::with(at_least, 2).and(Guard::with(at_least, 3))) => act(count),
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
    fn a_guard_s_local_is_shared_by_every_agent_using_it() {
        let mut app = app();
        let tree = || behaviour_tree! { Select => { when(asked_before) => act(count) } };
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
    fn a_guard_can_queue_commands_as_a_run_condition_can() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behaviour_tree! { Select => { when(noted) => act(count) } },
        );
        app.tick();
        assert!(
            app.world().get::<Asked>(npc).is_some(),
            "applied as it returned"
        );
    }
}
