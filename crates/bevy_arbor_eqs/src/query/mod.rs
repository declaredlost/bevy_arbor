//! A query: what it generates, how it filters and scores, and what it picks.

mod context;
mod item;

use std::sync::Arc;

use bevy::prelude::*;

pub use self::context::QueryContext;
pub use self::item::{Item, Scored};
use crate::generate::Generate;
use crate::test::Test;

/// An environment query: items from its generators, those its filters keep,
/// scored, and the one or more it [picks](Pick).
///
/// ```
/// # use bevy_arbor_eqs::prelude::*;
/// # use bevy_arbor_perception::prelude::*;
/// // Somewhere near me, outside where the enemy faces (or faced, when last
/// // seen), the further from it the better.
/// let cover = EnvQuery::new(
///     Donut::around(Agent).radius(3.0, 9.0).rings(3).points(16),
/// )
/// .filter(Distance::to(Agent), Filter::at_most(10.0))
/// .filter(
///     Dot::of(Hostiles::<Sight>::remembered()),
///     Filter::at_most(0.5),
/// )
/// .score(Distance::to(Agent), Score::lower())
/// .score(
///     Distance::to(Hostiles::<Sight>::remembered()),
///     Score::higher().weight(0.5),
/// );
/// ```
#[derive(Clone)]
pub struct EnvQuery {
    generators: Vec<Arc<dyn Generate>>,
    filters: Vec<(Arc<dyn Test>, Filter)>,
    scores: Vec<(Arc<dyn Test>, Score)>,
    pick: Pick,
}

/// What a filter keeps: items whose number is from `min` to `max`, each end
/// included.
#[derive(Reflect, Debug, Clone, Copy, PartialEq)]
#[reflect(Debug, PartialEq)]
pub struct Filter {
    min: f32,
    max: f32,
}

impl Filter {
    /// From `min` to `max`.
    #[must_use]
    pub fn range(min: f32, max: f32) -> Self {
        Self { min, max }
    }

    /// `min` or more.
    #[must_use]
    pub fn at_least(min: f32) -> Self {
        Self::range(min, f32::INFINITY)
    }

    /// `max` or less.
    #[must_use]
    pub fn at_most(max: f32) -> Self {
        Self::range(f32::NEG_INFINITY, max)
    }

    /// A yes or no, `yes`.
    #[must_use]
    pub fn is(yes: bool) -> Self {
        let one = f32::from(u8::from(yes));
        Self::range(one, one)
    }

    fn keeps(self, value: f32) -> bool {
        (self.min..=self.max).contains(&value)
    }
}

/// How a test's numbers score: normalized over the items to 0 for the lowest
/// and 1 for the highest, turned round for [`lower`](Self::lower), squared if
/// [`squared`](Self::squared), and weighted.
#[derive(Reflect, Debug, Clone, Copy, PartialEq)]
#[reflect(Debug, PartialEq)]
pub struct Score {
    higher: bool,
    squared: bool,
    weight: f32,
}

impl Score {
    /// The higher the better.
    #[must_use]
    pub fn higher() -> Self {
        Self {
            higher: true,
            squared: false,
            weight: 1.0,
        }
    }

    /// The lower the better.
    #[must_use]
    pub fn lower() -> Self {
        Self {
            higher: false,
            ..Self::higher()
        }
    }

    /// Squared, after it's normalized: what's best stands out more.
    #[must_use]
    pub fn squared(mut self) -> Self {
        self.squared = true;
        self
    }

    /// Weighted by `weight`, against the query's other scores.
    #[must_use]
    pub fn weight(mut self, weight: f32) -> Self {
        self.weight = weight;
        self
    }

    /// What `normalized`, from 0 to 1, scores.
    fn of(self, normalized: f32) -> f32 {
        let turned = if self.higher {
            normalized
        } else {
            1.0 - normalized
        };
        let shaped = if self.squared {
            turned * turned
        } else {
            turned
        };
        shaped * self.weight
    }
}

/// Which of what a query keeps it gives.
#[derive(Reflect, Debug, Clone, Copy, PartialEq, Default)]
#[reflect(Debug, PartialEq, Default)]
pub enum Pick {
    /// The best.
    #[default]
    Best,
    /// One at random among the best: this share of them, from 0 to 1, at
    /// least one: `RandomOfBest(0.05)` is one of the best 5%.
    RandomOfBest(f32),
    /// All of them, best first.
    All,
}

/// What a query found, and how it weighed everything.
pub(crate) struct Run {
    /// What it picked, best first: empty if nothing was kept.
    pub(crate) picked: Vec<Scored>,
    /// Every item, and its score, or `None` if a filter dropped it.
    pub(crate) weighed: Vec<(Item, Option<f32>)>,
}

impl EnvQuery {
    /// A query of what `generator` makes, keeping everything, picking the best.
    pub fn new(generator: impl Generate) -> Self {
        Self {
            generators: vec![Arc::new(generator)],
            filters: Vec::new(),
            scores: Vec::new(),
            pick: Pick::default(),
        }
    }

