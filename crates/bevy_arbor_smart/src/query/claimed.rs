use bevy_arbor_eqs::anchor::{Locate, Located};
use bevy_arbor_eqs::query::QueryContext;

use crate::claim::Claims;

/// An anchor at the slots the agent holds: what a query measures from to be
/// about the agent's slot, not the agent.
///
/// `Donut::around(ClaimedSlots)` is the places round it, a spot to step off
/// a turret to, and `Distance::to(ClaimedSlots)` how far each item is from
/// it, the ammo crate nearest the turret. Nowhere while the agent holds none.
#[derive(Debug, Clone, Copy, Default)]
pub struct ClaimedSlots;

impl Locate for ClaimedSlots {
    fn locate(&self, cx: &QueryContext, out: &mut Vec<Located>) {
        let Some(claims) = cx.world().get::<Claims>(cx.agent()) else {
            return;
        };
        for slot in claims.iter() {
            if let Some(position) = cx.position(slot) {
                out.push(Located {
                    position,
                    ahead: cx.ahead(slot),
                    entity: Some(slot),
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use bevy_arbor_eqs::anchor::Locate;
    use bevy_arbor_eqs::prelude::*;
    use bevy_arbor_eqs::query::QueryContext;
    use bevy_arbor_perception::Space;

    use super::ClaimedSlots;
    use crate::prelude::*;
    use crate::testing::*;

    fn located(app: &bevy_harness::TestApp, agent: Entity) -> Vec<Vec3> {
        let cx = QueryContext::new(app.world(), agent, Space::Volume, 1);
        let mut out = Vec::new();
        ClaimedSlots.locate(&cx, &mut out);
        out.into_iter().map(|located| located.position).collect()
    }

    #[test]
    fn it_is_where_the_slots_the_agent_holds_are() {
        let mut app = app();
        let seats = bench(
            &mut app,
            &[Vec3::new(5.0, 0.0, 0.0), Vec3::new(9.0, 0.0, 0.0)],
        );
        let agent = app.spawn(GlobalTransform::IDENTITY);
        assert!(located(&app, agent).is_empty(), "nowhere, holding none");
        app.entity_mut(seats[1]).insert(ClaimedBy(agent));
        assert_eq!(located(&app, agent), [Vec3::new(9.0, 0.0, 0.0)]);
    }

    /// The query: what's nearest the agent's seat.
    struct NearSeat;

    #[test]
    fn a_query_measures_from_it() {
        let mut app = app();
        let seats = bench(&mut app, &[Vec3::new(10.0, 0.0, 0.0)]);
        let agent = app.spawn(GlobalTransform::IDENTITY);
        app.entity_mut(seats[0]).insert(ClaimedBy(agent));
        app.app_mut().add_env_query(
            NearSeat,
            EnvQuery::new(Donut::around(Agent).radius(2.0, 2.0).points(4))
                .score(Distance::to(ClaimedSlots), Score::lower()),
        );
        app.entity_mut(agent).insert(Survey::of(NearSeat));
        app.tick();
        let spot = app
            .world()
            .get::<Found<NearSeat>>(agent)
            .unwrap()
            .position();
        let spot = spot.unwrap();
        assert!(
            (spot - Vec3::new(2.0, 0.0, 0.0)).length() < 1e-4,
            "the point toward the seat: {spot}"
        );
    }
}
