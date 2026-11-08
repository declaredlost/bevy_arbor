//! Slots: the places on a smart object an agent can claim, each offering an
//! activity, and maybe behavior of its own.

use bevy::prelude::*;

/// On a slot: the smart object it's on, as a Bevy relationship.
///
/// A slot is an entity: a seat on a bench, the gunner's place on a turret.
/// What it offers is a component of yours on it (`Sit`, `ManTurret`), what
/// [`FreeSlots`](crate::query::FreeSlots) finds it by; where it is, its
/// transform, usually a child of the object's. A `Plugged<Using>` on it is the
/// behavior it brings: plugged into whoever claims it. Despawning the object
/// despawns its slots.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_arbor_smart::prelude::*;
/// #[derive(Component)]
/// struct Bench;
///
/// #[derive(Component)]
/// struct Sit;
///
/// fn spawn_bench(mut commands: Commands) {
///     commands
///         .spawn((Bench, Transform::from_xyz(4.0, 0.0, 0.0)))
///         .with_related::<SlotOf>((Sit, Transform::from_xyz(3.5, 0.0, 0.0)))
///         .with_related::<SlotOf>((Sit, Transform::from_xyz(4.5, 0.0, 0.0)));
/// }
/// ```
#[derive(Component, Reflect, Debug, Clone, Copy, PartialEq, Eq)]
#[reflect(Component, Debug, PartialEq)]
#[relationship(relationship_target = Slots)]
pub struct SlotOf(pub Entity);

impl SlotOf {
    /// The smart object it's on.
    #[must_use]
    pub fn object(&self) -> Entity {
        self.0
    }
}

/// On a smart object: its slots, kept by [`SlotOf`].
#[derive(Component, Reflect, Debug, Default)]
#[reflect(Component, Debug)]
#[relationship_target(relationship = SlotOf, linked_spawn)]
pub struct Slots(Vec<Entity>);

impl Slots {
    /// The slots.
    pub fn iter(&self) -> impl Iterator<Item = Entity> + '_ {
        self.0.iter().copied()
    }
}

/// On a slot: it can't be found or claimed. One claimed as it's disabled is
/// given up: what claimed it is cancelled, as its decorator stops passing.
#[derive(Component, Reflect, Debug, Clone, Copy, Default)]
#[reflect(Component, Debug, Default)]
pub struct SlotDisabled;

/// The socket a claimed slot's behavior is plugged into: `Socket(Using)` in
/// the claiming agent's tree runs it.
///
/// A slot offers behavior with a `Plugged<Using>` on it, as the agent's own
/// would be: claiming the slot plugs it into the agent, and releasing it takes
/// it out.
#[derive(Debug, Clone, Copy, Default)]
pub struct Using;
