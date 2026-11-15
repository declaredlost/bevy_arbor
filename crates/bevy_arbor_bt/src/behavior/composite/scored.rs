use super::ordered::Order;
use crate::behavior::{Behave, Behavior, Child, Context, Outcome, Status};
use crate::decorator::{AgentFn, Typed};

/// How much a branch of a `Scored` is worth to the agent, as it starts.
///
/// Most are functions of the agent's components, typed scores (see
/// [`IntoScore`]). One of your own reads anything, through
/// [`Context::world`].
pub trait Score: Send + Sync + 'static {
    /// What it's worth: `None`, or NaN, goes last.
    fn score(&mut self, cx: &mut Context) -> Option<f32>;

    /// How it reads in a trace. Built only for a trace.
    fn label(&self) -> String;
}

/// What a [`Score`] can be made from: a score, or a function of the agent's
/// components, taking `&T` or `Option<&T>` of up to four, and returning `f32`.
/// One taking `&T` the agent hasn't got goes last.
#[diagnostic::on_unimplemented(
    message = "`{Self}` isn't a score",
    label = "not a score",
    note = "a score is a function of the agent's components (`fn(&Health) -> f32`), or a `Score` of your own"
)]
pub trait IntoScore<M> {
    /// The score it is.
    fn into_score(self) -> Box<dyn Score>;
}

impl<S: Score> IntoScore<()> for S {
    fn into_score(self) -> Box<dyn Score> {
        Box::new(self)
    }
}

impl<F: AgentFn<P, f32>, P: 'static> IntoScore<fn(P) -> f32> for F {
    fn into_score(self) -> Box<dyn Score> {
        Box::new(Typed::<F, P>::new(self))
    }
}

impl<F: AgentFn<P, f32>, P: 'static> Score for Typed<F, P> {
    fn score(&mut self, cx: &mut Context) -> Option<f32> {
        self.read(cx)
    }

    fn label(&self) -> String {
        self.name()
    }
}

/// A branch of a `Scored`: a node, and what scores it for the agent.
pub struct ScoredBranch {
    score: Option<Box<dyn Score>>,
    node: Behavior,
}

impl ScoredBranch {
    /// `node`, ranked by what `score` says of the agent.
    pub fn new<M>(score: impl IntoScore<M>, node: Behavior) -> Self {
        Self {
            score: Some(score.into_score()),
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
                    .and_then(|score| score.score(cx))
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
                Some(score) => Child::noted(&branch.node, format!("score {}", score.label())),
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

    fn little(_: &Flags) -> f32 {
        1.0
    }

    fn lots(_: &Flags) -> f32 {
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

    fn unscorable(_: &Flags) -> f32 {
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
