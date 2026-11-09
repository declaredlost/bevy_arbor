//! Claims: an agent holding a slot, so no one else takes it.

use std::marker::PhantomData;

use bevy::prelude::*;
use bevy::utils::prelude::ShortName;
use bevy_arbor_bt::behavior::{Context, Outcome, Plugged};
use bevy_arbor_bt::decorator::Decorate;
use bevy_arbor_eqs::survey::Found;

use crate::slot::{SlotDisabled, SlotOf, Using};

/// On a slot: the agent holding it, as a Bevy relationship. Free without one.
///
/// Added and removed by [`claim`]; an observer of `On<Add<ClaimedBy>>` or
/// `On<Remove<ClaimedBy>>` hears it claimed or given up. Removing it yourself
/// takes the slot from its agent: the branch holding it is cancelled, as its
/// `claim` stops passing. Despawning the agent gives it up.
#[derive(Component, Reflect, Debug, Clone, Copy, PartialEq, Eq)]
#[reflect(Component, Debug, PartialEq)]
#[relationship(relationship_target = Claims)]
pub struct ClaimedBy(pub Entity);

impl ClaimedBy {
    /// The agent holding it.
    #[must_use]
    pub fn agent(&self) -> Entity {
        self.0
    }
}

/// On an agent: the slots it holds, kept by [`ClaimedBy`]: where to go, for a
/// task taking it there.
#[derive(Component, Reflect, Debug, Default)]
#[reflect(Component, Debug)]
#[relationship_target(relationship = ClaimedBy)]
pub struct Claims(Vec<Entity>);

impl Claims {
    /// The slots it holds.
    pub fn iter(&self) -> impl Iterator<Item = Entity> + '_ {
        self.0.iter().copied()
    }
}

/// A decorator claiming the slot the query keyed by `Q` found for the agent,
/// for as long as its branch runs. Made by [`claim`].
pub struct Claim<Q: Send + Sync + 'static> {
    /// The slot it holds, while its branch runs.
    held: Option<Entity>,
    /// Whether it claimed the slot itself, so gives it up as its branch ends:
    /// not if the agent held it already, by another branch.
    claimed: bool,
    /// What the slot plugged into the agent, and what the agent had plugged
    /// in before, set aside to put back as the branch ends.
    swapped: Option<(Plugged<Using>, Option<Plugged<Using>>)>,
    key: PhantomData<fn() -> Q>,
}

/// A decorator claiming the slot the query keyed by `Q` found, for as long as
/// its branch runs: `when(found::<Bench> && claim::<Bench>()) => ..`.
///
/// It passes while that slot is free, or already the agent's, and isn't
/// disabled; once claimed, while it's still the agent's. So a slot taken by
/// another agent first, disabled, or taken from the agent (its [`ClaimedBy`]
/// removed) keeps the branch out, or cancels it.
///
/// As its branch starts, the slot is claimed, if it can be, and its
/// `Plugged<Using>`, if it has one, is plugged into the agent, for
/// `Socket(Using)` to run; as it ends, however it ends, both are given up,
/// and what the agent had plugged in before is put back. A slot the agent
/// already held, by another branch, stays held.
///
/// A branch tells every decorator in its condition it's starting, even one
/// under `!` or on the side of `||` that wasn't asked: so under `||`, each
/// side whose slot can be claimed claims it, and under `!` nothing is.
#[must_use]
pub fn claim<Q: Send + Sync + 'static>() -> Claim<Q> {
    Claim {
        held: None,
        claimed: false,
        swapped: None,
        key: PhantomData,
    }
}

impl<Q: Send + Sync + 'static> Claim<Q> {
    /// The slot the query found, if it found one.
    fn found(cx: &Context) -> Option<Entity> {
        cx.world()
            .get::<Found<Q>>(cx.agent())
            .and_then(Found::entity)
    }
}

/// Whether `agent` can claim `slot`: a slot, not disabled, and free or its
/// own already. Held already, only its own.
fn claimable(world: &World, slot: Entity, agent: Entity, held: bool) -> bool {
    let Ok(slot) = world.get_entity(slot) else {
        return false;
    };
    let holder = slot.get::<ClaimedBy>().map(ClaimedBy::agent);
    let free = if held {
        holder == Some(agent)
    } else {
        holder.is_none_or(|holder| holder == agent)
    };
    slot.contains::<SlotOf>() && !slot.contains::<SlotDisabled>() && free
}

