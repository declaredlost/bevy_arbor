//! What wakes a sleeping tree: what its last tick left it waiting on.
//!
//! An index finds, each tick, the trees whose wait may be over.
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
use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use super::BehaviorTree;
use crate::task::{Task, TaskOf};

/// Who in a tree waits on something: a decorator part, by its id, or a node.
///
/// A node is [`NODE`], and is ticked whenever its tree is.
pub(crate) type Owner = u32;

/// What a node waits on: not tied to a decorator part.
pub(crate) const NODE: Owner = 0;

/// A set of owners, as bits: a tree's first 127 decorator parts, and its nodes.
///
/// A part past those isn't in any set: it's asked whenever its tree ticks, as
/// one never known to have its answer standing.
#[derive(Default, Clone, Copy)]
pub(crate) struct Owners(u128);

impl Owners {
    /// None.
    pub(crate) const EMPTY: Self = Self(0);

    /// Just `owner`.
    fn of(owner: Owner) -> Self {
        Self(Self::bit(owner))
    }

    /// The owners in both it and `other`.
    fn and(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    /// The bit of `owner`, if it has one.
    fn bit(owner: Owner) -> u128 {
        1u128.checked_shl(owner).unwrap_or(0)
    }

    /// Adds `owner`.
    pub(crate) fn insert(&mut self, owner: Owner) {
        self.0 |= Self::bit(owner);
    }

    /// Whether `owner` is in it: always, for one past the first 127, so it's
    /// asked again.
    pub(crate) fn contains(self, owner: Owner) -> bool {
        owner >= 128 || self.0 & Self::bit(owner) != 0
    }

    /// Whether it's empty.
    pub(crate) fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Empties it.
    pub(crate) fn clear(&mut self) {
        self.0 = 0;
    }

    /// Adds every owner in `other`.
    fn extend(&mut self, other: Self) {
        self.0 |= other.0;
    }

    /// Whether any owner in it isn't in `fired`.
    ///
    /// A node counts as in it once anything is, as nodes are ticked whenever
    /// their tree is.
    fn beyond(self, fired: Self) -> bool {
        let fired = if fired.is_empty() {
            0
        } else {
            fired.0 | Self::bit(NODE)
        };
        self.0 & !fired != 0
    }
}

/// What a tree's last tick left it waiting on, each thing with who waits on it.
///
/// It's ticked again once any of it comes.
#[derive(Default)]
pub(crate) struct Plan {
    /// Who has to be asked every tick.
    polls: Owners,
    /// Timers, by the tree's clock, and whose.
    timers: Vec<(Owner, Duration)>,
    /// Components waited on a change to.
    watches: Vec<Watch>,
}

/// A component on an entity waited on to change, come or go, and who waits on
/// it.
///
/// It's once in a plan, however many wait on it.
#[derive(Clone, Copy)]
struct Watch {
    owners: Owners,
    entity: Entity,
    kind: TypeId,
    /// When it last changed as it was first looked at: `None` if it wasn't
    /// there.
    seen: Option<Tick>,
    /// When it last changed now: `None` if it isn't there.
    changed: fn(&World, Entity) -> Option<Tick>,
    /// Makes the index for its type, the first time one's watched.
    index: fn(&mut World) -> Index,
}

impl Watch {
    /// Whether it's on the same component of the same entity as `other`.
    fn same(&self, other: &Self) -> bool {
        self.is_on(other.entity, other.kind)
    }

