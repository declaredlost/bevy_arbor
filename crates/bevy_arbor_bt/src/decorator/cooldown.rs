use std::fmt;
use std::marker::PhantomData;
use std::time::Duration;

use bevy::prelude::*;
use bevy::utils::prelude::ShortName;

use super::decorate::Decorate;
use crate::behavior::{Context, Outcome};

/// A decorator keeping its branch from running again until `length` after it
/// last ended, however it ended: `when(has_target && cooldown(8.0))`.
///
/// Its tree sleeps while it cools, and is woken as it's done: a `Select` then
/// switches back to its branch as it would to any decorator passing again. Each
/// agent cools on its own.
#[derive(Reflect, Debug, Clone, PartialEq)]
#[reflect(Debug, PartialEq)]
pub struct Cooldown {
    length: Duration,
    /// When it's cooled, while it cools.
    #[reflect(ignore)]
    until: Option<Duration>,
}

impl Cooldown {
    /// Cooling for `length` after its branch ends.
    #[must_use]
    pub fn new(length: Duration) -> Self {
        Self {
            length,
            until: None,
        }
    }

    /// How long it cools.
    #[must_use]
    pub fn length(&self) -> Duration {
        self.length
    }
}

/// A [`Cooldown`] of `seconds`: none, if it's 0 or less.
#[must_use]
pub fn cooldown(seconds: f32) -> Cooldown {
    Cooldown::new(Duration::try_from_secs_f32(seconds).unwrap_or(Duration::ZERO))
}

impl Decorate for Cooldown {
    fn passes(&mut self, cx: &mut Context) -> bool {
        // At rest, only its branch ending starts it cooling: nothing to wake
        // for.
        let Some(until) = self.until else {
            cx.wait_on_nothing();
            return true;
        };
        if cx.now() >= until {
            self.until = None;
            cx.wait_on_nothing();
            return true;
        }
        cx.wake_at(until);
        false
    }

    fn end(&mut self, cx: &mut Context, _outcome: Option<Outcome>) {
        self.until = Some(cx.now().saturating_add(self.length));
    }

    fn label(&self) -> String {
        format!("cooldown({}s)", self.length.as_secs_f32())
    }
}

/// On an agent: when its shared cooldown `K` is over, by the clock its tree
/// ticks on. Written by every [`cooldown_of::<K>`](cooldown_of) as its branch
/// ends, and read by all of them; insert one yourself to start it from
/// elsewhere, as a stun or a disarm might.
#[derive(Component)]
pub struct Cooling<K: Send + Sync + 'static> {
    until: Duration,
    key: PhantomData<fn() -> K>,
}

impl<K: Send + Sync + 'static> Cooling<K> {
    /// Cooling until `until`, by the clock trees tick on.
    #[must_use]
    pub fn until(until: Duration) -> Self {
        Self {
            until,
            key: PhantomData,
        }
    }

    /// When it's over.
    #[must_use]
    pub fn over_at(&self) -> Duration {
        self.until
    }
}

impl<K: Send + Sync + 'static> fmt::Debug for Cooling<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(&format!("Cooling<{}>", ShortName::of::<K>()))
            .field("until", &self.until)
            .finish()
    }
}

/// A decorator sharing its cooldown with every other one keyed by `K` on the
/// agent. Made by [`cooldown_of`].
pub struct SharedCooldown<K: Send + Sync + 'static> {
    length: Duration,
    key: PhantomData<fn() -> K>,
}

/// A cooldown of `seconds` shared by every branch keyed by `K`, on each agent:
/// `when(cooldown_of::<Grenades>(8.0))` on the fire bomb and the frag
/// grenade alike, so throwing either cools both. None, if it's 0 or less.
#[must_use]
pub fn cooldown_of<K: Send + Sync + 'static>(seconds: f32) -> SharedCooldown<K> {
    SharedCooldown {
        length: Duration::try_from_secs_f32(seconds).unwrap_or(Duration::ZERO),
        key: PhantomData,
    }
}

