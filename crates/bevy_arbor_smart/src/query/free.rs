use std::marker::PhantomData;

use bevy::prelude::*;
use bevy_arbor_eqs::anchor::{EnvQueryAnchor, Locate};
use bevy_arbor_eqs::generate::Generate;
use bevy_arbor_eqs::query::{EnvQueryContext, EnvQueryItem};

use crate::claim::ClaimedBy;
use crate::slot::{SlotDisabled, SlotOf};

/// The slots offering `A` within `range` of an anchor that the agent could
/// claim.
///
/// Free or its own already, and not disabled. Each is a thing item, at its
/// slot.
///
/// It looks through every slot offering `A`, so keep `A` to an activity, not a
/// marker every slot has.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_arbor_eqs::prelude::*;
/// # use bevy_arbor_smart::prelude::*;
/// # #[derive(Component)] struct Sit;
/// // The nearest free seat.
/// # struct Seat;
/// let seat = EnvQuery::new(Seat, FreeSlots::<Sit>::within(Querier, 15.0))
///     .score(Distance::to(Querier), Score::lower());
/// ```
pub struct FreeSlots<A: Component> {
    around: EnvQueryAnchor,
    range: f32,
    offering: PhantomData<fn() -> A>,
}

/// The slots offering `A`: what a [`FreeSlots<A>`] looks through.
type Offering<A> = QueryState<
    (Entity, &'static GlobalTransform, Option<&'static ClaimedBy>),
    (With<SlotOf>, With<A>, Without<SlotDisabled>),
>;

impl<A: Component> FreeSlots<A> {
    /// Slots offering `A` within `range` of `around`.
    pub fn within(around: impl Locate, range: f32) -> Self {
        Self {
            around: EnvQueryAnchor::new(around),
            range,
            offering: PhantomData,
        }
    }
}

impl<A: Component> Generate for FreeSlots<A> {
    type State = Offering<A>;

    fn generate(&self, slots: &mut Offering<A>, cx: &EnvQueryContext, out: &mut Vec<EnvQueryItem>) {
        let anchors = cx.locate(&self.around);
        if anchors.is_empty() {
            return;
        }
        let agent = cx.querier();
        for (slot, transform, claimed) in cx.query(slots) {
            if claimed.is_some_and(|claimed| claimed.agent() != agent) {
                continue;
            }
            let position = cx.space().flatten(transform.translation());
            if anchors.iter().any(|at| at.reaches(position, self.range)) {
                out.push(EnvQueryItem::thing(slot, position));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use bevy_arbor_eqs::anchor::Querier;
    use bevy_arbor_eqs::generate::Generate;
    use bevy_arbor_eqs::query::EnvQueryContext;
    use bevy_arbor_perception::Space;

    use super::FreeSlots;
    use crate::prelude::*;
    use crate::testing::*;

    #[test]
    fn free_slots_are_those_the_agent_could_claim() {
        let mut app = app();
        let agent = app.spawn(GlobalTransform::IDENTITY);
        let other = app.spawn(GlobalTransform::IDENTITY);
        let seats = bench(
            &mut app,
            &[
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(2.0, 0.0, 0.0),
                Vec3::new(3.0, 0.0, 0.0),
                Vec3::new(4.0, 0.0, 0.0),
                Vec3::new(50.0, 0.0, 0.0),
            ],
        );
        app.entity_mut(seats[1]).insert(ClaimedBy(other));
        app.entity_mut(seats[2]).insert(ClaimedBy(agent));
        app.entity_mut(seats[3]).insert(SlotDisabled);
        let bench = app.world().get::<SlotOf>(seats[0]).unwrap().object();
        app.spawn((SlotOf(bench), GlobalTransform::from_xyz(1.0, 0.0, 0.0)));
        let generator = FreeSlots::<Sit>::within(Querier, 10.0);
        let mut slots = FromWorld::from_world(app.world_mut());
        let cx = EnvQueryContext::new(app.world(), agent, Space::Volume, 1);
        let mut items = Vec::new();
        generator.generate(&mut slots, &cx, &mut items);
        let mut found: Vec<Entity> = items.iter().filter_map(|item| item.entity).collect();
        found.sort();
        let mut expected = vec![seats[0], seats[2]];
        expected.sort();
        assert_eq!(
            found, expected,
            "free, or its own; not another's, disabled, far, or offering nothing"
        );
    }
}