    /// Whether it's on the component `kind` of `entity`.
    fn is_on(&self, entity: Entity, kind: TypeId) -> bool {
        self.entity == entity && self.kind == kind
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
        self.polls.insert(owner);
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
    /// now.
    ///
    /// What the tree did to it so far, it's seen.
    pub(crate) fn watch<T: Component>(&mut self, owner: Owner, world: &World, entity: Entity) {
        let kind = TypeId::of::<T>();
        // Looked at twice, the first look counts: what came between, it hasn't
        // seen.
        if let Some(watch) = self
            .watches
            .iter_mut()
            .find(|watch| watch.is_on(entity, kind))
        {
            watch.owners.insert(owner);
            return;
        }
        self.watches.push(Watch {
            owners: Owners::of(owner),
            entity,
            kind,
            seen: changed::<T>(world, entity),
            changed: changed::<T>,
            index: Index::of::<T>,
        });
    }

    /// Carries over what `kept` waited on in `before`, as it was.
    ///
    /// They weren't asked again, so wait on the same, seen as then.
    pub(crate) fn keep(&mut self, before: &Self, kept: Owners) {
        if kept.is_empty() {
            return;
        }
        self.polls.extend(before.polls.and(kept));
        for &(owner, at) in &before.timers {
            if kept.contains(owner) {
                self.wake_at(owner, at);
            }
        }
        for watch in &before.watches {
            let owners = watch.owners.and(kept);
            if owners.is_empty() {
                continue;
            }
            match self.watches.iter_mut().find(|now| now.same(watch)) {
                Some(now) => {
                    now.owners.extend(owners);
                    // Seen as the earlier look saw it: what came since, the
                    // ones kept haven't seen.
                    now.seen = watch.seen;
                }
                None => self.watches.push(Watch { owners, ..*watch }),
            }
        }
    }

    /// Room for as much as `before` waited on.
    pub(crate) fn sized_as(before: Option<&Self>) -> Self {
        Self {
            watches: Vec::with_capacity(before.map_or(0, |before| before.watches.len())),
            ..Self::default()
        }
    }

    /// Whether any of it has come, putting in `fired` who it came for.
    ///
    /// Come is as of the change tick `now`, or for a timer, the clock past it.
    /// Without a clock, a timer always is.
    ///
    /// Once it's due, what a node waits on isn't looked at: nodes are ticked
    /// whenever their tree is, so only which decorator parts it came for
    /// matters.
    pub(crate) fn due(
        &self,
        world: &World,
        clock: Option<Duration>,
        now: Tick,
        fired: &mut Owners,
    ) -> bool {
        fired.clear();
        fired.extend(self.polls);
        for &(owner, at) in &self.timers {
            if clock.is_none_or(|clock| clock >= at) {
                fired.insert(owner);
            }
        }
        for watch in &self.watches {
            if watch.owners.beyond(*fired) && watch.come(world, now) {
                fired.extend(watch.owners);
            }
        }
        !fired.is_empty()
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

/// What `plan` waits on a change to: nothing, without one.
fn watches(plan: Option<&Plan>) -> &[Watch] {
    plan.map_or(&[], |plan| &plan.watches)
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
    indexes: HashMap<TypeId, Index>,
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
            indexes: HashMap::default(),
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
struct Index {
    /// The trees watching it, by the entity it's on.
    ///
    /// Empty for a task, whose watcher is its agent.
    watchers: EntityHashMap<Watchers>,
    /// Whether it's [`Task`], watched only by its agent's tree.
    ///
    /// That tree is found by its [`TaskOf`], not indexed.
    tasks: bool,
    scan: Box<dyn Scan>,
}

impl Index {
    /// The index for `T`, hearing it taken away from now on.
    fn of<T: Component>(world: &mut World) -> Self {
        // A task's going is heard by its own hook: no observer of each one.
        let tasks = TypeId::of::<T>() == TypeId::of::<Task>();
        if !tasks {
            world.add_observer(removed::<T>);
        }
        Self {
            watchers: EntityHashMap::default(),
            tasks,
            scan: Box::new(Scanner::<T>(QueryState::new(world))),
        }
    }
}

/// Has a task's agent's tree looked at as the task goes, despawned or not.
///
/// Whether it was waiting on it, its own plan says. [`Task`]'s hook, cheaper
/// than an observer of every task going.
pub(crate) fn task_removed(mut world: DeferredWorld, context: HookContext) {
    let Some(agent) = world.get::<TaskOf>(context.entity).map(TaskOf::agent) else {
        return;
    };
    if let Some(mut wakes) = world.get_resource_mut::<Wakes>() {
        wakes.next.push(agent);
    }
}

/// What of `watches` is in the index: every component watched but a task's.
///
/// A task's agent is found by its `TaskOf`.
fn indexed(watches: &[Watch]) -> impl Iterator<Item = (Entity, TypeId)> + '_ {
    let task = TypeId::of::<Task>();
    watches
        .iter()
        .filter(move |watch| watch.kind != task)
        .map(|watch| (watch.entity, watch.kind))
}

/// The trees watching one entity's component: one, kept inline, or several.
#[derive(Default)]
enum Watchers {
    #[default]
    None,
    One(Entity),
    Many(Vec<Entity>),
}

impl Watchers {
    /// Adds `agent`'s tree.
    fn add(&mut self, agent: Entity) {
        match self {
            Self::None => *self = Self::One(agent),
            Self::One(first) => *self = Self::Many(vec![*first, agent]),
            Self::Many(agents) => agents.push(agent),
        }
    }

    /// Takes `agent`'s tree off: whether none are left.
    fn remove(&mut self, agent: Entity) -> bool {
        match self {
            Self::None => true,
            Self::One(one) => {
                if *one == agent {
                    *self = Self::None;
                }
                matches!(self, Self::None)
            }
            Self::Many(agents) => {
                if let Some(at) = agents.iter().position(|&watcher| watcher == agent) {
                    agents.swap_remove(at);
                }
                if let [one] = agents.as_slice() {
                    *self = Self::One(*one);
                }
                false
            }
        }
    }

    /// Every tree watching.
    fn as_slice(&self) -> &[Entity] {
        match self {
            Self::None => &[],
            Self::One(agent) => std::slice::from_ref(agent),
            Self::Many(agents) => agents,
        }
    }
}

/// Hears `T` taken off an entity, for its watchers to be looked at.
fn removed<T: Component>(removed: On<Remove<T>>, mut wakes: ResMut<Wakes>) {
    let kind = TypeId::of::<T>();
    if wakes
        .indexes
        .get(&kind)
        .is_some_and(|index| index.watchers.contains_key(&removed.entity))
    {
        wakes.removed.push((kind, removed.entity));
    }
}

/// Finds what changed of one component type.
trait Scan: Send + Sync {
    /// Puts in `due` the trees in `watchers` watching a component changed since
    /// `since`.
    ///
    /// It's as of `now`; for tasks, it's each changed one's agent's tree.
    fn changed(
        &mut self,
        world: &World,
        watchers: &EntityHashMap<Watchers>,
        tasks: bool,
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
        watchers: &EntityHashMap<Watchers>,
        tasks: bool,
        since: Tick,
        now: Tick,
        due: &mut Vec<Entity>,
    ) {
        let all = self.0.iter(world);
        if tasks {
            for (task, value) in all {
                if value.last_changed().is_newer_than(since, now)
                    && let Some(of) = world.get::<TaskOf>(task)
                {
                    due.push(of.agent());
                }
            }
            return;
        }
        // Few watched among many: each looked up. Else the storage passed
        // through once, which costs far less an entity.
        if watchers.len().saturating_mul(8) < all.len() {
            for (entity, watching) in watchers {
                if changed::<T>(world, *entity).is_some_and(|tick| tick.is_newer_than(since, now)) {
                    due.extend_from_slice(watching.as_slice());
                }
            }
            return;
        }
        for (entity, value) in all {
            if value.last_changed().is_newer_than(since, now)
                && let Some(watching) = watchers.get(&entity)
            {
                due.extend_from_slice(watching.as_slice());
            }
        }
    }
}

impl Wakes {
    /// The trees that may be due a tick, into `due`: each to be asked whether
    /// it is.
    ///
    /// It's as of the trees' `clock` and the change tick `now`. Some may come
    /// more than once.
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
        for index in self.indexes.values_mut() {
            index
                .scan
                .changed(world, &index.watchers, index.tasks, since, now, due);
        }
        for (kind, entity) in self.removed.drain(..) {
            if let Some(watching) = self
                .indexes
                .get(&kind)
                .and_then(|index| index.watchers.get(&entity))
            {
                due.extend_from_slice(watching.as_slice());
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
            .retain(|&agent| match world.get::<BehaviorTree>(agent) {
                Some(tree) if tree.is_fresh() => {
                    due.push(agent);
                    false
                }
                Some(tree) => tree.outcome().is_some(),
                None => false,
            });
    }

    /// Puts `agent`'s tree to sleep on `plan`, having slept on `before`.
    ///
    /// Indexes what it waits on now and didn't then, and forgets what it waited
    /// on then and doesn't now. Mostly, it's the same. One asking every tick
    /// goes in `next`, for [`Self::looked_at`] to hand over.
    pub(crate) fn sleep(
        world: &mut World,
        agent: Entity,
        before: Option<&Plan>,
        plan: &Plan,
        next: &mut Vec<Entity>,
    ) {
        let old = watches(before);
        if plan.is_poll() {
            next.push(agent);
        }
        let new = |watch: &&Watch| !old.iter().any(|old| old.same(watch));
        // A type watched for the first time has its index made: rarely.
        let unknown = {
            let wakes = world.resource::<Self>();
            plan.watches
                .iter()
                .filter(new)
                .any(|watch| !wakes.indexes.contains_key(&watch.kind))
        };
        if unknown {
            for watch in plan.watches.iter().filter(new) {
                Self::index(world, watch.kind, watch.index);
            }
        }
        let mut wakes = world.resource_mut::<Self>();
        for watch in old {
            if !plan.watches.iter().any(|new| new.same(watch)) {
                wakes.unwatch(agent, watch);
            }
        }
        for watch in plan.watches.iter().filter(new) {
            if let Some(index) = wakes.indexes.get_mut(&watch.kind)
                && !index.tasks
            {
                index.watchers.entry(watch.entity).or_default().add(agent);
            }
        }
        // One already in the heap stays good: none for the same time again.
        if let Some(at) = plan.at()
            && before.and_then(Plan::at) != Some(at)
        {
            wakes.timers.push(Reverse((at, agent)));
        }
    }

    /// Whether `plan` waits on just what `before` did: most often so.
    ///
    /// Then sleeping on it changes nothing in the index.
    ///
    /// A task isn't in the index, its agent found by its `TaskOf`, so a new
    /// task in place of the last doesn't count.
    pub(crate) fn same(before: Option<&Plan>, plan: &Plan) -> bool {
        indexed(watches(before)).eq(indexed(&plan.watches))
            && before.and_then(Plan::at) == plan.at()
    }

    /// Hands over the trees asking to be looked at on the next tick.
    ///
    /// They're gathered as trees went to sleep.
    pub(crate) fn looked_at(&mut self, next: &mut Vec<Entity>) {
        self.next.append(next);
    }

    /// Forgets what `agent`'s tree waited on, by `plan`: it's gone, or
    /// finished.
    ///
    /// A timer stays in the heap, and has it looked at for nothing at worst.
    pub(crate) fn forget(&mut self, agent: Entity, plan: &Plan) {
        for watch in &plan.watches {
            self.unwatch(agent, watch);
        }
    }

    /// Takes `agent` off the watchers of `watch`.
    fn unwatch(&mut self, agent: Entity, watch: &Watch) {
        let Some(index) = self
            .indexes
            .get_mut(&watch.kind)
            .filter(|index| !index.tasks)
        else {
            return;
        };
        if let Some(watching) = index.watchers.get_mut(&watch.entity)
            && watching.remove(agent)
        {
            index.watchers.remove(&watch.entity);
        }
    }

    /// Makes the index for tasks, as the plugin's built.
    ///
    /// A tree waiting on its task alone never goes through [`Self::sleep`],
    /// tasks not being indexed.
    pub(crate) fn index_tasks(world: &mut World) {
        Self::index(world, TypeId::of::<Task>(), Index::of::<Task>);
    }

    /// Makes the index for `kind` with `make`, unless it's made.
    fn index(world: &mut World, kind: TypeId, make: fn(&mut World) -> Index) {
        if !world.resource::<Self>().indexes.contains_key(&kind) {
            let index = make(world);
            world.resource_mut::<Self>().indexes.insert(kind, index);
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

    use crate::behavior::Context;
    use crate::prelude::*;
    use crate::testing::*;

    static ASKED: AtomicUsize = AtomicUsize::new(0);

    fn high_and_counted(flags: &Flags) -> bool {
        ASKED.fetch_add(1, Ordering::Relaxed);
        flags.high
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
        let yes = with_flags(&mut app, tree(), true, false);
        let no = with_flags(&mut app, tree(), true, true);
        app.tick();
        assert_eq!(outcome(&app, yes), Some(true));
        assert_eq!(outcome(&app, no), Some(false));
    }

    #[test]
    fn a_typed_while_stops_its_child_once_what_it_reads_changes() {
        let mut app = app();
        let npc = with_flags(
            &mut app,
            behavior_tree! { RepeatWhile(is_high) => { task(Work) } },
            true,
            false,
        );
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
    fn a_change_made_after_a_condition_looked_wakes_the_tree_next_tick() {
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
            "the condition said no, then the act ran"
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
                    when(move |cx: &mut Context| {
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
        app.entity_mut(npc).insert(BehaviorTreePaused);
        set(&mut app, npc, true, false);
        app.ticks(2);
        assert_eq!(app.count::<With<Other>>(), 1, "paused");
        app.entity_mut(npc).remove::<BehaviorTreePaused>();
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 1, "caught up");
    }
}
