//! Guards: read-only systems asked about an agent.

mod check;

use std::ops::Not;

use bevy::ecs::system::ReadOnlySystem;
use bevy::prelude::*;

pub(crate) use self::check::Check;

/// A condition on an agent, asked every tick while its branch could matter.
///
/// The macro writes one from `when(has_target && !calm)` or
/// `when(within, 5.0)`.
pub struct Guard {
    name: String,
    test: Test,
}

enum Test {
    Check(Check<bool>),
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

/// What a [`Guard`] can be made from: a guard, or a read-only system on the
/// agent.
///
/// A read-only system taking the agent and returning `bool`, or a
/// `Result<bool>` for one using `?`: an error doesn't pass, and goes to Bevy's
/// error handler. Anything taking a guard takes one of these, so `has_target`
/// and `Guard::with(within, 5.0).or(has_target)` go in alike.
///
/// A system is cached by its type, so it can't capture anything; one that does
/// doesn't compile. What varies goes in as a parameter, with
/// [`Guard::with`]:
///
/// ```compile_fail
/// # use bevy::prelude::*;
/// # use bevy_arbor::prelude::*;
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

    /// Asks it about `agent`. One that can't run says no.
    pub(crate) fn passes(&mut self, world: &mut World, agent: Entity) -> bool {
        match &mut self.test {
            Test::Check(check) => check.run(world, agent).unwrap_or(false),
            Test::Not(guard) => !guard.passes(world, agent),
            Test::And(a, b) => a.passes(world, agent) && b.passes(world, agent),
            Test::Or(a, b) => a.passes(world, agent) || b.passes(world, agent),
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
}