    /// What `generator` makes, too.
    #[must_use]
    pub fn generate(mut self, generator: impl Generate) -> Self {
        self.generators.push(Arc::new(generator));
        self
    }

    /// Keeps only items `test` gives a number `filter` keeps. Filters run
    /// before scores, cheapest first, whatever order they're given in.
    #[must_use]
    pub fn filter(mut self, test: impl Test, filter: Filter) -> Self {
        self.filters.push((Arc::new(test), filter));
        self
    }

    /// Scores items by `test`, as `score` says. Several add up.
    #[must_use]
    pub fn score(mut self, test: impl Test, score: Score) -> Self {
        self.scores.push((Arc::new(test), score));
        self
    }

    /// Gives what `pick` says: the best, unless told otherwise.
    #[must_use]
    pub fn pick(mut self, pick: Pick) -> Self {
        self.pick = pick;
        self
    }

    /// Gets its tests ready to run in `world`.
    pub(crate) fn prepare(&self, world: &mut World) {
        for (test, _) in &self.filters {
            test.prepare(world);
        }
        for (test, _) in &self.scores {
            test.prepare(world);
        }
    }

    /// Runs it for the agent of `cx`.
    pub(crate) fn run(&self, cx: &QueryContext) -> Run {
        let mut items = Vec::new();
        for generator in &self.generators {
            generator.generate(cx, &mut items);
        }
        // Cheapest first, each asked only about what the ones before kept, so
        // the costly see fewer items.
        let mut filters: Vec<&(Arc<dyn Test>, Filter)> = self.filters.iter().collect();
        filters.sort_by_key(|(test, _)| test.cost());
        let mut kept: Vec<bool> = vec![true; items.len()];
        for (test, filter) in filters {
            let indices: Vec<usize> = (0..items.len()).filter(|&index| kept[index]).collect();
            let asked: Vec<Item> = indices.iter().map(|&index| items[index]).collect();
            for (index, value) in indices.into_iter().zip(test.test_all(cx, &asked)) {
                kept[index] = value.is_some_and(|value| filter.keeps(value));
            }
        }
        let survivors: Vec<Item> = items
            .iter()
            .zip(&kept)
            .filter_map(|(item, kept)| kept.then_some(*item))
            .collect();
        let scores = self.scores_of(cx, &survivors);
        let mut ranked: Vec<Scored> = survivors
            .iter()
            .zip(&scores)
            .map(|(&item, &score)| Scored { item, score })
            .collect();
        ranked.sort_by(|a, b| b.score.total_cmp(&a.score));
        let mut each = scores.into_iter();
        let weighed = items
            .into_iter()
            .zip(kept)
            .map(|(item, kept)| (item, if kept { each.next() } else { None }))
            .collect();
        Run {
            picked: self.picked(cx, ranked),
            weighed,
        }
    }

    /// Each of `items`' score: its scores, normalized and weighted, added up;
    /// 1 each with none.
    fn scores_of(&self, cx: &QueryContext, items: &[Item]) -> Vec<f32> {
        if self.scores.is_empty() {
            return vec![1.0; items.len()];
        }
        let mut totals = vec![0.0; items.len()];
        for (test, score) in &self.scores {
            // NaN is no answer: it would otherwise rank above everything.
            let values: Vec<Option<f32>> = test
                .test_all(cx, items)
                .into_iter()
                .map(|value| value.filter(|value| !value.is_nan()))
                .collect();
            let (low, high) = values
                .iter()
                .flatten()
                .fold((f32::INFINITY, f32::NEG_INFINITY), |(low, high), &value| {
                    (low.min(value), high.max(value))
                });
            for (total, value) in totals.iter_mut().zip(&values) {
                let Some(value) = value else {
                    continue;
                };
                // All alike, they're all the best.
                let normalized = if high > low {
                    (value - low) / (high - low)
                } else {
                    1.0
                };
                *total += score.of(normalized);
            }
        }
        totals
    }

