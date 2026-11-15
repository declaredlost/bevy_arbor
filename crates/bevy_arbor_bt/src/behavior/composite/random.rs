use super::ordered::Order;
use crate::behavior::{Behave, Behavior, Child, Context, Outcome, Status};

/// Its children in an order shuffled as it starts, until one succeeds.
///
/// Shuffled by a generator of its own, seeded from `seed`, or with none, from
/// the agent: the same tree on the same agent shuffles the same, run after
/// run.
pub(in crate::behavior) struct Random {
    seed: Option<u64>,
    /// Where its generator is, once it's started.
    state: Option<u64>,
    children: Vec<Behavior>,
    order: Order,
}

impl Random {
    pub(in crate::behavior) fn new(seed: Option<u64>, children: Vec<Behavior>) -> Self {
        Self {
            seed,
            state: None,
            children,
            order: Order::default(),
        }
    }

    /// A roll from 0 up to 1: `SplitMix64`, its top 24 bits.
    fn roll(&mut self, cx: &Context) -> f32 {
        let state = self
            .state
            .get_or_insert_with(|| self.seed.unwrap_or_else(|| cx.agent().to_bits()));
        *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut mixed = *state;
        mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        mixed ^= mixed >> 31;
        let top = u32::try_from(mixed >> 40).unwrap_or(0);
        #[expect(clippy::cast_precision_loss, reason = "24 bits fit an `f32` exactly")]
        let roll = top as f32 / (1u32 << 24) as f32;
        roll
    }

    /// A shuffled order: Fisher-Yates, a roll for each place from the last.
    fn shuffle(&mut self, cx: &Context) -> Vec<usize> {
        let mut order: Vec<usize> = (0..self.children.len()).collect();
        for last in (1..order.len()).rev() {
            let rolled = self.roll(cx);
            order.swap(last, pick(rolled, last));
        }
        order
    }
}

impl Behave for Random {
    fn start(&mut self, cx: &mut Context) {
        let order = self.shuffle(cx);
        self.order.start(order);
    }

    fn tick(&mut self, cx: &mut Context) -> Status {
        self.order.tick(&mut self.children, |child| child, cx)
    }

    fn end(&mut self, cx: &mut Context, _outcome: Option<Outcome>) {
        for child in &mut self.children {
            child.cancel(cx);
        }
    }

    fn label(&self) -> String {
        match self.seed {
            Some(seed) => format!("Random({seed})"),
            None => "Random".to_owned(),
        }
    }

    fn children(&self) -> Vec<Child<'_>> {
        self.children.iter().map(Child::new).collect()
    }
}

/// Where a roll in `0..1` lands among `0..=last`: the roll scaled to the
/// `last + 1` places, floored, and kept in range (a roll of 1 or more lands on
/// `last`, below 0 on `0`).
fn pick(rolled: f32, last: usize) -> usize {
    let places = u16::try_from(last + 1).unwrap_or(u16::MAX);
    let scaled = rolled * f32::from(places);
    let floor = (1..places).take_while(|&k| f32::from(k) <= scaled).count();
    floor.min(last)
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use crate::prelude::*;
    use crate::testing::*;

    #[test]
    fn random_tries_each_once_and_fails_once_all_have() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behavior_tree! { Random => { act(refuse), act(refuse), act(refuse) } },
        );
        app.tick();
        assert_eq!(acts(&app), 3);
        assert_eq!(outcome(&app, npc), Some(false));
    }

    #[test]
    fn a_seed_shuffles_the_same_every_time_and_seeds_differ() {
        let mut app = app();
        let tree = |seed| behavior_tree! { Random(seed) => { task(Work), task(Other) } };
        for seed in 0..64 {
            agent(&mut app, tree(seed));
            agent(&mut app, tree(seed));
        }
        app.tick();
        let mut works = app.world_mut().query_filtered::<&TaskOf, With<Work>>();
        let mut firsts: Vec<_> = works.iter(app.world()).map(TaskOf::agent).collect();
        firsts.sort();
        assert!(
            firsts
                .chunks(2)
                .all(|pair| pair.len() == 2 && pair[0].to_bits() + 1 == pair[1].to_bits()),
            "the same seed, the same first"
        );
        let started = firsts.len();
        assert!(
            (32..=96).contains(&started),
            "about half first: {started} of 128"
        );
    }

    #[test]
    fn a_roll_lands_on_its_floored_place() {
        use super::pick;
        assert_eq!(pick(0.0, 3), 0);
        assert_eq!(pick(0.25, 1), 0);
        assert_eq!(pick(0.5, 3), 2);
        assert_eq!(pick(0.99, 3), 3);
        assert_eq!(pick(1.0, 3), 3);
        assert_eq!(pick(-1.0, 3), 0);
    }
}
