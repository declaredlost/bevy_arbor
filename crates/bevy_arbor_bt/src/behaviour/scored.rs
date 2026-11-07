use bevy::prelude::*;

use super::Behaviour;
use super::ordered::Order;
use crate::behaviour::{Behave, Check, Context, Status};
use crate::tree::TreeOutline;

/// A branch of a `Scored`: a node, and the system scoring it for the agent.
pub struct ScoredBranch {
    score: Option<Check<f32>>,
    node: Behaviour,
}

impl ScoredBranch {
    /// `node`, ranked by what `system` says of the agent.
    pub fn new<S, M>(system: S, node: Behaviour) -> Self
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
    pub fn last(node: Behaviour) -> Self {
        Self { score: None, node }
    }
}

/// Its branches from the highest score down, scored as it starts, until one
/// succeeds.
pub(super) struct Scored {
    branches: Vec<ScoredBranch>,
    order: Order,
}

impl Scored {
    pub(super) fn new(branches: Vec<ScoredBranch>) -> Self {
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
    fn polls(&self) -> bool {
        false
    }

    fn tick(&mut self, cx: &mut Context) -> Status {
        if !self.order.started() {
            let order = self.rank(cx);
            self.order.start(order);
        }
        self.order
            .tick(&mut self.branches, |branch| &mut branch.node, cx)
    }

    fn cancel(&mut self, cx: &mut Context) {
        self.order.reset();
        for branch in &mut self.branches {
            branch.node.cancel(cx);
        }
    }

    fn outline(&self, out: &mut TreeOutline) {
        out.line("Scored");
        for branch in &self.branches {
            if let Some(score) = &branch.score {
                out.note(format!("score {}", score.name()));
            }
            out.child(&branch.node);
        }
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
            behaviour_tree! {
                Scored => {
                    score(little) => task("Work", Work),
                    score(lots) => task("Other", Other),
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
            behaviour_tree! {
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
            behaviour_tree! {
                Scored => {
                    score(unscorable) => task("Other", Other),
                    score(little) => task("Work", Work),
                }
            },
        );
        app.tick();
        assert_eq!(app.count::<With<Work>>(), 1);
        assert_eq!(app.count::<With<Other>>(), 0);
    }
}
