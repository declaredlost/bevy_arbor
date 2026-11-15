//! What wakes a sleeping tree: what its last tick left it waiting on, and the
//! index that finds, each tick, the trees whose wait may be over.
//!
//! A sleeping tree isn't looked at. What it watches is indexed by component
//! type: a tick looks once for what changed of each type watched, and only the
//! trees watching what did are asked whether their wait is over. Removals are
//! heard by an observer of each type, timers kept in a heap.

use std::any::TypeId;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::time::Duration;

use bevy::ecs::change_detection::{CheckChangeTicks, Tick};
use bevy::ecs::entity::EntityHashMap;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

/// Who in a tree waits on something: a decorator part, by its id, or
/// [`NODE`] for a node, which is ticked whenever its tree is.
pub(crate) type Owner = u32;

/// What a node waits on: not tied to a decorator part.
pub(crate) const NODE: Owner = 0;

/// What a tree's last tick left it waiting on, each thing with who waits on
/// it. It's ticked again once any of it comes.
#[derive(Default)]
pub(crate) struct Plan {
    /// Who has to be asked every tick.
    polls: Vec<Owner>,
    /// Timers, by the tree's clock, and whose.
    timers: Vec<(Owner, Duration)>,
    /// Components waited on a change to.
    watches: Vec<Watch>,
}

/// A component on an entity, waited on to change, come or go.
#[derive(Clone, Copy)]
struct Watch {
    owner: Owner,
    entity: Entity,
    kind: TypeId,
    /// When it last changed as it was looked at: `None` if it wasn't there.
    seen: Option<Tick>,
    /// When it last changed now: `None` if it isn't there.
    changed: fn(&World, Entity) -> Option<Tick>,
    /// Makes the index for its type, the first time one's watched.
    index: fn(&mut World) -> Kind,
}

impl Watch {
    /// Whether it's on the same component of the same entity as `other`.
    fn same(&self, other: &Self) -> bool {
        self.entity == other.entity && self.kind == other.kind
    }

    /// Whether it changed, came or went since it was seen, as of `now`.
    fn come(&self, world: &World, now: Tick) -> bool {
        match ((self.changed)(world, self.entity), self.seen) {
            (Some(changed), Some(seen)) => changed.is_newer_than(seen, now),
            (None, None) => false,
            // Came or went.
            _ => true,
        }
    }
}

impl Plan {
    /// `owner` ticked again next tick, whatever else happens.
    pub(crate) fn poll(&mut self, owner: Owner) {
        if !self.polls.contains(&owner) {
            self.polls.push(owner);
        }
    }

    /// Whether it's to be ticked again next tick, whatever else happens.
    pub(crate) fn is_poll(&self) -> bool {
        !self.polls.is_empty()
    }

    /// `owner` ticked again once the tree's clock gets to `at`.
    pub(crate) fn wake_at(&mut self, owner: Owner, at: Duration) {
        match self.timers.iter_mut().find(|(who, _)| *who == owner) {
            Some((_, soonest)) => *soonest = (*soonest).min(at),
            None => self.timers.push((owner, at)),
        }
    }

    /// The soonest a timer in it is up.
    fn at(&self) -> Option<Duration> {
        self.timers.iter().map(|(_, at)| *at).min()
    }

    /// `owner` ticked again once `T` on `entity` changes, comes or goes after
    /// now: what the tree did to it so far, it's seen.
    pub(crate) fn watch<T: Component>(&mut self, owner: Owner, world: &World, entity: Entity) {
        let kind = TypeId::of::<T>();
        // Looked at twice, the first look counts: what came between, it hasn't
        // seen.
        if self
            .watches
            .iter()
            .any(|watch| watch.owner == owner && watch.entity == entity && watch.kind == kind)
        {
            return;
        }
        self.watches.push(Watch {
            owner,
            entity,
            kind,
            seen: changed::<T>(world, entity),
            changed: changed::<T>,
            index: Kind::of::<T>,
        });
    }

    /// Carries over what `owner` waited on in `before`, as it was: it wasn't
    /// asked again, so waits on the same, seen as then.
    pub(crate) fn keep(&mut self, before: &Self, owner: Owner) {
        if before.polls.contains(&owner) {
            self.poll(owner);
        }
        for &(who, at) in &before.timers {
            if who == owner {
                self.wake_at(owner, at);
            }
        }
        for watch in before.watches.iter().filter(|watch| watch.owner == owner) {
            if !self
                .watches
                .iter()
                .any(|kept| kept.owner == owner && kept.same(watch))
            {
                self.watches.push(*watch);
            }
        }
    }

