//! A query: what it generates, how it filters and scores, and what it picks.

mod context;
mod filter;
mod item;
mod pick;
mod run;
mod score;
mod state;

use std::fmt;
use std::marker::PhantomData;
use std::sync::Arc;

use bevy::prelude::*;
use bevy::utils::prelude::ShortName;

pub use self::context::EnvQueryContext;
pub use self::filter::Filter;
pub use self::item::{EnvQueryItem, ScoredItem, Weighed};
pub use self::pick::Pick;
pub(crate) use self::run::Seed;
pub use self::score::Score;
use self::state::{AnyGenerate, AnyTest};
use crate::generate::Generate;
use crate::test::Test;

/// What a query is for, as a type of your own: `Cover`, `UseSpot`, `Ammo`.
///
/// It keys the query, a [`Survey`](crate::survey::Survey) of it, and what it
/// found, a [`Found`](crate::survey::Found): any type will do.
pub trait EnvQueryKey: Send + Sync + 'static {}

impl<K: Send + Sync + 'static> EnvQueryKey for K {}

/// An environment query for `K`, carried by an agent or a thing that's asked
/// it.
///
/// Items from its generators, those its filters keep, scored: it gives the one
/// or more it [picks](Pick). An agent carries the queries it
/// asks of itself; a thing, a turret, a door, those an agent asks of it, as
/// where to stand to use it.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_arbor_eqs::prelude::*;
/// # use bevy_arbor_perception::prelude::*;
/// /// Somewhere to take cover.
/// struct Cover;
///
/// // Somewhere near me, outside where the enemy faces (or faced, when last
/// // seen), the further from it the better.
/// let cover = EnvQuery::new(
///     Cover,
///     Donut::around(Querier).radius(3.0, 9.0).rings(3).points(16),
/// )
/// .filter(Distance::to(Querier), Filter::at_most(10.0))
/// .filter(
///     Dot::of(Percepts::<Sight>::all().hostile()),
///     Filter::at_most(0.5),
/// )
/// .score(Distance::to(Querier), Score::lower())
/// .score(
///     Distance::to(Percepts::<Sight>::all().hostile()),
///     Score::higher().weight(0.5),
/// );
/// ```
///
/// Cloning one is cheap: clones share what it's made of, and what its tests
/// and generators keep in a world between runs.
#[derive(Component)]
pub struct EnvQuery<K: EnvQueryKey> {
    parts: Arc<Parts>,
    key: PhantomData<fn() -> K>,
}

impl<K: EnvQueryKey> Clone for EnvQuery<K> {
    fn clone(&self) -> Self {
        Self {
            parts: Arc::clone(&self.parts),
            key: PhantomData,
        }
    }
}

impl<K: EnvQueryKey> fmt::Debug for EnvQuery<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EnvQuery<{}>", ShortName::of::<K>())
    }
}

/// What a query is made of.
#[derive(Clone)]
pub(crate) struct Parts {
    generators: Vec<Arc<dyn AnyGenerate>>,
    filters: Vec<(Arc<dyn AnyTest>, Filter)>,
    scores: Vec<(Arc<dyn AnyTest>, Score)>,
    pick: Pick,
}

impl<K: EnvQueryKey> EnvQuery<K> {
    /// A query for `key`, of what `generator` makes, keeping everything,
    /// picking the best.
    pub fn new(_key: K, generator: impl Generate) -> Self {
        Self {
            parts: Arc::new(Parts {
                generators: vec![Arc::new(generator)],
                filters: Vec::new(),
                scores: Vec::new(),
                pick: Pick::default(),
            }),
            key: PhantomData,
        }
    }

    /// What `generator` makes, too.
    #[must_use]
    pub fn generate(mut self, generator: impl Generate) -> Self {
        self.parts_mut().generators.push(Arc::new(generator));
        self
    }