impl<K: Send + Sync + 'static> Decorate for SharedCooldown<K> {
    fn passes(&mut self, cx: &mut Context) -> bool {
        let agent = cx.agent();
        // Another branch, or anything else, starting it wakes the tree.
        cx.watch::<Cooling<K>>(agent);
        let Some(until) = cx.world().get::<Cooling<K>>(agent).map(Cooling::over_at) else {
            return true;
        };
        if cx.now() >= until {
            return true;
        }
        cx.wake_at(until);
        false
    }

    fn end(&mut self, cx: &mut Context, _outcome: Option<Outcome>) {
        let agent = cx.agent();
        let until = cx.now().saturating_add(self.length);
        let Ok(mut entity) = cx.world_mut().get_entity_mut(agent) else {
            return;
        };
        // Never shortened: one cooling longer stays.
        if entity
            .get::<Cooling<K>>()
            .is_none_or(|cooling| cooling.until < until)
        {
            entity.insert(Cooling::<K>::until(until));
        }
    }

    fn label(&self) -> String {
        format!(
            "cooldown_of::<{}>({}s)",
            ShortName::of::<K>(),
            self.length.as_secs_f32()
        )
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::prelude::*;

    use super::{Cooling, cooldown, cooldown_of};
    use crate::prelude::*;
    use crate::testing::*;

    fn grenade_or_shoot() -> Behavior {
        behavior_tree! {
            Forever => {
                Select => {
                    when(high && cooldown(1.0)) => task((Work, Report::Succeed)),
                    task(Other),
                }
            }
        }
    }

    #[test]
    fn its_branch_waits_out_the_cooldown_then_runs_again() {
        let mut app = app();
        let npc = agent(&mut app, grenade_or_shoot());
        set(&mut app, npc, true, false);
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 1, "thrown");
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 0, "reported");
        assert_eq!(app.count::<With<Other>>(), 1, "cooling: shooting");
        app.secs(0.5);
        assert_eq!(works_started(&app), 1, "still cooling");
        assert_eq!(app.count::<With<Other>>(), 1);
        app.secs(0.6);
        assert_eq!(works_started(&app), 2, "cooled: thrown again");
    }

    #[test]
    fn it_cools_after_a_failure_too() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behavior_tree! {
                Select => {
                    when(high && cooldown(1.0)) => task((Work, Report::Fail)),
                    task(Other),
                }
            },
        );
        set(&mut app, npc, true, false);
        app.ticks(3);
        assert_eq!(works_started(&app), 1, "failed, and cooling");
        assert_eq!(app.count::<With<Other>>(), 1);
        app.secs(1.1);
        assert_eq!(works_started(&app), 2, "cooled");
    }

    #[test]
    fn it_cools_after_its_branch_is_cancelled_too() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behavior_tree! {
                Select => {
                    when(high && cooldown(1.0)) => task(Work),
                    task(Other),
                }
            },
        );
        set(&mut app, npc, true, false);
        app.tick();
        set(&mut app, npc, false, false);
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 0, "cancelled");
        set(&mut app, npc, true, false);
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 0, "cooling");
        app.secs(1.1);
        assert_eq!(app.count::<With<Work>>(), 1, "cooled");
    }

    #[test]
    fn a_cooled_branch_takes_over_from_a_lower_one() {
        let mut app = app();
        agent(
            &mut app,
            behavior_tree! {
                Forever => {
                    Select => {
                        when(cooldown(1.0)) => act(count),
                        Wait(100.0),
                    }
                }
            },
        );
        app.tick();
        assert_eq!(acts(&app), 1);
        app.secs(0.5);
        assert_eq!(acts(&app), 1, "cooling, waiting");
        app.secs(0.6);
        assert_eq!(acts(&app), 2, "woken as it cooled, and switched");
    }

    #[test]
    fn cooldown_takes_seconds_and_none_for_nothing() {
        assert_eq!(cooldown(1.5).length(), Duration::from_millis(1500));
        assert_eq!(cooldown(-1.0).length(), Duration::ZERO);
        assert_eq!(cooldown(f32::NAN).length(), Duration::ZERO);
    }

    struct Grenades;

    #[test]
    fn a_shared_cooldown_cools_every_branch_keyed_alike() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behavior_tree! {
                Forever => {
                    Select => {
                        when(high && cooldown_of::<Grenades>(1.0)) => act(count),
                        when(cooldown_of::<Grenades>(1.0)) => task((Work, Report::Succeed)),
                        task(Other),
                    }
                }
            },
        );
        set(&mut app, npc, true, false);
        app.tick();
        let thrown = acts(&app);
        assert!(thrown > 0, "the first thrown");
        set(&mut app, npc, false, false);
        app.ticks(3);
        assert_eq!(works_started(&app), 0, "the second cooling too");
        assert_eq!(app.count::<With<Other>>(), 1);
        app.secs(1.1);
        assert!(works_started(&app) > 0, "both cooled");
    }

    #[test]
    fn a_shared_cooldown_started_from_elsewhere_holds_its_branches() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behavior_tree! {
                Forever => {
                    Select => {
                        when(cooldown_of::<Grenades>(1.0)) => act(count),
                        Wait(100.0),
                    }
                }
            },
        );
        app.entity_mut(npc)
            .insert(Cooling::<Grenades>::until(Duration::from_secs(1)));
        app.tick();
        assert_eq!(acts(&app), 0, "cooling, though nothing in the tree ended");
        app.secs(1.1);
        assert!(acts(&app) > 0, "and woken as it's over");
    }

    #[test]
    fn a_shared_cooldown_is_not_shortened() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behavior_tree! {
                Select => {
                    when(cooldown_of::<Grenades>(1.0)) => task(Work),
                    Wait(100.0),
                }
            },
        );
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 1);
        // Started from elsewhere, longer: the branch is cancelled, and ending
        // it would cool for a second only.
        app.entity_mut(npc)
            .insert(Cooling::<Grenades>::until(Duration::from_secs(5)));
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 0, "cancelled");
        let until = app.world().get::<Cooling<Grenades>>(npc).unwrap().over_at();
        assert_eq!(until, Duration::from_secs(5), "kept the longer");
    }

    /// How often `asked` has been asked.
    static ASKED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    #[derive(Component)]
    struct Here;

    fn asked(_: &Here) -> bool {
        ASKED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        true
    }

    #[test]
    fn a_tree_with_a_cooldown_at_rest_still_sleeps() {
        let mut app = app();
        app.spawn((
            Here,
            BehaviorTree::new(behavior_tree! {
                Select => { when(asked && cooldown(1.0)) => Wait(100.0) }
            }),
        ));
        app.ticks(10);
        assert_eq!(
            ASKED.load(std::sync::atomic::Ordering::Relaxed),
            1,
            "asked once, then asleep"
        );
    }
}
