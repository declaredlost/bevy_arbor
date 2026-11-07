//! What wakes a sleeping tree: what its last tick left it waiting on.

use std::any::TypeId;
use std::time::Duration;

use bevy::ecs::change_detection::{CheckChangeTicks, Tick};
use bevy::prelude::*;

/// What a tree's last tick left it waiting on. It's ticked again once any of
/// it comes; until then it sleeps, and costs a look at each thing here.
#[derive(Default)]
pub(crate) struct Plan {
    /// Something in it has to be asked every tick.
    poll: bool,
    /// The soonest a timer in it is up, by the tree's clock.
    at: Option<Duration>,
    /// Components it waits on a change to.
    watches: Vec<Watch>,
}

/// A component on an entity, waited on to change, come or go.
struct Watch {
    entity: Entity,
    kind: TypeId,
    /// When it last changed as it was looked at: `None` if it wasn't there.
    seen: Option<Tick>,
    /// When it last changed now: `None` if it isn't there.
    changed: fn(&World, Entity) -> Option<Tick>,
}

impl Plan {
    /// Ticked again next tick, whatever else happens.
    pub(crate) fn poll(&mut self) {
        self.poll = true;
    }

    /// Ticked again once the tree's clock gets to `at`.
    pub(crate) fn wake_at(&mut self, at: Duration) {
        self.at = Some(self.at.map_or(at, |soonest| soonest.min(at)));
    }

    /// Ticked again once `T` on `entity` changes, comes or goes after now:
    /// what the tree did to it so far, it's seen.
    pub(crate) fn watch<T: Component>(&mut self, world: &World, entity: Entity) {
        let kind = TypeId::of::<T>();
        // Looked at twice, the first look counts: what came between, it hasn't
        // seen.
        if self
            .watches
            .iter()
            .any(|watch| watch.entity == entity && watch.kind == kind)
        {
            return;
        }
        self.watches.push(Watch {
            entity,
            kind,
            seen: changed::<T>(world, entity),
            changed: changed::<T>,
        });
    }

    /// Whether it waits on nothing at all.
    pub(crate) fn is_empty(&self) -> bool {
        !self.poll && self.at.is_none() && self.watches.is_empty()
    }

    /// Whether any of it has come, as of the change tick `now`, or the clock
    /// is past a timer. Without a clock, a timer always is.
    pub(crate) fn due(&self, world: &World, clock: Option<Duration>, now: Tick) -> bool {
        self.poll
            || self
                .at
                .is_some_and(|at| clock.is_none_or(|clock| clock >= at))
            || self.watches.iter().any(|watch| {
                match ((watch.changed)(world, watch.entity), watch.seen) {
                    (Some(changed), Some(seen)) => changed.is_newer_than(seen, now),
                    (None, None) => false,
                    // Came or went.
                    _ => true,
                }
            })
    }

    /// Keeps the change ticks it holds from getting too old.
    pub(crate) fn check_ticks(&mut self, check: CheckChangeTicks) {
        for seen in self
            .watches
            .iter_mut()
            .filter_map(|watch| watch.seen.as_mut())
        {
            seen.check_tick(check);
        }
    }
}

