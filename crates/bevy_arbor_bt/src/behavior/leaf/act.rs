use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use bevy::ecs::error::ErrorContext;
use bevy::ecs::system::RegisteredSystemError;
use bevy::prelude::*;
use bevy::utils::prelude::DebugName;

use crate::behavior::{Behave, Context, Outcome, Status};

/// What an act's system said: nothing yet, no, or yes.
#[derive(Default)]
struct Said(AtomicU8);

impl Said {
    const NO: u8 = 1;
    const NOTHING: u8 = 0;
    const YES: u8 = 2;

    fn set(&self, yes: bool) {
        self.0
            .store(if yes { Self::YES } else { Self::NO }, Ordering::Relaxed);
    }

    fn get(&self) -> Option<bool> {
        match self.0.load(Ordering::Relaxed) {
            Self::NOTHING => None,
            said => Some(said == Self::YES),
        }
    }
}

type Queue = Box<dyn Fn(&mut Commands, Entity, Arc<Said>) + Send + Sync>;

/// Runs a system once: what it returns is the result.
///
/// It runs with the tree's commands, as a cached Bevy system: its tree is
/// ticked again once they're applied, in the same run, to hear what it said.
pub(in crate::behavior) struct Act {
    name: String,
    queue: Queue,
    /// What it's waiting to hear from, once it's run.
    running: Option<Arc<Said>>,
}

impl Act {
    /// Runs `system`, its input made from the agent by `input`.
    pub(in crate::behavior) fn new<S, T, M>(
        name: String,
        system: S,
        input: impl Fn(Entity) -> T + Send + Sync + 'static,
    ) -> Self
    where
        S: IntoSystem<In<T>, bool, M> + Copy + Send + Sync + 'static,
        T: Send + 'static,
    {
        let label = name.clone();
        let queue: Queue = Box::new(move |commands, agent, said| {
            let input = input(agent);
            let name = label.clone();
            commands.queue(move |world: &mut World| {
                said.set(run(world, system, input, &name));
            });
        });
        Self {
            name,
            queue,
            running: None,
        }
    }
}

/// Runs `system` on `input`: `false` if it can't, or returns an error, which
/// goes to Bevy's error handler, as a scheduled system's would.
fn run<S, T, M>(world: &mut World, system: S, input: T, name: &str) -> bool
where
    S: IntoSystem<In<T>, bool, M> + 'static,
    T: 'static,
{
    match world.run_system_cached_with(system, input) {
        Ok(said) => said,
        Err(RegisteredSystemError::Skipped(why)) => {
            debug!("{name} didn't run: {why}");
            false
        }
        Err(error) => {
            let error = match error {
                RegisteredSystemError::Failed(error) => error,
                other => BevyError::from(other.to_string()),
            };
            let context = ErrorContext::System {
                name: DebugName::owned(name.to_owned()),
                last_run: world.last_change_tick(),
            };
            (world.fallback_error_handler())(error, context);
            false
        }
    }
}

impl Behave for Act {
    fn tick(&mut self, cx: &mut Context) -> Status {
        let Some(said) = &self.running else {
            let said = Arc::new(Said::default());
            let agent = cx.agent();
            (self.queue)(&mut cx.commands(), agent, said.clone());
            self.running = Some(said);
            cx.acted();
            return Status::Running;
        };
        match said.get() {
            Some(said) => {
                self.running = None;
                Outcome::from(said).into()
            }
            // Its commands not applied yet: asked again next tick.
            None => Status::Running,
        }
    }

    fn end(&mut self, _cx: &mut Context, _outcome: Option<Outcome>) {
        self.running = None;
    }

    fn label(&self) -> String {
        format!("act {}", self.name)
    }
}
