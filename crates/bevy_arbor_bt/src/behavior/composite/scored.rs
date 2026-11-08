use bevy::prelude::*;

use super::ordered::Order;
use crate::behavior::system::Check;
use crate::behavior::{Behave, Behavior, Child, Context, Outcome, Status};

/// A branch of a `Scored`: a node, and the system scoring it for the agent.
pub struct ScoredBranch {
    score: Option<Check<f32>>,
    node: Behavior,
}

impl ScoredBranch {
    /// `node`, ranked by what `system` says of the agent.
    pub fn new<S, M>(system: S, node: Behavior) -> Self
    where
        S: IntoSystem<In<Entity>, f32, M> + Copy + Send + Sync + 'static,
    {
        Self {
            score: Some(Check::new(system)),
            node,
        }
    }

    /// `node`, after every branch with a score.
    #[must_use]
    pub fn last(node: Behavior) -> Self {
        Self { score: None, node }
    }
}

/// Its branches from the highest score down, scored as it starts, until one
/// succeeds.
pub(in crate::behavior) struct Scored {
    branches: Vec<ScoredBranch>,
    order: Order,
}

impl Scored {
    pub(in crate::behavior) fn new(branches: Vec<ScoredBranch>) -> Self {
        Self {
            branches,
            order: Order::default(),
        }
    }

    /// The order to try them in: highest score first, and one that can't be
    /// scored, isn't, or scores NaN, last.
    fn rank(&mut self, cx: &mut Context) -> Vec<usize> {
        let scores: Vec<f32> = self
            .branches
            .iter_mut()
            .map(|branch| {
                branch
                    .score
                    .as_mut()
                    .and_then(|score| score.run(cx))
                    .filter(|score| !score.is_nan())
                    .unwrap_or(f32::NEG_INFINITY)
            })
            .collect();
        let mut order: Vec<usize> = (0..self.branches.len()).collect();
        order.sort_by(|a, b| scores[*b].total_cmp(&scores[*a]));
        order
    }
}

impl Behave for Scored {
    fn start(&mut self, cx: &mut Context) {
        let order = self.rank(cx);
        self.order.start(order);
    }

    fn tick(&mut self, cx: &mut Context) -> Status {
        self.order
            .tick(&mut self.branches, |branch| &mut branch.node, cx)
    }

    fn end(&mut self, cx: &mut Context, _outcome: Option<Outcome>) {
        for branch in &mut self.branches {
            branch.node.cancel(cx);
        }
    }

    fn children(&self) -> Vec<Child<'_>> {
        self.branches
            .iter()
            .map(|branch| match &branch.score {
                Some(score) => Child::noted(&branch.node, format!("score {}", score.name())),
                None => Child::new(&branch.node),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use crate::prelude::*;
    use crate::testing::*;

    fn little(In(_): In<Entity>) -> f32 {
        1.0
    }

    fn lots(In(_): In<Entity>) -> f32 {
        5.0
    }

    #[test]
    fn scored_runs_the_highest_first() {
        let mut app = app();
        agent(
            &mut app,
            behavior_tree! {
                Scored => {
                    score(little) => task(Work),
                    score(lots) => task(Other),
                }
            },
        );
        app.tick();
        assert_eq!(app.count::<With<Other>>(), 1);
        assert_eq!(app.count::<With<Work>>(), 0);
    }

    #[test]
    fn scored_tries_the_next_after_a_failure() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behavior_tree! {
                Scored => {
                    score(little) => act(count),
                    score(lots) => act(refuse),
                }
            },
        );
        app.tick();
        assert_eq!(acts(&app), 2);
        assert_eq!(outcome(&app, npc), Some(true));
    }

    fn unscorable(In(_): In<Entity>) -> f32 {
        f32::NAN
    }

    #[test]
    fn a_nan_score_goes_last() {
        let mut app = app();
        agent(
            &mut app,
            behavior_tree! {
                Scored => {
                    score(unscorable) => task(Other),
                    score(little) => task(Work),
                }
            },
        );
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 1);
        assert_eq!(app.count::<With<Other>>(), 0);
    }
}
