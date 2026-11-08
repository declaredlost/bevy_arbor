use super::ordered::Order;
use crate::behavior::system::Check;
use crate::behavior::{Behave, Behavior, Child, Context, Outcome, Status};

/// Its children in an order shuffled as it starts, by rolls from `roll`, until
/// one succeeds.
pub(in crate::behavior) struct Random {
    roll: Check<f32>,
    children: Vec<Behavior>,
    order: Order,
}

impl Random {
    pub(in crate::behavior) fn new(roll: Check<f32>, children: Vec<Behavior>) -> Self {
        Self {
            roll,
            children,
            order: Order::default(),
        }
    }

    /// A shuffled order: Fisher-Yates, a roll for each place from the last.
    fn shuffle(&mut self, cx: &mut Context) -> Vec<usize> {
        let mut order: Vec<usize> = (0..self.children.len()).collect();
        for last in (1..order.len()).rev() {
            let rolled = self.roll.run(cx).unwrap_or(0.0);
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
        format!("Random({})", self.roll.name())
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
