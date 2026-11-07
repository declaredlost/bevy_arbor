use std::any::{Any, TypeId};
use std::fmt::Debug;
use std::sync::atomic::{AtomicU64, Ordering};

use bevy::ecs::change_detection::CheckChangeTicks;
use bevy::ecs::error::ErrorContext;
use bevy::ecs::system::{BoxedSystem, RunSystemError};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::utils::prelude::DebugName;

use crate::behaviour::Context;
use crate::tree::TreeOutline;

/// A system asked about an agent: returning its answer, or a `Result` of it.
///
/// A `Result` is for one using `?` (Bevy's own conversion). It's kept by its
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
                "guards, acts, scores and rolls are kept by their type, so they can't \
                 capture anything: pass what varies as a parameter"
            );
        }
        Self {
            name,
            system: Box::new(Kept {
                system,
                input,
                make: |system| Box::new(IntoSystem::into_system(system)),
                at: None,
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
        let (world, systems) = cx.world_and_systems();
        let ran = if let Some(systems) = systems {
            self.system.run(world, systems, agent)
        } else {
            // Outside a tick: the world's own, taken out for the run.
            let mut systems = world.remove_resource::<Systems>().unwrap_or_default();
            let ran = self.system.run(world, &mut systems, agent);
            world.insert_resource(systems);
            ran
        };
        match ran {
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

/// A kept system, its input built from the agent: what [`Check`] holds,
/// without the input's type.
trait Run<O>: Send + Sync {
    /// Runs it for `agent`, keeping it in `systems` on its first run.
    fn run(&mut self, world: &mut World, systems: &mut Systems, agent: Entity) -> Result<O, Ran>;
}

/// Why a system gave no answer.
enum Ran {
    /// It returned an error, or couldn't run: Bevy's error handler's.
    Failed(BevyError),
    /// It was skipped, as Bevy skips a system: not an error.
    Skipped(String),
}

/// `system`, kept in a table the first time it runs, and found there by its
/// place after. `make` is how it's built, kept as a function so the system's
/// marker type isn't part of this.
struct Kept<S, F, T: 'static, O: 'static> {
    system: S,
    input: F,
    make: fn(S) -> BoxedSystem<In<T>, O>,
    /// Which table it's in, and where.
    at: Option<(u64, usize)>,
}

impl<S, F, T, O> Run<O> for Kept<S, F, T, O>
where
    S: Copy + Send + Sync + 'static,
    F: Fn(Entity) -> T + Send + Sync,
    T: 'static,
    O: 'static,
{
    fn run(&mut self, world: &mut World, systems: &mut Systems, agent: Entity) -> Result<O, Ran> {
        let index = match self.at {
            Some((table, index)) if table == systems.id => index,
            // First run, or another table: another world's, or one made while
            // this world's was out.
            _ => {
                let index = systems.place(self.system, self.make);
                self.at = Some((systems.id, index));
                index
            }
        };
        let Some(slot) = systems.slot::<T, O>(index) else {
            return Err(Ran::Failed("its system isn't where it was kept".into()));
        };
        if !slot.initialized {
            slot.system.initialize(world);
            slot.initialized = true;
        }
        let ran = slot
            .system
            .run_without_applying_deferred((self.input)(agent), world);
        slot.system.queue_deferred(world.into());
        world.flush();
        ran.map_err(|error| match error {
            RunSystemError::Failed(error) => Ran::Failed(error),
            RunSystemError::Skipped(why) => Ran::Skipped(why.to_string()),
        })
    }
}

/// Where the next table's number comes from.
static TABLES: AtomicU64 = AtomicU64::new(0);

/// Every system trees run, by its type: one each, shared by every tree, node
/// and agent, with its `Local`s and change ticks.
///
/// Kept here rather than registered with the world: running a registered
/// system finds it on its entity twice, which costs more than most systems
/// do. It's taken out of the world while trees tick, so a system is one
/// lookup in a list.
#[derive(Resource)]
pub(crate) struct Systems {
    /// Which table this is, so a check never reads another's places.
    id: u64,
    places: HashMap<TypeId, usize>,
    slots: Vec<Box<dyn Stored>>,
}

impl Default for Systems {
    fn default() -> Self {
        Self {
            id: TABLES.fetch_add(1, Ordering::Relaxed),
            places: HashMap::default(),
            slots: Vec::new(),
        }
    }
}

impl Systems {
    /// Where `system` is, put there if it isn't yet.
    fn place<S: 'static, T: 'static, O: 'static>(
        &mut self,
        system: S,
        make: fn(S) -> BoxedSystem<In<T>, O>,
    ) -> usize {
        *self
            .places
            .entry(TypeId::of::<(S, T, O)>())
            .or_insert_with(|| {
                self.slots.push(Box::new(Slot {
                    system: make(system),
                    initialized: false,
                }));
                self.slots.len() - 1
            })
    }

    /// The system at `index`, if it takes `T` and returns `O`.
    fn slot<T: 'static, O: 'static>(&mut self, index: usize) -> Option<&mut Slot<T, O>> {
        let slot: &mut dyn Any = &mut **self.slots.get_mut(index)?;
        slot.downcast_mut()
    }

    /// Keeps every system's change ticks from getting too old, as a schedule
    /// does for its own.
    pub(crate) fn check_ticks(&mut self, check: CheckChangeTicks) {
        for slot in &mut self.slots {
            slot.check_ticks(check);
        }
    }
}

/// A system, and whether it's been initialized yet.
struct Slot<T: 'static, O: 'static> {
    system: BoxedSystem<In<T>, O>,
    initialized: bool,
}

/// A [`Slot`], whatever it takes and returns.
trait Stored: Any + Send + Sync {
    fn check_ticks(&mut self, check: CheckChangeTicks);
}

impl<T: 'static, O: 'static> Stored for Slot<T, O> {
    fn check_ticks(&mut self, check: CheckChangeTicks) {
        self.system.check_change_tick(check);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use bevy::ecs::error::{ErrorContext, FallbackErrorHandler};
    use bevy::prelude::*;

    use crate::prelude::*;
    use crate::testing::*;

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