impl<Q: Send + Sync + 'static> Decorate for Claim<Q> {
    fn passes(&mut self, cx: &mut Context) -> bool {
        let agent = cx.agent();
        // What the query finds next changes which slot it is.
        cx.watch::<Found<Q>>(agent);
        // While it holds one, that one; else what was found.
        let Some(slot) = self.held.or_else(|| Self::found(cx)) else {
            return false;
        };
        // Taken, given up or disabled elsewhere wakes the tree.
        cx.watch::<ClaimedBy>(slot);
        cx.watch::<SlotDisabled>(slot);
        claimable(cx.world(), slot, agent, self.held.is_some())
    }

    fn start(&mut self, cx: &mut Context) {
        let agent = cx.agent();
        let Some(slot) = Self::found(cx) else {
            return;
        };
        // Started without passing, under `!` or `||`: nothing to hold.
        if !claimable(cx.world(), slot, agent, false) {
            return;
        }
        let world = cx.world_mut();
        let Ok(mut entity) = world.get_entity_mut(slot) else {
            return;
        };
        self.claimed = entity.get::<ClaimedBy>().is_none();
        if self.claimed {
            entity.insert(ClaimedBy(agent));
        }
        let brought = entity.get::<Plugged<Using>>().cloned();
        if let Some(brought) = brought
            && let Ok(mut agent) = world.get_entity_mut(agent)
        {
            // The agent's own, set aside while the slot's runs; unless the
            // slot's is plugged in already.
            let own = agent.get::<Plugged<Using>>().cloned();
            if !own.as_ref().is_some_and(|own| own.same(&brought)) {
                agent.insert(brought.clone());
                self.swapped = Some((brought, own));
            }
        }
        self.held = Some(slot);
    }

    fn end(&mut self, cx: &mut Context, _outcome: Option<Outcome>) {
        let Some(slot) = self.held.take() else {
            return;
        };
        let claimed = std::mem::take(&mut self.claimed);
        let agent = cx.agent();
        let world = cx.world_mut();
        if claimed
            && let Ok(mut entity) = world.get_entity_mut(slot)
            && entity
                .get::<ClaimedBy>()
                .is_some_and(|claimed| claimed.agent() == agent)
        {
            entity.remove::<ClaimedBy>();
        }
        let Some((brought, own)) = self.swapped.take() else {
            return;
        };
        let Ok(mut agent) = world.get_entity_mut(agent) else {
            return;
        };
        // Only what the slot plugged in, if it's still there: one plugged in
        // since stays.
        if agent
            .get::<Plugged<Using>>()
            .is_some_and(|plugged| plugged.same(&brought))
        {
            match own {
                Some(own) => agent.insert(own),
                None => agent.remove::<Plugged<Using>>(),
            };
        }
    }

    fn label(&self) -> String {
        format!("claim::<{}>", ShortName::of::<Q>())
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use bevy_arbor_bt::prelude::*;
    use bevy_arbor_eqs::prelude::*;

    use crate::prelude::*;
    use crate::testing::*;

    #[test]
    fn an_agent_claims_the_seat_it_found_and_runs_what_the_seat_brings() {
        let mut app = app();
        let seats = bench(
            &mut app,
            &[Vec3::new(5.0, 0.0, 0.0), Vec3::new(9.0, 0.0, 0.0)],
        );
        let agent = sitter(&mut app);
        app.ticks(2);
        assert_eq!(held(&app, agent), Some(seats[0]), "the nearer");
        assert_eq!(
            app.world().get::<ClaimedBy>(seats[0]).map(ClaimedBy::agent),
            Some(agent)
        );
        assert_eq!(app.count::<With<GoThere>>(), 1, "on its way");
        arrive(&mut app);
        app.tick();
        assert_eq!(app.count::<With<Rest>>(), 1, "the bench's behavior");
    }

    #[test]
    fn a_seat_one_agent_holds_is_not_anothers() {
        let mut app = app();
        let seats = bench(&mut app, &[Vec3::new(5.0, 0.0, 0.0)]);
        let first = sitter(&mut app);
        let second = sitter(&mut app);
        app.ticks(4);
        let holders: Vec<_> = [first, second]
            .into_iter()
            .filter(|&agent| held(&app, agent) == Some(seats[0]))
            .collect();
        assert_eq!(holders.len(), 1, "one seat, one holder");
        assert_eq!(app.count::<With<GoThere>>(), 1);
    }

    #[test]
    fn two_seats_two_agents() {
        let mut app = app();
        let seats = bench(
            &mut app,
            &[Vec3::new(5.0, 0.0, 0.0), Vec3::new(6.0, 0.0, 0.0)],
        );
        let first = sitter(&mut app);
        let second = sitter(&mut app);
        app.ticks(6);
        let mut held: Vec<_> = [first, second]
            .into_iter()
            .filter_map(|agent| held(&app, agent))
            .collect();
        held.sort();
        let mut expected = seats.clone();
        expected.sort();
        assert_eq!(held, expected);
    }

    #[test]
    fn a_seat_disabled_while_held_is_given_up_and_its_behavior_goes() {
        let mut app = app();
        let seats = bench(&mut app, &[Vec3::new(5.0, 0.0, 0.0)]);
        let agent = sitter(&mut app);
        app.ticks(2);
        arrive(&mut app);
        app.tick();
        assert_eq!(app.count::<With<Rest>>(), 1);
        app.entity_mut(seats[0]).insert(SlotDisabled);
        app.tick();
        assert_eq!(held(&app, agent), None);
        assert!(app.world().get::<ClaimedBy>(seats[0]).is_none());
        assert!(
            app.world().get::<Plugged<Using>>(agent).is_none(),
            "unplugged"
        );
        assert_eq!(app.count::<With<Rest>>(), 0, "cancelled");
    }

    #[test]
    fn an_agent_going_gives_its_seat_up() {
        let mut app = app();
        let seats = bench(&mut app, &[Vec3::new(5.0, 0.0, 0.0)]);
        let agent = sitter(&mut app);
        app.ticks(2);
        app.entity_mut(agent).despawn();
        app.tick();
        assert!(app.world().get::<ClaimedBy>(seats[0]).is_none());
    }

    #[test]
    fn a_seat_going_while_held_cancels_and_unplugs() {
        let mut app = app();
        let seats = bench(&mut app, &[Vec3::new(5.0, 0.0, 0.0)]);
        let agent = sitter(&mut app);
        app.ticks(2);
        arrive(&mut app);
        app.tick();
        app.entity_mut(seats[0]).despawn();
        app.tick();
        assert!(app.world().get::<Plugged<Using>>(agent).is_none());
        assert_eq!(app.count::<With<Rest>>(), 0);
    }

    #[test]
    fn the_agents_own_is_put_back_once_the_seats_is_done() {
        fn own() -> Behavior {
            behavior_tree! { Wait(100.0) }
        }
        let mut app = app();
        let seats = bench(&mut app, &[Vec3::new(5.0, 0.0, 0.0)]);
        let agent = sitter(&mut app);
        let mine = Plugged::new(Using, own);
        app.entity_mut(agent).insert(mine.clone());
        app.ticks(2);
        assert_eq!(held(&app, agent), Some(seats[0]));
        let plugged = app.world().get::<Plugged<Using>>(agent).unwrap();
        assert!(!plugged.same(&mine), "the seat's, while it's held");
        app.entity_mut(seats[0]).insert(SlotDisabled);
        app.tick();
        let plugged = app.world().get::<Plugged<Using>>(agent).unwrap();
        assert!(plugged.same(&mine), "its own, back");
    }

    #[test]
    fn a_seat_bringing_nothing_leaves_the_agents_own_plugged_in() {
        fn own() -> Behavior {
            behavior_tree! { Wait(100.0) }
        }
        let mut app = app();
        let bench = app.spawn(GlobalTransform::IDENTITY);
        let seat = app.spawn((SlotOf(bench), Sit, GlobalTransform::from_xyz(5.0, 0.0, 0.0)));
        let agent = sitter(&mut app);
        app.entity_mut(agent).insert(Plugged::new(Using, own));
        app.ticks(2);
        assert_eq!(held(&app, agent), Some(seat));
        app.entity_mut(seat).insert(SlotDisabled);
        app.tick();
        assert!(
            app.world().get::<Plugged<Using>>(agent).is_some(),
            "its own stays"
        );
    }

    /// What an agent does while it holds what it wanted.
    #[derive(Component, Clone)]
    struct Idle;

    /// A second query, finding the same seats as [`Seat`].
    struct AnySeat;

    /// Reports every running `T` task as done.
    fn finish<T: Component>(app: &mut bevy_harness::TestApp) {
        let mut tasks = app.world_mut().query_filtered::<&mut Task, With<T>>();
        for mut task in tasks.iter_mut(app.world_mut()) {
            task.succeed();
        }
    }

    #[test]
    fn not_claiming_takes_no_one_elses_seat() {
        let mut app = app();
        let seats = bench(&mut app, &[Vec3::new(5.0, 0.0, 0.0)]);
        let owner = app.spawn(GlobalTransform::IDENTITY);
        let other = app.spawn((
            GlobalTransform::IDENTITY,
            BehaviorTree::new(behavior_tree! {
                Sequence => {
                    task(Survey::of(Seat)),
                    task(GoThere),
                    Select => { when(!claim::<Seat>()) => task(Idle) },
                }
            }),
        ));
        app.ticks(2);
        // Found free, then taken while it walks.
        app.entity_mut(seats[0]).insert(ClaimedBy(owner));
        arrive(&mut app);
        app.ticks(2);
        assert_eq!(app.count::<With<Idle>>(), 1, "taken, so `!claim` passes");
        assert_eq!(
            app.world().get::<ClaimedBy>(seats[0]).map(ClaimedBy::agent),
            Some(owner),
            "still the owner's"
        );
        assert_eq!(held(&app, other), None);
        assert!(
            app.world().get::<Plugged<Using>>(other).is_none(),
            "nor what it brings"
        );
    }

    #[test]
    fn a_seat_taken_from_its_agent_cancels_and_unplugs() {
        let mut app = app();
        let seats = bench(&mut app, &[Vec3::new(5.0, 0.0, 0.0)]);
        let agent = sitter(&mut app);
        app.ticks(2);
        arrive(&mut app);
        app.tick();
        assert_eq!(app.count::<With<Rest>>(), 1);
        app.entity_mut(seats[0]).remove::<ClaimedBy>();
        app.tick();
        assert_eq!(app.count::<With<Rest>>(), 0, "cancelled");
        assert!(
            app.world().get::<Plugged<Using>>(agent).is_none(),
            "unplugged"
        );
    }

    #[test]
    fn a_seat_claimed_twice_stays_held_until_the_first_claim_ends() {
        let mut app = app();
        app.app_mut().add_env_query(
            AnySeat,
            EnvQuery::new(FreeSlots::<Sit>::within(Agent, 20.0)),
        );
        let seats = bench(&mut app, &[Vec3::new(5.0, 0.0, 0.0)]);
        let agent = app.spawn((
            GlobalTransform::IDENTITY,
            BehaviorTree::new(behavior_tree! {
                Sequence => {
                    task(Survey::of(Seat)),
                    task(Survey::of(AnySeat)),
                    Select => {
                        when(claim::<Seat>()) => Sequence => {
                            Select => { when(claim::<AnySeat>()) => task(Idle) },
                            task(GoThere),
                        },
                    },
                }
            }),
        ));
        app.ticks(4);
        assert_eq!(app.count::<With<Idle>>(), 1);
        finish::<Idle>(&mut app);
        app.ticks(2);
        assert_eq!(app.count::<With<GoThere>>(), 1, "the first claim's branch");
        assert_eq!(held(&app, agent), Some(seats[0]), "still held");
        assert_eq!(app.count::<With<Rest>>(), 0);
        arrive(&mut app);
        app.ticks(2);
        assert_eq!(held(&app, agent), None, "given up as the first ends");
        assert!(app.world().get::<Plugged<Using>>(agent).is_none());
    }
}
