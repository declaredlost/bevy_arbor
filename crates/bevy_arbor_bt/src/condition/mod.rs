//! Conditions: whether a branch may run, read off the world, and nothing else.
//!
//! Typed conditions are functions of the agent's components, asked again once
//! one of them changes; system conditions, read-only systems on the agent,
//! asked every tick. Either, or one of your own, combines with `!`, `&&` and
//! `||` in `when(..)`.

mod logic;
mod system;
mod typed;

pub(crate) use self::logic::poll_if_silent;
pub use self::logic::{And, Not, Or, not};
pub use self::system::{ConditionSystem, with};
pub use self::typed::{AgentFn, ConditionParam, TypedCondition};
use crate::behavior::Context;
use crate::inspect::{Inspect, Inspection};

/// Whether a branch may run, read off the world, and nothing else: no start, no
/// end.
///
/// Conditions combine with `!`, `&&` and `||` into another one, in `when(..)`:
/// `when(has_target && !calm)`.
///
/// Most are functions of the agent's components, typed conditions, or
/// read-only systems on the agent, system conditions (see [`IntoCondition`]).
/// One of your own reads anything, through [`Context::world`]: another entity,
/// a resource, time. Say what you wait on as you're asked, with
/// [`Context::watch`] or [`Context::wake_at`]: one that says nothing is asked
/// again every tick. A closure taking the context is one too, for one that
/// needs no type of its own.
///
/// ```
/// use bevy::prelude::*;
/// use bevy_arbor_bt::behavior::Context;
/// use bevy_arbor_bt::condition::Condition;
/// use bevy_arbor_bt::inspect::Inspect;
///
/// #[derive(Component)]
/// struct Target(Entity);
///
/// #[derive(Component)]
/// struct Health(f32);
///
/// /// Whether the agent's target is still standing.
/// #[derive(Inspect)]
/// struct TargetStanding;
///
/// impl Condition for TargetStanding {
///     fn passes(&mut self, cx: &mut Context) -> bool {
///         let Some(target) =
///             cx.world().get::<Target>(cx.agent()).map(|t| t.0)
///         else {
///             cx.watch::<Target>(cx.agent());
///             return false;
///         };
///         cx.watch::<Target>(cx.agent());
///         cx.watch::<Health>(target);
///         cx.world().get::<Health>(target).is_some_and(|h| h.0 > 0.0)
///     }
/// }
/// ```
pub trait Condition: Inspect + Send + Sync + 'static {
    /// Whether its branch may run, asked before it starts and again while it
    /// runs.
    ///
    /// While it runs, a `Select` or `RepeatWhile` asks again once what it waits
    /// on comes.
    fn passes(&mut self, cx: &mut Context) -> bool;
}

