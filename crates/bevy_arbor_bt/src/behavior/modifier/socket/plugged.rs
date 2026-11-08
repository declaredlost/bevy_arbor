use std::fmt;
use std::marker::PhantomData;
use std::sync::Arc;

use bevy::prelude::*;
use bevy::utils::prelude::ShortName;

use crate::behavior::Behavior;

/// On an agent: what's plugged into its tree's socket `S`, run as a
/// [`Socket`](Behavior::socket) reaches it.
///
/// Inserting one plugs it in, inserting another swaps it, and removing it
/// unplugs it. A socket running as it's swapped cancels what it ran and starts
/// the new one, once what it ran stops holding; as it's unplugged, it fails.
///
/// It holds how to build the subtree, as each agent runs its own:
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_arbor_bt::prelude::*;
/// # #[derive(Component, Clone)] struct Kite;
/// /// The socket weapons plug into.
/// struct Weapon;
///
/// fn bow_tactics() -> Behavior {
///     behavior_tree! { task(Kite) }
/// }
///
/// # fn equip(mut commands: Commands, npc: Entity) {
/// commands
///     .entity(npc)
///     .insert(Plugged::new(Weapon, bow_tactics));
/// # }
/// ```
#[derive(Component)]
pub struct Plugged<S: Send + Sync + 'static> {
    build: Arc<dyn Fn() -> Behavior + Send + Sync>,
    socket: PhantomData<fn() -> S>,
}

impl<S: Send + Sync + 'static> Plugged<S> {
    /// Plugs what `build` builds into the socket `S`.
    pub fn new(_socket: S, build: impl Fn() -> Behavior + Send + Sync + 'static) -> Self {
        Self {
            build: Arc::new(build),
            socket: PhantomData,
        }
    }

    /// A subtree built from it.
    pub(crate) fn build(&self) -> Behavior {
        (self.build)()
    }

    /// Whether it's the same as `other`: a clone of it, plugging in the same.
    #[must_use]
    pub fn same(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.build, &other.build)
    }
}

impl<S: Send + Sync + 'static> Clone for Plugged<S> {
    fn clone(&self) -> Self {
        Self {
            build: Arc::clone(&self.build),
            socket: PhantomData,
        }
    }
}

impl<S: Send + Sync + 'static> fmt::Debug for Plugged<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Plugged<{}>", ShortName::of::<S>())
    }
}

/// A typed condition: whether something is plugged into the agent's socket `S`.
///
/// `when(sees_enemy && plugged::<Weapon>) => Socket(Weapon)` keeps an empty
/// socket out of the way, and wakes the tree as something is plugged in or
/// taken out.
#[must_use]
pub fn plugged<S: Send + Sync + 'static>(_: &Plugged<S>) -> bool {
    true
}
