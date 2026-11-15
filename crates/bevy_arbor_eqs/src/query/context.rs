use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use bevy::ecs::query::{QueryFilter, ReadOnlyQueryData};
use bevy::prelude::*;
use bevy_arbor_perception::Space;
use bevy_arbor_perception::sense::Facing;

use crate::anchor::{EnvQueryAnchor, Located};

/// What a query runs with: the world, read-only, and whose query it is.
///
/// What generators, tests and anchors are given.
pub struct EnvQueryContext<'w> {
    world: &'w World,
    querier: Entity,
    /// What carries the query: the querier, or the thing it asked it of.
    carrier: Entity,
    space: Space,
    /// Where the query's own randomness is up to.
    rng: Cell<u64>,
    /// Where each anchor was found this run, kept with the anchor.
    ///
    /// The anchor is kept so its address isn't another's meanwhile.
    located: RefCell<Vec<(EnvQueryAnchor, Rc<[Located]>)>>,
}

impl<'w> EnvQueryContext<'w> {
    /// `querier`'s query, in `space`, its randomness from `seed`.
    #[must_use]
    pub fn new(world: &'w World, querier: Entity, space: Space, seed: u64) -> Self {
        Self {
            world,
            querier,
            carrier: querier,
            space,
            // Never zero: xorshift stays there.
            rng: Cell::new(seed | 1),
            located: RefCell::new(Vec::new()),
        }
    }

    /// Where `anchor` is: located once a run, however many items and tests
    /// ask.
    ///
    /// A test measuring every item from it locates it only once.
    pub fn locate(&self, anchor: &EnvQueryAnchor) -> Rc<[Located]> {
        if let Some((_, found)) = self
            .located
            .borrow()
            .iter()
            .find(|(known, _)| Arc::ptr_eq(&known.0, &anchor.0))
        {
            return Rc::clone(found);
        }
        let mut found = Vec::new();
        anchor.0.locate(self, &mut found);
        let found: Rc<[Located]> = found.into();
        self.located
            .borrow_mut()
            .push((anchor.clone(), Rc::clone(&found)));
        found
    }

    /// The world, to read.
    #[must_use]
    pub fn world(&self) -> &'w World {
        self.world
    }

    /// What `state` finds in the world, as it is now.
    ///
    /// Brought up to date with what's been spawned since it was made.
    ///
    /// For a generator or test keeping a `QueryState` as its state.
    pub fn query<'s, D: ReadOnlyQueryData, F: QueryFilter>(
        &self,
        state: &'s mut QueryState<D, F>,
    ) -> Query<'w, 's, D, F> {
        state.update_archetypes(self.world);
        state.query_manual(self.world)
    }

    /// The query carried by `carrier`, not the querier: what [`Carrier`]
    /// is.
    ///
    /// [`Carrier`]: crate::anchor::Carrier
    #[must_use]
    pub fn carried_by(mut self, carrier: Entity) -> Self {
        self.carrier = carrier;
        self
    }

    /// The agent running the query: what [`Querier`](crate::anchor::Querier)
    /// is.
    #[must_use]
    pub fn querier(&self) -> Entity {
        self.querier
    }

    /// What carries the query: the querier, or the thing it asked it of.
    #[must_use]
    pub fn carrier(&self) -> Entity {
        self.carrier
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

    /// Which way `entity` looks, unit length, or zero if it can't tell.
    ///
    /// From its [`Facing`], as sight takes it, else from its transform.
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
