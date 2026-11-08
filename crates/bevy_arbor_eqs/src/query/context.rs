use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use bevy::prelude::*;
use bevy_arbor_perception::Space;
use bevy_arbor_perception::sense::Facing;

use crate::anchor::{Locate, Located};

/// What a query runs with: the world, read-only, and whose query it is.
///
/// What generators, tests and anchors are given.
pub struct QueryContext<'w> {
    world: &'w World,
    agent: Entity,
    space: Space,
    /// Where the query's own randomness is up to.
    rng: Cell<u64>,
    /// Where each anchor was found this run, kept with the anchor, so its
    /// address isn't another's meanwhile.
    located: RefCell<Vec<(Arc<dyn Locate>, Rc<[Located]>)>>,
}

impl<'w> QueryContext<'w> {
    /// `agent`'s query, in `space`, its randomness from `seed`.
    #[must_use]
    pub fn new(world: &'w World, agent: Entity, space: Space, seed: u64) -> Self {
        Self {
            world,
            agent,
            space,
            // Never zero: xorshift stays there.
            rng: Cell::new(seed | 1),
            located: RefCell::new(Vec::new()),
        }
    }

    /// Where `anchor` is: located once a run, however many items and tests
    /// ask, so a test measuring every item from it locates it only once.
    pub fn locate(&self, anchor: &Arc<dyn Locate>) -> Rc<[Located]> {
        if let Some((_, found)) = self
            .located
            .borrow()
            .iter()
            .find(|(known, _)| Arc::ptr_eq(known, anchor))
        {
            return Rc::clone(found);
        }
        let mut found = Vec::new();
        anchor.locate(self, &mut found);
        let found: Rc<[Located]> = found.into();
        self.located
            .borrow_mut()
            .push((Arc::clone(anchor), Rc::clone(&found)));
        found
    }

    /// The world, to read.
    #[must_use]
    pub fn world(&self) -> &'w World {
        self.world
    }

    /// Whose query it is.
    #[must_use]
    pub fn agent(&self) -> Entity {
        self.agent
    }

    /// The space it's in: the one perception works in.
    #[must_use]
    pub fn space(&self) -> Space {
        self.space
    }

    /// Where `entity` is, in this space, if it's anywhere.
    #[must_use]
    pub fn position(&self, entity: Entity) -> Option<Vec3> {
        let transform = self.world.get::<GlobalTransform>(entity)?;
        Some(self.space.flatten(transform.translation()))
    }

    /// Which way `entity` looks, unit length, or zero if it can't tell: its
    /// [`Facing`], as sight takes it, else as its transform does.
    #[must_use]
    pub fn ahead(&self, entity: Entity) -> Vec3 {
        let facing = self.world.get::<Facing>(entity);
        self.world
            .get::<GlobalTransform>(entity)
            .map_or(Vec3::ZERO, |transform| {
                self.space.looking(transform, facing)
            })
    }

    /// A number from 0 up to 1, the query's own: the same for the same seed.
    pub fn roll(&self) -> f32 {
        // xorshift64*: plenty for picking among the best.
        let mut x = self.rng.get();
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng.set(x);
        let bits = x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 40;
        #[expect(clippy::cast_precision_loss, reason = "24 bits, exact in an f32")]
        let roll = bits as f32 / (1u64 << 24) as f32;
        roll
    }
}
