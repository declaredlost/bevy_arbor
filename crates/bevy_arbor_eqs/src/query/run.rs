use std::sync::Arc;

use bevy::prelude::*;
use bevy_arbor_perception::Space;

use super::state::{EnvQueryStates, Prepared};
use super::{EnvQueryContext, EnvQueryItem, Parts, Pick, ScoredItem, Weighed};

/// What a query found, and how it weighed everything.
pub(crate) struct Run {
    /// What it picked, best first: empty if nothing was kept.
    pub(crate) picked: Vec<ScoredItem>,
    /// Every item as it was weighed, in the order its generators made them.
    pub(crate) weighed: Vec<Weighed>,
}

/// Where queries' randomness comes from, moved on each run.
#[derive(Resource)]
pub(crate) struct Seed(pub(crate) u64);

impl Default for Seed {
    fn default() -> Self {
        Self(0x9E37_79B9_7F4A_7C15)
    }
}

impl Seed {
    /// The next run's seed, moved on from the last.
    fn next(world: &mut World) -> u64 {
        let mut seed = world.get_resource_or_init::<Self>();
        seed.0 = seed
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        seed.0
    }
}

impl Parts {
    /// Runs `parts` for `querier`, carried by `carrier`.
    ///
    /// Its tests and generators get their state ready first, with the world to
    /// change: what they keep in it, made the first time it runs there.
    pub(crate) fn run_in(
        parts: &Arc<Self>,
        world: &mut World,
        querier: Entity,
        carrier: Entity,
    ) -> Run {
        // Perception's, if it's there; 3D if not.
        let space = world.get_resource::<Space>().copied().unwrap_or_default();
        let seed = Seed::next(world);
        let mut prepared = EnvQueryStates::take(world, parts);
        prepared.prepare(parts, world);
        let cx = EnvQueryContext::new(world, querier, space, seed).carried_by(carrier);
        let run = parts.run(&mut prepared, &cx);
        EnvQueryStates::put(world, parts, prepared);
        run
    }
}

impl Parts {
    /// Runs it for the agent of `cx`, with its parts' state, `prepared`.
    fn run(&self, prepared: &mut Prepared, cx: &EnvQueryContext) -> Run {
        let mut items = Vec::new();
        for (generator, state) in self.generators.iter().zip(&mut prepared.generators) {
            generator.generate(state, cx, &mut items);
        }
        // Cheapest first, each asked only about what the ones before kept, so
        // the costly see fewer items.
        let mut order: Vec<usize> = (0..self.filters.len()).collect();
        order.sort_by_key(|&index| self.filters[index].0.cost());
        let mut kept: Vec<bool> = vec![true; items.len()];
        for index in order {
            let (test, filter) = &self.filters[index];
            let indices: Vec<usize> = (0..items.len()).filter(|&at| kept[at]).collect();
            let asked: Vec<EnvQueryItem> = indices.iter().map(|&at| items[at]).collect();
            let values = test.test(&mut prepared.filters[index], cx, &asked);
            for (at, value) in indices.into_iter().zip(values) {
                kept[at] = value.is_some_and(|value| filter.keeps(value));
            }
        }
        let survivors: Vec<EnvQueryItem> = items
            .iter()
            .zip(&kept)
            .filter_map(|(item, kept)| kept.then_some(*item))
            .collect();
        let scores = self.scores_of(prepared, cx, &survivors);
        // Each kept item's score, by its place among all of them.
        let mut score: Vec<Option<f32>> = vec![None; items.len()];
        let mut ranked: Vec<(usize, ScoredItem)> = Vec::with_capacity(survivors.len());
        let kept_at = (0..items.len()).filter(|&at| kept[at]);
        for ((at, &item), &value) in kept_at.zip(&survivors).zip(&scores) {
            score[at] = Some(value);
            ranked.push((at, ScoredItem { item, score: value }));
        }
        ranked.sort_by(|(_, a), (_, b)| b.score.total_cmp(&a.score));
        let picked = self.picked(cx, ranked);
        let mut chosen = vec![false; items.len()];
        for &(at, _) in &picked {
            chosen[at] = true;
        }
        let weighed = items
            .into_iter()
            .enumerate()
            .map(|(at, item)| Weighed {
                item,
                score: score[at],
                picked: chosen[at],
            })
            .collect();
        Run {
            picked: picked.into_iter().map(|(_, scored)| scored).collect(),
            weighed,
        }
    }

    /// Each of `items`' score: its scores, normalized and weighted, added up.
    ///
    /// With no scores, 1 each.
    fn scores_of(
        &self,
        prepared: &mut Prepared,
        cx: &EnvQueryContext,
        items: &[EnvQueryItem],
    ) -> Vec<f32> {
        if self.scores.is_empty() {
            return vec![1.0; items.len()];
        }
        let mut totals = vec![0.0; items.len()];
        for ((test, score), state) in self.scores.iter().zip(&mut prepared.scores) {
            // NaN is no answer: it would otherwise rank above everything.
            let values: Vec<Option<f32>> = test
                .test(state, cx, items)
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
    fn picked(
        &self,
        cx: &EnvQueryContext,
        mut ranked: Vec<(usize, ScoredItem)>,
    ) -> Vec<(usize, ScoredItem)> {
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