/// What a [`Condition`] can be made from: a function, a system, or a condition.
///
/// A function of the agent's components, a Bevy
/// [`SystemCondition`](bevy::ecs::schedule::SystemCondition) taking the agent,
/// or a condition of your own.
///
/// A function of the agent's components is a typed condition: it takes `&T` or
/// `Option<&T>` of up to four components, and returns `bool`. It's asked again
/// only once one of them changes, comes or goes, so a tree under typed
/// conditions sleeps until then. One taking `&T` the agent hasn't got doesn't
/// pass. A closure is one too, so what varies is captured: a function making
/// one returns `impl Condition`, built where the closure's type is known.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_arbor_bt::condition::{Condition, IntoCondition};
/// #[derive(Component)]
/// struct Target(Option<Entity>);
///
/// #[derive(Component)]
/// struct Ammo(u32);
///
/// fn has_target(target: &Target) -> bool {
///     target.0.is_some()
/// }
///
/// /// At least `least` rounds left.
/// fn ammo(least: u32) -> impl Condition {
///     (move |ammo: &Ammo| ammo.0 >= least).into_condition()
/// }
///
/// let armed = has_target.and(ammo(3));
/// ```
///
/// Anything else, a typed condition can't read: another entity, a resource,
/// time. For that, a system condition: a Bevy
/// [`SystemCondition`](bevy::ecs::schedule::SystemCondition) taking the agent
/// as `In<Entity>`, returning `bool`, or a `Result<bool>` for one using `?`: an
/// error doesn't pass, and goes to Bevy's error handler. It can read anything,
/// so it's asked every tick, and its tree never sleeps while it could matter.
/// Better still, a system of your own keeps what it reads on the agent, for a
/// typed condition: what perception does.
///
/// Being a run condition, it's read-only: no `ResMut` or `&mut` in a query. It
/// can still take `Commands`, which are applied as it returns; what changes
/// things belongs in an `act`.
///
/// A system is cached by its type, so every condition, tree and agent using it
/// shares one, `Local`s and change detection too: a `Local` sees what other
/// agents left in it, and `Changed` or `Added` filters what changed since it
/// was last asked about any agent, not this one. Being cached, it can't
/// capture anything; one that does doesn't compile. What varies goes in as a
/// parameter, with [`with`]:
///
/// ```compile_fail
/// # use bevy::prelude::*;
/// # use bevy_arbor_bt::condition::IntoCondition;
/// let reach = 5.0;
/// let condition = (move |In(_): In<Entity>| reach > 1.0).into_condition();
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` isn't a condition",
    label = "not a condition",
    note = "a condition is a function of the agent's components (`fn(&Target) -> bool`), or a `Condition` of your own",
    note = "one acting as its branch starts or ends, as a cooldown or a claim, goes after a comma: `when(has_target, cooldown(8.0))`"
)]
pub trait IntoCondition<M>: Sized {
    /// The condition it is.
    type Condition: Condition;

    /// It, as a condition.
    fn into_condition(self) -> Self::Condition;

    /// Passes when this and `other` both do; `other` only asked if this does.
    ///
    /// `a && b` in `when(..)`.
    fn and<N, C: IntoCondition<N>>(self, other: C) -> And<Self::Condition, C::Condition> {
        And(self.into_condition(), other.into_condition())
    }

    /// Passes when this or `other` does; `other` only asked if this doesn't.
    ///
    /// `a || b` in `when(..)`.
    fn or<N, C: IntoCondition<N>>(self, other: C) -> Or<Self::Condition, C::Condition> {
        Or(self.into_condition(), other.into_condition())
    }
}

/// A closure taking the [`Context`] is a condition of your own, as a
/// [`Condition`] is.
///
/// Written `|cx: &mut Context| ..`, it reads anything, and says what it waits
/// on as one does.
impl<F> Condition for F
where
    F: FnMut(&mut Context) -> bool + Send + Sync + 'static,
{
    fn passes(&mut self, cx: &mut Context) -> bool {
        self(cx)
    }
}

/// A closure taking the [`Context`] shows as `condition`: it has no name of
/// its own.
impl<F> Inspect for F
where
    F: FnMut(&mut Context) -> bool + Send + Sync + 'static,
{
    fn inspect(&self, inspection: &mut Inspection<'_>) {
        inspection.name("condition");
    }
}

impl<C: Condition> IntoCondition<()> for C {
    type Condition = C;

    fn into_condition(self) -> C {
        self
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use bevy::ecs::error::{ErrorContext, FallbackErrorHandler};
    use bevy::prelude::*;

    use crate::behavior::Context;
    use crate::condition::{Condition, IntoCondition, with};
    use crate::prelude::*;
    use crate::testing::*;

    fn at_least(In((agent, least)): In<(Entity, u32)>, levels: Query<&Level>) -> bool {
        levels.get(agent).is_ok_and(|level| level.0 >= least)
    }

    #[derive(Component)]
    struct Level(u32);

    #[test]
    fn boolean_conditions_read_as_written() {
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
    fn a_condition_takes_a_parameter() {
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
        assert_eq!(acts(&app), 1, "one function, two conditions");
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
    fn a_condition_or_act_can_use_question_marks_and_an_error_counts_as_no() {
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
    fn a_condition_built_by_hand_goes_in_too() {
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
    fn a_condition_s_local_is_shared_by_every_agent_using_it() {
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
    fn a_condition_can_queue_commands_as_a_run_condition_can() {
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

    /// Whether another entity's `Level` is above 2: a condition of its own.
    ///
    /// It reads beyond the agent.
    #[derive(Inspect)]
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

    static SILENT: AtomicUsize = AtomicUsize::new(0);

    /// A condition of its own saying nothing it waits on.
    #[derive(Inspect)]
    struct Silent;

    impl Condition for Silent {
        fn passes(&mut self, _cx: &mut Context) -> bool {
            SILENT.fetch_add(1, Ordering::Relaxed);
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
                    when(is_low || Silent) => task(Work),
                    Wait(100.0),
                }
            },
        );
        app.ticks(4);
        assert_eq!(SILENT.load(Ordering::Relaxed), 4);
    }
}