    /// What it gives of `ranked`, best first.
    fn picked(&self, cx: &QueryContext, mut ranked: Vec<Scored>) -> Vec<Scored> {
        match self.pick {
            Pick::All => ranked,
            _ if ranked.is_empty() => ranked,
            Pick::Best => {
                ranked.truncate(1);
                ranked
            }
            Pick::RandomOfBest(share) => {
                #[expect(
                    clippy::cast_precision_loss,
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "a share of a count of items, rounded up, at least one"
                )]
                let best = ((ranked.len() as f32 * share.clamp(0.0, 1.0)).ceil() as usize).max(1);
                #[expect(
                    clippy::cast_precision_loss,
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "a roll scaled to a count of items, floored"
                )]
                let index = ((cx.roll() * best as f32) as usize).min(best - 1);
                vec![ranked.swap_remove(index)]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use bevy_arbor_perception::prelude::*;

    use super::*;
    use crate::anchor::{Agent, At};
    use crate::test::{Cost, Distance};
    use crate::testing::*;

    /// Points 1 to 5 out along +X.
    struct Line;

    impl Generate for Line {
        fn generate(&self, _cx: &QueryContext, out: &mut Vec<Item>) {
            out.extend((1..=5u8).map(|x| Item::Point(Vec3::new(f32::from(x), 0.0, 0.0))));
        }
    }

    fn ran(query: &EnvQuery) -> Run {
        let mut app = app(Space::Volume);
        let agent = app.spawn(GlobalTransform::IDENTITY);
        app.tick();
        query.run(&QueryContext::new(app.world(), agent, Space::Volume, 3))
    }

    fn xs(scored: &[Scored]) -> Vec<f32> {
        scored
            .iter()
            .map(|scored| scored.item.position().x)
            .collect()
    }

    #[test]
    fn filters_keep_what_is_in_range() {
        let query = EnvQuery::new(Line)
            .filter(Distance::to(Agent), Filter::range(2.0, 4.0))
            .pick(Pick::All);
        let run = ran(&query);
        let mut kept = xs(&run.picked);
        kept.sort_by(f32::total_cmp);
        assert_eq!(kept, [2.0, 3.0, 4.0]);
        assert_eq!(
            run.weighed
                .iter()
                .filter(|(_, score)| score.is_none())
                .count(),
            2
        );
    }

    #[test]
    fn scores_rank_the_best_first_and_add_up_by_weight() {
        let lower = EnvQuery::new(Line)
            .score(Distance::to(Agent), Score::lower())
            .pick(Pick::All);
        assert_eq!(xs(&ran(&lower).picked), [1.0, 2.0, 3.0, 4.0, 5.0]);
        let higher = EnvQuery::new(Line)
            .score(Distance::to(Agent), Score::higher())
            .pick(Pick::All);
        assert_eq!(ran(&higher).picked[0].score, 1.0);
        assert_eq!(xs(&ran(&higher).picked)[0], 5.0);
        // Near 4 matters twice as much as away from the agent.
        let weighed = EnvQuery::new(Line)
            .score(
                Distance::to(At(Vec3::new(4.0, 0.0, 0.0))),
                Score::lower().weight(2.0),
            )
            .score(Distance::to(Agent), Score::higher());
        assert_eq!(xs(&ran(&weighed).picked), [4.0]);
    }

    #[test]
    fn squared_makes_the_best_stand_out() {
        assert_eq!(Score::higher().squared().of(0.5), 0.25);
        assert_eq!(Score::lower().squared().of(0.25), 0.5625);
    }

    #[test]
    fn a_query_that_only_filters_scores_everything_one() {
        let query = EnvQuery::new(Line).pick(Pick::All);
        assert!(
            ran(&query)
                .picked
                .iter()
                .all(|scored| (scored.score - 1.0).abs() < f32::EPSILON)
        );
    }

    #[test]
    fn best_gives_one_and_random_of_best_one_of_the_best() {
        let best = EnvQuery::new(Line).score(Distance::to(Agent), Score::higher());
        assert_eq!(xs(&ran(&best).picked), [5.0]);
        let random = EnvQuery::new(Line)
            .score(Distance::to(Agent), Score::higher())
            .pick(Pick::RandomOfBest(0.4));
        let picked = xs(&ran(&random).picked);
        assert_eq!(picked.len(), 1);
        assert!(picked[0] >= 4.0, "among the best two: {picked:?}");
    }

    #[test]
    fn nothing_kept_picks_nothing() {
        let query = EnvQuery::new(Line).filter(Distance::to(Agent), Filter::at_least(100.0));
        assert_eq!(ran(&query).picked.len(), 0);
    }

    /// A test counting how often it's asked.
    struct Counted(Arc<AtomicUsize>, Cost, f32);

    impl Test for Counted {
        fn test(&self, _cx: &QueryContext, item: &Item) -> Option<f32> {
            self.0.fetch_add(1, Ordering::Relaxed);
            Some(f32::from(u8::from(item.position().x < self.2)))
        }

        fn cost(&self) -> Cost {
            self.1
        }
    }

    #[test]
    fn cheap_filters_run_first_so_costly_ones_see_less() {
        let costly = Arc::new(AtomicUsize::new(0));
        let cheap = Arc::new(AtomicUsize::new(0));
        let query = EnvQuery::new(Line)
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
        fn test(&self, _cx: &QueryContext, item: &Item) -> Option<f32> {
            let x = item.position().x;
            Some(if (x - 3.0).abs() < f32::EPSILON {
                f32::NAN
            } else {
                x
            })
        }
    }

    #[test]
    fn a_nan_is_no_answer_rather_than_the_best() {
        let query = EnvQuery::new(Line).score(Broken, Score::lower());
        assert_eq!(xs(&ran(&query).picked), [1.0], "not the broken one");
    }
}