    /// Keeps only items `test` gives a number `filter` keeps: filters run
    /// before scores.
    ///
    /// They run cheapest first, whatever order they're given in.
    #[must_use]
    pub fn filter(mut self, test: impl Test, filter: Filter) -> Self {
        self.parts_mut().filters.push((Arc::new(test), filter));
        self
    }

    /// Scores items by `test`, as `score` says. Several add up.
    #[must_use]
    pub fn score(mut self, test: impl Test, score: Score) -> Self {
        self.parts_mut().scores.push((Arc::new(test), score));
        self
    }

    /// Gives what `pick` says: the best, unless told otherwise.
    #[must_use]
    pub fn pick(mut self, pick: Pick) -> Self {
        self.parts_mut().pick = pick;
        self
    }

    /// What it's made of, to add to: its own, if it's shared.
    fn parts_mut(&mut self) -> &mut Parts {
        Arc::make_mut(&mut self.parts)
    }

    /// Runs it now for `querier`, as if it carried it: what it picks, best
    /// first.
    ///
    /// For a system or a test that wants an answer at once. A tree asks with
    /// a [`Survey`](crate::survey::Survey) instead, run within the budget, its
    /// answer kept on the agent as [`Found`](crate::survey::Found).
    pub fn run(&self, world: &mut World, querier: Entity) -> Vec<ScoredItem> {
        self.run_on(world, querier, querier)
    }

    /// Runs it now for `querier`, as `carrier` carries it: what it picks.
    ///
    /// Its [`Carrier`](crate::anchor::Carrier) anchor is `carrier`, as for a
    /// survey asked [`on`](crate::survey::Survey::on) it.
    pub fn run_on(&self, world: &mut World, querier: Entity, carrier: Entity) -> Vec<ScoredItem> {
        Parts::run_in(&self.parts, world, querier, carrier).picked
    }