    /// Whether any of it has come, as of the change tick `now`, or the clock
    /// is past a timer, putting in `fired` who it came for. Without a clock, a
    /// timer always is.
    pub(crate) fn due(
        &self,
        world: &World,
        clock: Option<Duration>,
        now: Tick,
        fired: &mut Vec<Owner>,
    ) -> bool {
        let start = fired.len();
        fired.extend_from_slice(&self.polls);
        for &(owner, at) in &self.timers {
            if clock.is_none_or(|clock| clock >= at) {
                fired.push(owner);
            }
        }
        for watch in &self.watches {
            if !fired[start..].contains(&watch.owner) && watch.come(world, now) {
                fired.push(watch.owner);
            }
        }
        fired.len() > start
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

/// Who watches what, and who may be due: the index trees sleep in.
#[derive(Resource)]
pub(crate) struct Wakes {
    /// Each component type watched, by its `TypeId`.
    kinds: HashMap<TypeId, Kind>,
    /// Timers: when, by the trees' clock, and whose.
    timers: BinaryHeap<Reverse<(Duration, Entity)>>,
    /// Trees to look at next tick: asking every tick, new, restarted, or
    /// unpaused.
    next: Vec<Entity>,
    /// Trees that finished: looked at each tick, for one restarted.
    finished: Vec<Entity>,
    /// Watched components taken off their entities since the last look.
    removed: Vec<(TypeId, Entity)>,
    /// Up to the change tick it last looked for changes at.
    looked: Tick,
}

impl FromWorld for Wakes {
    fn from_world(world: &mut World) -> Self {
        Self {
            kinds: HashMap::default(),
            timers: BinaryHeap::new(),
            next: Vec::new(),
            finished: Vec::new(),
            removed: Vec::new(),
            looked: Tick::new(world.change_tick().get().wrapping_sub(1)),
        }
    }
}

/// Trees watching one component type, and how to find which of its entities
/// changed.
pub(crate) struct Kind {
    /// The trees watching it, by the entity it's on.
    watchers: EntityHashMap<Vec<Entity>>,
    scan: Box<dyn Scan>,
}

impl Kind {
    /// The index for `T`, hearing it taken away from now on.
    fn of<T: Component>(world: &mut World) -> Self {
        world.add_observer(removed::<T>);
        Self {
            watchers: EntityHashMap::default(),
            scan: Box::new(Scanner::<T>(QueryState::new(world))),
        }
    }
}

/// Hears `T` taken off an entity, for its watchers to be looked at.
fn removed<T: Component>(removed: On<Remove<T>>, mut wakes: ResMut<Wakes>) {
    let kind = TypeId::of::<T>();
    if wakes
        .kinds
        .get(&kind)
        .is_some_and(|index| index.watchers.contains_key(&removed.entity))
    {
        wakes.removed.push((kind, removed.entity));
    }
}

/// Finds what changed of one component type.
trait Scan: Send + Sync {
    /// Puts in `due` the trees in `watchers` watching an entity whose component
    /// changed since `since`, as of `now`.
    fn changed(
        &mut self,
        world: &World,
        watchers: &EntityHashMap<Vec<Entity>>,
        since: Tick,
        now: Tick,
        due: &mut Vec<Entity>,
    );
}

/// The [`Scan`] of `T`.
struct Scanner<T: Component>(QueryState<(Entity, Ref<'static, T>)>);

impl<T: Component> Scan for Scanner<T> {
    fn changed(
        &mut self,
        world: &World,
        watchers: &EntityHashMap<Vec<Entity>>,
        since: Tick,
        now: Tick,
        due: &mut Vec<Entity>,
    ) {
        let all = self.0.iter(world);
        // Few watched among many: each looked up. Else the storage passed
        // through once, which costs far less an entity.
        if watchers.len().saturating_mul(8) < all.len() {
            for (entity, watching) in watchers {
                if changed::<T>(world, *entity).is_some_and(|tick| tick.is_newer_than(since, now)) {
                    due.extend_from_slice(watching);
                }
            }
            return;
        }
        for (entity, value) in all {
            if value.last_changed().is_newer_than(since, now)
                && let Some(watching) = watchers.get(&entity)
            {
                due.extend_from_slice(watching);
            }
        }
    }
}

impl Wakes {
    /// The trees that may be due a tick, as of the trees' `clock` and the
    /// change tick `now`, into `due`: each to be asked whether it is. Some may
    /// come more than once.
    pub(crate) fn due(
        &mut self,
        world: &World,
        clock: Option<Duration>,
        now: Tick,
        due: &mut Vec<Entity>,
    ) {
        due.append(&mut self.next);
        // What trees change as they tick is stamped `now` too, after this
        // look: the next one counts it, and what each tree saw keeps it from
        // waking it again.
        let since = std::mem::replace(&mut self.looked, Tick::new(now.get().wrapping_sub(1)));
        for index in self.kinds.values_mut() {
            index.scan.changed(world, &index.watchers, since, now, due);
        }
        for (kind, entity) in self.removed.drain(..) {
            if let Some(watching) = self
                .kinds
                .get(&kind)
                .and_then(|index| index.watchers.get(&entity))
            {
                due.extend_from_slice(watching);
            }
        }
        while let Some(Reverse((at, agent))) = self.timers.peek().copied() {
            // Without a clock, a timer always is up.
            if clock.is_some_and(|clock| clock < at) {
                break;
            }
            self.timers.pop();
            due.push(agent);
        }
        // Finished, unless restarted: then fresh, and off the list.
        self.finished
            .retain(|&agent| match world.get::<super::BehaviorTree>(agent) {
                Some(tree) if tree.is_fresh() => {
                    due.push(agent);
                    false
                }
                Some(tree) => tree.outcome().is_some(),
                None => false,
            });
    }

    /// Puts `agent`'s tree to sleep on `plan`, having slept on `before`:
    /// indexes what it waits on now and didn't then, and forgets what it
    /// waited on then and doesn't now. Mostly, it's the same. One asking every
    /// tick goes in `next`, for [`Self::looked_at`] to hand over.
    pub(crate) fn sleep(
        world: &mut World,
        agent: Entity,
        before: Option<&Plan>,
        plan: &Plan,
        next: &mut Vec<Entity>,
    ) {
        let same = Watch::same;
        let empty: &[Watch] = &[];
        let old = before.map_or(empty, |before| &before.watches);
        if plan.is_poll() {
            next.push(agent);
        }
        for watch in &plan.watches {
            if !old.iter().any(|old| same(old, watch))
                && !world.resource::<Self>().kinds.contains_key(&watch.kind)
            {
                let index = (watch.index)(world);
                world.resource_mut::<Self>().kinds.insert(watch.kind, index);
            }
        }
        let mut wakes = world.resource_mut::<Self>();
        // Each component of each entity once, however many in the tree watch
        // it.
        for (at, watch) in old.iter().enumerate() {
            if !old[..at].iter().any(|earlier| same(earlier, watch))
                && !plan.watches.iter().any(|new| same(new, watch))
            {
                wakes.unwatch(agent, watch);
            }
        }
        for (at, watch) in plan.watches.iter().enumerate() {
            if !plan.watches[..at]
                .iter()
                .any(|earlier| same(earlier, watch))
                && !old.iter().any(|old| same(old, watch))
                && let Some(index) = wakes.kinds.get_mut(&watch.kind)
            {
                index.watchers.entry(watch.entity).or_default().push(agent);
            }
        }
        // One already in the heap stays good: none for the same time again.
        if let Some(at) = plan.at()
            && before.and_then(Plan::at) != Some(at)
        {
            wakes.timers.push(Reverse((at, agent)));
        }
    }

    /// Whether `plan` waits on just what `before` did, so sleeping on it
    /// changes nothing in the index: most often so.
    pub(crate) fn same(before: Option<&Plan>, plan: &Plan) -> bool {
        let empty: &[Watch] = &[];
        let old = before.map_or(empty, |before| &before.watches);
        old.len() == plan.watches.len()
            && old.iter().zip(&plan.watches).all(|(a, b)| a.same(b))
            && before.and_then(Plan::at) == plan.at()
    }

    /// Hands over the trees asking to be looked at next tick, gathered as
    /// trees went to sleep.
    pub(crate) fn looked_at(&mut self, next: &mut Vec<Entity>) {
        self.next.append(next);
    }

    /// Forgets what `agent`'s tree waited on, by `plan`: it's gone, or
    /// finished. A timer stays in the heap, and has it looked at for nothing
    /// at worst.
    pub(crate) fn forget(&mut self, agent: Entity, plan: &Plan) {
        for (at, watch) in plan.watches.iter().enumerate() {
            if !plan.watches[..at].iter().any(|earlier| earlier.same(watch)) {
                self.unwatch(agent, watch);
            }
        }
    }

    /// Takes `agent` off the watchers of `watch`.
    fn unwatch(&mut self, agent: Entity, watch: &Watch) {
        let Some(index) = self.kinds.get_mut(&watch.kind) else {
            return;
        };
        if let Some(watching) = index.watchers.get_mut(&watch.entity) {
            if let Some(at) = watching.iter().position(|&watcher| watcher == agent) {
                watching.swap_remove(at);
            }
            if watching.is_empty() {
                index.watchers.remove(&watch.entity);
            }
        }
    }

    /// Looks at `agent`'s tree next tick: it's new, restarted or unpaused.
    pub(crate) fn wake(&mut self, agent: Entity) {
        self.next.push(agent);
    }

    /// Looks at `agent`'s finished tree each tick, for it being restarted.
    pub(crate) fn finished(&mut self, agent: Entity) {
        self.finished.push(agent);
    }

    /// Keeps the change tick it holds from getting too old.
    pub(crate) fn check_ticks(&mut self, check: CheckChangeTicks) {
        self.looked.check_tick(check);
    }
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
    fn a_tree_under_typed_conditions_sleeps_until_what_they_read_changes() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behavior_tree! {
                Select => {
                    when(high_and_counted) => task(Work),
                    task(Other),
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
    fn a_typed_condition_on_a_missing_component_fails_until_it_comes() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behavior_tree! {
                Select => {
                    when(sees) => task(Work),
                    task(Other),
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
    fn a_typed_condition_can_take_a_component_that_may_be_missing() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behavior_tree! {
                Select => {
                    when(calm) => task(Work),
                    task(Other),
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
    fn typed_conditions_combine_as_written() {
        let mut app = app();
        let tree = || {
            behavior_tree! {
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
            behavior_tree! { RepeatWhile(is_high) => { task(Work) } },
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
    fn a_change_made_after_a_decorator_looked_wakes_the_tree_next_tick() {
        let mut app = app();
        agent(
            &mut app,
            behavior_tree! {
                Select => {
                    when(is_low) => task(Other),
                    Sequence => {
                        act(make_low),
                        task(Work),
                    },
                }
            },
        );
        app.tick();
        assert_eq!(
            app.count::<With<Work>>(),
            1,
            "the decorator said no, then the act ran"
        );
        app.tick();
        assert_eq!(
            app.count::<With<Other>>(),
            1,
            "woken by what the act changed"
        );
        assert_eq!(app.count::<With<Work>>(), 0);
    }

    #[derive(Component)]
    struct Level(u32);

    #[test]
    fn a_change_to_one_of_many_is_heard_whether_looked_up_or_passed_over() {
        let mut app = app();
        // Many with `Level`, one watched: it's looked up, not passed over.
        let watched = app.spawn(Level(0));
        for _ in 0..100 {
            app.spawn(Level(0));
        }
        app.entity_mut(watched).insert(Level(0));
        let npc = agent(
            &mut app,
            behavior_tree! {
                Select => {
                    when(move |cx: &mut crate::behavior::Context| {
                        cx.watch::<Level>(watched);
                        cx.world().get::<Level>(watched).is_some_and(|level| level.0 > 0)
                    }) => task(Work),
                    task(Other),
                }
            },
        );
        app.ticks(2);
        assert_eq!(app.count::<With<Other>>(), 1);
        app.entity_mut(watched).insert(Level(1));
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 1, "heard, looked up");
        // And despawned: heard by its observer.
        app.world_mut().despawn(watched);
        app.tick();
        assert_eq!(app.count::<With<Other>>(), 1, "heard it go");
        assert!(app.world().get::<BehaviorTree>(npc).is_some());
    }

    #[test]
    fn a_paused_tree_sees_what_changed_while_it_was_paused() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behavior_tree! {
                Select => {
                    when(is_high) => task(Work),
                    task(Other),
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
