use std::fmt::Debug;

use bevy::ecs::error::ErrorContext;
use bevy::ecs::system::{RegisteredSystemError, SystemId};
use bevy::prelude::*;
use bevy::utils::prelude::DebugName;

use crate::behaviour::Context;
use crate::tree::TreeOutline;

/// A system asked about an agent: returning its answer, or a `Result` of it.
///
/// A `Result` is for one using `?` (Bevy's own conversion). It's cached by its
/// type, so every tree, node and agent using it shares one, with its `Local`s
/// and change ticks: that's what keeps it cheap. It can't capture anything:
/// what varies goes in as a parameter.
pub(crate) struct Check<O> {
    name: String,
    system: Box<dyn Run<O>>,
}

impl<O: 'static> Check<O> {
    /// `system`, taking the agent.
    pub(crate) fn new<S, M>(system: S) -> Self
    where
        S: IntoSystem<In<Entity>, O, M> + Copy + Send + Sync + 'static,
    {
        let name = TreeOutline::short::<S>().to_owned();
        Self::build(name, system, |agent| agent)
    }

    /// `system`, taking the agent and `param`.
    pub(crate) fn with<S, P, M>(system: S, param: P) -> Self
    where
        S: IntoSystem<In<(Entity, P)>, O, M> + Copy + Send + Sync + 'static,
        P: Clone + Debug + Send + Sync + 'static,
    {
        let name = format!("{}({param:?})", TreeOutline::short::<S>());
        Self::build(name, system, move |agent| (agent, param.clone()))
    }

    fn build<T, S, M>(
        name: String,
        system: S,
        input: impl Fn(Entity) -> T + Send + Sync + 'static,
    ) -> Self
    where
        T: 'static,
        S: IntoSystem<In<T>, O, M> + Copy + Send + Sync + 'static,
    {
        const {
            assert!(
                size_of::<S>() == 0,
                "guards, acts, scores and rolls are cached by their type, so they can't \
                 capture anything: pass what varies as a parameter"
            );
        }
        Self {
            name,
            system: Box::new(Cached {
                system,
                input,
                register: |world, system| world.register_system_cached(system),
                id: None,
            }),
        }
    }

    /// What it's called in the outline.
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    /// Runs it for the agent, if it can run, and its tree is still there.
    ///
    /// One skipped, as Bevy skips a system, is logged and gives nothing. One
    /// returning an error, or that can't run for any other reason, gives
    /// nothing, and the error goes to Bevy's error handler, as a scheduled
    /// system's would.
    pub(crate) fn run(&mut self, cx: &mut Context) -> Option<O> {
        if cx.gone() {
            return None;
        }
        let agent = cx.agent();
        let world = cx.world_mut();
        match self.system.run(world, agent) {
            Ok(value) => Some(value),
            Err(Ran::Failed(error)) => {
                // Asked by its tree, not a schedule: the last change is the
                // nearest there is to its last run.
                let context = ErrorContext::System {
                    name: DebugName::owned(self.name.clone()),
                    last_run: world.last_change_tick(),
                };
                (world.fallback_error_handler())(error, context);
                None
            }
            Err(Ran::Skipped(why)) => {
                debug!("{} didn't run: {why}", self.name);
                None
            }
        }
    }
}

/// A cached system, its input built from the agent: what [`Check`] holds,
/// without the input's type.
trait Run<O>: Send + Sync {
    /// Runs it for `agent`, registering it on its first run.
    fn run(&mut self, world: &mut World, agent: Entity) -> Result<O, Ran>;
}

/// Why a system gave no answer.
enum Ran {
    /// It returned an error, or couldn't run: Bevy's error handler's.
    Failed(BevyError),
    /// It was skipped, as Bevy skips a system: not an error.
    Skipped(String),
}

/// `system`, registered with the world once it first runs. `register` is how,
/// kept as a function so the system's marker type isn't part of this.
struct Cached<S, F, T: 'static, O: 'static> {
    system: S,
    input: F,
    register: fn(&mut World, S) -> SystemId<In<T>, O>,
    id: Option<SystemId<In<T>, O>>,
}

impl<S, F, T, O> Run<O> for Cached<S, F, T, O>
where
    S: Copy + Send + Sync + 'static,
    F: Fn(Entity) -> T + Send + Sync,
    T: 'static,
    O: 'static,
{
    fn run(&mut self, world: &mut World, agent: Entity) -> Result<O, Ran> {
        let (system, register) = (self.system, self.register);
        let id = *self.id.get_or_insert_with(|| register(world, system));
        match world.run_system_with(id, (self.input)(agent)) {
            // Unregistered since it was registered: registered again.
            Err(RegisteredSystemError::SystemIdNotRegistered(_)) => {
                let id = register(world, system);
                self.id = Some(id);
                world.run_system_with(id, (self.input)(agent))
            }
            ran => ran,
        }
        .map_err(|error| match error {
            RegisteredSystemError::Failed(error) => Ran::Failed(error),
            RegisteredSystemError::Skipped(why) => Ran::Skipped(why.to_string()),
            other => Ran::Failed(other.into()),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use bevy::ecs::error::{ErrorContext, FallbackErrorHandler};
    use bevy::ecs::system::RegisteredSystem;
    use bevy::prelude::*;

    use crate::prelude::*;
    use crate::testing::*;

    fn work_or_other() -> Behaviour {
        behaviour_tree! {
            Forever => {
                Select => {
                    when(high) => task("Work", Work),
                    task("Other", Other),
                }
            }
        }
    }

    #[test]
    fn a_guard_unregistered_from_outside_is_registered_again() {
        let mut app = app();
        let npc = agent(&mut app, work_or_other());
        app.tick();
        app.world_mut()
            .unregister_system_cached::<In<Entity>, bool, _, _>(high)
            .unwrap();
        set(&mut app, npc, true, false);
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 1, "asked again");
    }

    static BROKEN: AtomicUsize = AtomicUsize::new(0);

    fn count_broken(_: BevyError, _: ErrorContext) {
        BROKEN.fetch_add(1, Ordering::Relaxed);
    }

    #[test]
    fn a_guard_that_cannot_run_goes_to_the_error_handler() {
        let mut app = app();
        app.app_mut()
            .insert_resource(FallbackErrorHandler(count_broken));
        agent(&mut app, work_or_other());
        app.tick();
        let world = app.world_mut();
        let id = world.register_system_cached(high);
        world
            .entity_mut(id.entity())
            .remove::<RegisteredSystem<In<Entity>, bool>>();
        app.tick();
        assert_eq!(BROKEN.load(Ordering::Relaxed), 1);
    }

    #[derive(Resource)]
    struct Missing;

    fn needs_missing(In(_): In<Entity>, _: Res<Missing>) -> bool {
        true
    }

    static MISSING: AtomicUsize = AtomicUsize::new(0);

    fn count_missing(_: BevyError, _: ErrorContext) {
        MISSING.fetch_add(1, Ordering::Relaxed);
    }

    #[test]
    fn a_guard_on_a_missing_resource_goes_to_the_error_handler() {
        let mut app = app();
        app.app_mut()
            .insert_resource(FallbackErrorHandler(count_missing));
        let npc = agent(
            &mut app,
            behaviour_tree! { Select => { when(needs_missing) => act(count) } },
        );
        app.tick();
        assert_eq!(outcome(&app, npc), Some(false));
        assert_eq!(MISSING.load(Ordering::Relaxed), 1);
    }
}
