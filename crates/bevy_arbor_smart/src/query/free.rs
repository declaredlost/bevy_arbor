use std::sync::Arc;

use bevy::prelude::*;
use bevy_arbor_eqs::anchor::{Locate, Located};
use bevy_arbor_eqs::generate::{CachedQuery, Generate};
use bevy_arbor_eqs::query::{Item, QueryContext};

use crate::claim::ClaimedBy;
use crate::slot::{SlotDisabled, SlotOf};

/// The slots offering `A` within `range` of an anchor that the agent could
/// claim: free or its own already, and not disabled. Each is a thing item, at
/// its slot.
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
/// let seat = EnvQuery::new(FreeSlots::<Sit>::within(Agent, 15.0))
///     .score(Distance::to(Agent), Score::lower());
/// ```
pub struct FreeSlots<A: Component> {
    around: Arc<dyn Locate>,
    range: f32,
    slots: CachedQuery<
        (Entity, &'static GlobalTransform, Option<&'static ClaimedBy>),
        (With<SlotOf>, With<A>, Without<SlotDisabled>),
    >,
}

impl<A: Component> FreeSlots<A> {
    /// Slots offering `A` within `range` of `around`.
    pub fn within(around: impl Locate, range: f32) -> Self {
        Self {
            around: Arc::new(around),
            range,
            slots: CachedQuery::default(),
        }
    }
}

impl<A: Component> Generate for FreeSlots<A> {
    fn generate(&self, cx: &QueryContext, out: &mut Vec<Item>) {
        let anchors = cx.locate(&self.around);
        if anchors.is_empty() {
            return;
        }
        let agent = cx.agent();
        self.slots.with(cx.world(), |slots| {
            for (slot, transform, claimed) in &slots {
                if claimed.is_some_and(|claimed| claimed.agent() != agent) {
                    continue;
                }
                let position = cx.space().flatten(transform.translation());
                let near = anchors
                    .iter()
                    .any(|Located { position: at, .. }| at.distance(position) <= self.range);
                if near {
                    out.push(Item::Entity {
                        entity: slot,
                        position,
                    });
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use bevy_arbor_eqs::anchor::Agent;
    use bevy_arbor_eqs::generate::Generate;
    use bevy_arbor_eqs::query::{Item, QueryContext};
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
        let cx = QueryContext::new(app.world(), agent, Space::Volume, 1);
        let mut items = Vec::new();
        FreeSlots::<Sit>::within(Agent, 10.0).generate(&cx, &mut items);
        let mut found: Vec<Entity> = items.iter().filter_map(Item::entity).collect();
        found.sort();
        let mut expected = vec![seats[0], seats[2]];
        expected.sort();
        assert_eq!(
            found, expected,
            "free, or its own; not another's, disabled, far, or offering nothing"
        );
    }
}