    /// What it's made of, without its key.
    pub(crate) fn parts(&self) -> &Arc<Parts> {
        &self.parts
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use bevy_arbor_perception::Space;

    use super::run::Run;
    use super::*;
    use crate::anchor::{At, Querier};
    use crate::test::{Cost, Distance};
    use crate::testing::*;

    /// Points 1 to 5 out along +X.
    struct Line;

    impl Generate for Line {
        type State = ();

        fn generate(&self, (): &mut (), _cx: &EnvQueryContext, out: &mut Vec<EnvQueryItem>) {
            out.extend((1..=5u8).map(|x| EnvQueryItem::place(Vec3::new(f32::from(x), 0.0, 0.0))));
        }
    }

    struct Points;

    fn ran(query: &EnvQuery<Points>) -> Run {
        let mut app = app(Space::Volume);
        let agent = app.spawn(GlobalTransform::IDENTITY);
        app.tick();
        Parts::run_in(query.parts(), app.world_mut(), agent, agent)
    }

    fn xs(scored: &[ScoredItem]) -> Vec<f32> {
        scored.iter().map(|scored| scored.item.position.x).collect()
    }

    #[test]
    fn filters_keep_what_is_in_range() {
        let query = EnvQuery::new(Points, Line)
            .filter(Distance::to(Querier), Filter::range(2.0, 4.0))
            .pick(Pick::All);
        let run = ran(&query);
        let mut kept = xs(&run.picked);
        kept.sort_by(f32::total_cmp);
        assert_eq!(kept, [2.0, 3.0, 4.0]);
        assert_eq!(
            run.weighed
                .iter()
                .filter(|weighed| weighed.score.is_none())
                .count(),
            2
        );
    }

    #[test]
    fn scores_rank_the_best_first_and_add_up_by_weight() {
        let lower = EnvQuery::new(Points, Line)
            .score(Distance::to(Querier), Score::lower())
            .pick(Pick::All);
        assert_eq!(xs(&ran(&lower).picked), [1.0, 2.0, 3.0, 4.0, 5.0]);
        let higher = EnvQuery::new(Points, Line)
            .score(Distance::to(Querier), Score::higher())
            .pick(Pick::All);
        assert_eq!(ran(&higher).picked[0].score, 1.0);
        assert_eq!(xs(&ran(&higher).picked)[0], 5.0);
        // Near 4 matters twice as much as away from the agent.
        let weighed = EnvQuery::new(Points, Line)
            .score(
                Distance::to(At(Vec3::new(4.0, 0.0, 0.0))),
                Score::lower().weight(2.0),
            )
            .score(Distance::to(Querier), Score::higher());
        assert_eq!(xs(&ran(&weighed).picked), [4.0]);
    }

    #[test]
    fn squared_makes_the_best_stand_out() {
        assert_eq!(Score::higher().squared().of(0.5), 0.25);
        assert_eq!(Score::lower().squared().of(0.25), 0.5625);
    }

    #[test]
    fn a_query_that_only_filters_scores_everything_one() {
        let query = EnvQuery::new(Points, Line).pick(Pick::All);
        assert!(
            ran(&query)
                .picked
                .iter()
                .all(|scored| (scored.score - 1.0).abs() < f32::EPSILON)
        );
    }

    #[test]
    fn best_gives_one_and_random_of_best_one_of_the_best() {
        let best = EnvQuery::new(Points, Line).score(Distance::to(Querier), Score::higher());
        assert_eq!(xs(&ran(&best).picked), [5.0]);
        let random = EnvQuery::new(Points, Line)
            .score(Distance::to(Querier), Score::higher())
            .pick(Pick::RandomOfBest(0.4));
        let picked = xs(&ran(&random).picked);
        assert_eq!(picked.len(), 1);
        assert!(picked[0] >= 4.0, "among the best two: {picked:?}");
    }

    #[test]
    fn nothing_kept_picks_nothing() {
        let query =
            EnvQuery::new(Points, Line).filter(Distance::to(Querier), Filter::at_least(100.0));
        assert_eq!(ran(&query).picked.len(), 0);
    }

    /// A test counting how often it's asked.
    struct Counted(Arc<AtomicUsize>, Cost, f32);

    impl Test for Counted {
        type State = ();

        fn test(
            &self,
            (): &mut (),
            _cx: &EnvQueryContext,
            items: &[EnvQueryItem],
        ) -> Vec<Option<f32>> {
            items
                .iter()
                .map(|item| {
                    self.0.fetch_add(1, Ordering::Relaxed);
                    Some(f32::from(u8::from(item.position.x < self.2)))
                })
                .collect()
        }

        fn cost(&self) -> Cost {
            self.1
        }
    }

    #[test]
    fn cheap_filters_run_first_so_costly_ones_see_less() {
        let costly = Arc::new(AtomicUsize::new(0));
        let cheap = Arc::new(AtomicUsize::new(0));
        let query = EnvQuery::new(Points, Line)
            .filter(Counted(costly.clone(), Cost::High, 10.0), Filter::is(true))
            .filter(Counted(cheap.clone(), Cost::Low, 2.5), Filter::is(true));
        ran(&query);
        assert_eq!(cheap.load(Ordering::Relaxed), 5, "every item");
        assert_eq!(
            costly.load(Ordering::Relaxed),
            2,
            "only what the cheap one kept"
        );
    }

    /// A test giving NaN for the item at 3.
    struct Broken;

    impl Test for Broken {
        type State = ();

        fn test(
            &self,
            (): &mut (),
            _cx: &EnvQueryContext,
            items: &[EnvQueryItem],
        ) -> Vec<Option<f32>> {
            items
                .iter()
                .map(|item| {
                    let x = item.position.x;
                    Some(if (x - 3.0).abs() < f32::EPSILON {
                        f32::NAN
                    } else {
                        x
                    })
                })
                .collect()
        }
    }

    #[test]
    fn a_nan_is_no_answer_rather_than_the_best() {
        let query = EnvQuery::new(Points, Line).score(Broken, Score::lower());
        assert_eq!(xs(&ran(&query).picked), [1.0], "not the broken one");
    }
}
