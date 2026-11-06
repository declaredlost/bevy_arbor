use std::fmt::Debug;

use bevy::ecs::system::SystemId;
use bevy::prelude::*;

use crate::tree::TreeOutline;

/// A system asked about an agent. It's cached by its type, so every tree
/// using it shares one, and it can't capture anything: what varies goes in
/// as a parameter.
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

    /// Runs it for `agent`, if it can run: one that can't, because what it
    /// asks for isn't there, is logged and gives nothing.
    pub(crate) fn run(&mut self, world: &mut World, agent: Entity) -> Option<O> {
        self.system
            .run(world, agent)
            .inspect_err(|error| debug!("{} didn't run: {error}", self.name))
            .ok()
    }
}

/// A cached system, its input built from the agent: what [`Check`] holds,
/// without the input's type.
trait Run<O>: Send + Sync {
    /// Runs it for `agent`, registering it on its first run.
    fn run(&mut self, world: &mut World, agent: Entity) -> Result<O, String>;
}

/// `system`, registered with the world once it first runs. `register` is
/// how, kept as a function so the system's marker type isn't part of this.
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
    fn run(&mut self, world: &mut World, agent: Entity) -> Result<O, String> {
        let (system, register) = (self.system, self.register);
        let id = *self.id.get_or_insert_with(|| register(world, system));
        world
            .run_system_with(id, (self.input)(agent))
            .map_err(|error| error.to_string())
    }
}
