use super::ordered::Order;
use crate::behavior::system::Check;
use crate::behavior::{Behave, Behavior, Context, Outcome, Status};
use crate::inspect::{Inspect, Inspection};

/// Its children in an order shuffled as it starts, until one succeeds.
pub(in crate::behavior) struct Random {
    roll: Roll,
    children: Vec<Behavior>,
    order: Order,
}

/// Where a [`Random`]'s shuffles come from.
pub(in crate::behavior) enum Roll {
    /// A generator of its own, seeded from the agent once it's shuffled.
    Own(Option<fastrand::Rng>),
    /// A system taking the agent, rolling from 0 up to 1.
    System(Check<f32>),
}

impl Random {
    pub(in crate::behavior) fn new(roll: Roll, children: Vec<Behavior>) -> Self {
        Self {
            roll,
            children,
            order: Order::default(),
        }
    }

    /// A shuffled order of its children.
    fn shuffle(&mut self, cx: &mut Context) -> Vec<usize> {
        let mut order: Vec<usize> = (0..self.children.len()).collect();
        match &mut self.roll {
            Roll::Own(rng) => {
                let agent = cx.agent();
                rng.get_or_insert_with(|| fastrand::Rng::with_seed(agent.to_bits()))
                    .shuffle(&mut order);
            }
            // Fisher-Yates, a roll for each place from the last.
            Roll::System(check) => {
                for last in (1..order.len()).rev() {
                    let rolled = check.run(cx).unwrap_or(0.0);
                    order.swap(last, pick(rolled, last));
                }
            }
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
}

impl Inspect for Random {
    fn inspect(&self, inspection: &mut Inspection<'_>) {
        inspection.name("Random");
        if let Roll::System(check) = &self.roll {
            inspection.param(check.name());
        }
        for child in &self.children {
            inspection.child(child);
        }
    }
}

/// Where a roll in `0..1` lands among `0..=last`, kept in range.
///
/// The roll is scaled to the `last + 1` places and floored: a roll of 1 or more
/// lands on `last`, below 0 on `0`.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "a place among a node's children, clamped to them"
)]
fn pick(rolled: f32, last: usize) -> usize {
    ((rolled * (last + 1) as f32) as usize).min(last)
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use crate::prelude::*;
    use crate::testing::*;

    fn first(In(_): In<Entity>) -> f32 {
        0.0
    }

    #[test]
    fn random_goes_by_the_rolls_it_is_given() {
        // Rolling 0 every time swaps each last with the first: 0, 1, 2
        // becomes 1, 2, 0.
        let mut app = app();
        agent(
            &mut app,
            behavior_tree! {
                Random(first) => {
                    task(Work),
                    task(Other),
                    act(count),
                }
            },
        );
        app.tick();
        assert_eq!(app.count::<With<Other>>(), 1);
        assert_eq!(app.count::<With<Work>>(), 0);
    }

    #[test]
    fn random_fails_once_all_have() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behavior_tree! { Random(first) => { act(refuse), act(refuse) } },
        );
        app.tick();
        assert_eq!(acts(&app), 2);
        assert_eq!(outcome(&app, npc), Some(false));
    }

    #[test]
    fn random_of_its_own_tries_every_child_once() {
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