/// When `T` on `entity` last changed, if it's there.
fn changed<T: Component>(world: &World, entity: Entity) -> Option<Tick> {
    world
        .get_entity(entity)
        .ok()
        .and_then(|entity| entity.get_ref::<T>())
        .map(|value| value.last_changed())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use bevy::prelude::*;

    use crate::prelude::*;
    use crate::testing::*;

    static ASKED: AtomicUsize = AtomicUsize::new(0);

    fn high_and_counted(flags: &Flags) -> bool {
        ASKED.fetch_add(1, Ordering::Relaxed);
        flags.high
    }

    fn is_high(flags: &Flags) -> bool {
        flags.high
    }

    fn is_low(flags: &Flags) -> bool {
        flags.low
    }

    #[test]
    fn a_tree_under_typed_guards_sleeps_until_what_they_read_changes() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behaviour_tree! {
                Select => {
                    when(high_and_counted) => task("Work", Work),
                    task("Other", Other),
                }
            },
        );
        app.tick();
        let asked = ASKED.load(Ordering::Relaxed);
        app.ticks(5);
        assert_eq!(ASKED.load(Ordering::Relaxed), asked, "asleep: not asked");
        assert_eq!(app.count::<With<Other>>(), 1);

        set(&mut app, npc, true, false);
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 1, "woken by the change");
        assert_eq!(app.count::<With<Other>>(), 0);
    }

    #[derive(Component)]
    struct Seen;

    fn sees(_: &Seen) -> bool {
        true
    }

    #[test]
    fn a_typed_guard_on_a_missing_component_fails_until_it_comes() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behaviour_tree! {
                Select => {
                    when(sees) => task("Work", Work),
                    task("Other", Other),
                }
            },
        );
        app.tick();
        assert_eq!(app.count::<With<Other>>(), 1);
        app.entity_mut(npc).insert(Seen);
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 1, "woken as it came");
        app.entity_mut(npc).remove::<Seen>();
        app.tick();
        assert_eq!(app.count::<With<Other>>(), 1, "woken as it went");
    }

    fn calm(seen: Option<&Seen>) -> bool {
        seen.is_none()
    }

    #[test]
    fn a_typed_guard_can_take_a_component_that_may_be_missing() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behaviour_tree! {
                Select => {
                    when(calm) => task("Work", Work),
                    task("Other", Other),
                }
            },
        );
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 1);
        app.entity_mut(npc).insert(Seen);
        app.tick();
        assert_eq!(app.count::<With<Other>>(), 1);
    }

    #[test]
    fn typed_guards_combine_as_written() {
        let mut app = app();
        let tree = || {
            behaviour_tree! {
                Select => {
                    when(is_high && !is_low) => act(count),
                }
            }
        };
        let yes = agent(&mut app, tree());
        set(&mut app, yes, true, false);
        let no = agent(&mut app, tree());
        set(&mut app, no, true, true);
        app.tick();
        assert_eq!(outcome(&app, yes), Some(true));
        assert_eq!(outcome(&app, no), Some(false));
    }

    #[test]
    fn a_typed_while_stops_its_child_once_what_it_reads_changes() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behaviour_tree! { While(is_high) => { task("Work", Work) } },
        );
        set(&mut app, npc, true, false);
        app.ticks(3);
        assert_eq!(app.count::<With<Work>>(), 1);
        set(&mut app, npc, false, false);
        app.tick();
        assert_eq!(outcome(&app, npc), Some(true));
        assert_eq!(app.count::<With<Work>>(), 0);
    }

    fn make_low(In(agent): In<Entity>, mut flags: Query<&mut Flags>) -> bool {
        flags.get_mut(agent).is_ok_and(|mut flags| {
            flags.low = true;
            true
        })
    }

    #[test]
    fn a_change_made_after_a_guard_looked_wakes_the_tree_next_tick() {
        let mut app = app();
        agent(
            &mut app,
            behaviour_tree! {
                Select => {
                    when(is_low) => task("Other", Other),
                    Sequence => {
                        act(make_low),
                        task("Work", Work),
                    },
                }
            },
        );
        app.tick();
        assert_eq!(
            app.count::<With<Work>>(),
            1,
            "the guard said no, then the act ran"
        );
        app.tick();
        assert_eq!(
            app.count::<With<Other>>(),
            1,
            "woken by what the act changed"
        );
        assert_eq!(app.count::<With<Work>>(), 0);
    }

    #[test]
    fn a_paused_tree_sees_what_changed_while_it_was_paused() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behaviour_tree! {
                Select => {
                    when(is_high) => task("Work", Work),
                    task("Other", Other),
                }
            },
        );
        app.tick();
        app.entity_mut(npc).insert(TreePaused);
        set(&mut app, npc, true, false);
        app.ticks(2);
        assert_eq!(app.count::<With<Other>>(), 1, "paused");
        app.entity_mut(npc).remove::<TreePaused>();
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 1, "caught up");
    }
}
