use std::fmt::Debug;
use std::marker::PhantomData;

use bevy::prelude::*;
use bevy::utils::prelude::ShortName;

use super::ordered::Order;
use crate::behavior::system::{Check, Param, WithParam};
use crate::behavior::{Behave, Behavior, Context, Outcome, Status};
use crate::condition::AgentFn;
use crate::inspect::{Inspect, Inspection, label};

/// What ranks a branch of a `Scored` for the agent: higher goes first.
///
/// Most are score systems or functions of the agent's components (see
/// [`IntoScore`]); one of your own reads anything through the [`Context`].
pub trait Score: Inspect + Send + Sync + 'static {
    /// The branch's score for the agent of `cx`: `None`, or NaN, if it has
    /// none, and goes last.
    fn score(&mut self, cx: &mut Context) -> Option<f32>;
}

/// What a [`Score`] can be made from: a system, a function, or a score.
///
/// A score system takes the agent as `In<Entity>` and returns `f32`; it can
/// read anything, and change things. It's kept by its type, as an act is.
///
/// A function of the agent's components, a typed score, takes `&T` or
/// `Option<&T>` of up to four, and returns `f32`: one taking `&T` the agent
/// hasn't got has no score, and goes last.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_arbor_bt::prelude::*;
/// # #[derive(Component, Clone)] struct Eat;
/// # #[derive(Component, Clone)] struct Sleep;
/// #[derive(Component)]
/// struct Hunger(f32);
///
/// #[derive(Resource)]
/// struct Night(bool);
///
/// fn hungry(hunger: &Hunger) -> f32 {
///     hunger.0
/// }
///
/// fn sleepy(In(_): In<Entity>, night: Res<Night>) -> f32 {
///     if night.0 { 1.0 } else { 0.0 }
/// }
///
/// let tree = behavior_tree! {
///     Scored => {
///         score(hungry) => task(Eat),
///         score(sleepy) => task(Sleep),
///     }
/// };
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` isn't a score",
    label = "not a score",
    note = "a score is a system taking `In<Entity>` and returning `f32`, a function of the agent's components (`fn(&Hunger) -> f32`), or a `Score` of your own"
)]
pub trait IntoScore<M> {
    /// It, as a score.
    fn into_score(self) -> impl Score;
}

impl<S: Score> IntoScore<()> for S {
    fn into_score(self) -> impl Score {
        self
    }
}

/// A score system on the agent, as a score.
struct ScoreSystem(Check<f32>);

impl Score for ScoreSystem {
    fn score(&mut self, cx: &mut Context) -> Option<f32> {
        self.0.run(cx)
    }
}

impl Inspect for ScoreSystem {
    fn inspect(&self, inspection: &mut Inspection<'_>) {
        inspection.name(self.0.name());
    }
}

impl<S, M> IntoScore<fn(M)> for S
where
    S: IntoSystem<In<Entity>, f32, M> + Copy + Send + Sync + 'static,
{
    fn into_score(self) -> impl Score {
        ScoreSystem(Check::new(self))
    }
}

/// A score system taking the agent and a parameter, made by
/// [`with`](crate::behavior::with): `score(with(distance_to, home))`.
impl<S, P, M> IntoScore<Param<M>> for WithParam<S, P>
where
    S: IntoSystem<In<(Entity, P)>, f32, M> + Copy + Send + Sync + 'static,
    P: Clone + Debug + Send + Sync + 'static,
{
    fn into_score(self) -> impl Score {
        ScoreSystem(Check::with(self.system, self.param))
    }
}

/// A function of the agent's components, as a score.
struct TypedScore<F, P> {
    function: F,
    params: PhantomData<fn() -> P>,
}

impl<F: AgentFn<P, f32>, P: 'static> Score for TypedScore<F, P> {
    fn score(&mut self, cx: &mut Context) -> Option<f32> {
        let agent = cx.world().get_entity(cx.agent()).ok()?;
        self.function.call(agent)
    }
}

impl<F, P> Inspect for TypedScore<F, P> {
    fn inspect(&self, inspection: &mut Inspection<'_>) {
        inspection.name(ShortName(std::any::type_name::<F>()));
    }
}

impl<F: AgentFn<P, f32>, P: 'static> IntoScore<fn(P) -> f32> for F {
    fn into_score(self) -> impl Score {
        TypedScore {
            function: self,
            params: PhantomData,
        }
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
            score: Some(Box::new(score.into_score())),
            node,
        }
    }

    /// `node`, after every branch with a score.
    #[must_use]
    pub fn otherwise(node: Behavior) -> Self {
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

    /// The order to try them in: highest score first, and the unscored last.
    ///
    /// One that can't be scored, isn't, or scores NaN goes last.
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
}

impl Inspect for Scored {
    fn inspect(&self, inspection: &mut Inspection<'_>) {
        inspection.name("Scored");
        for branch in &self.branches {
            match &branch.score {
                Some(score) => {
                    inspection.noted(format_args!("score {}", label(&**score)), &branch.node)
                }
                None => inspection.child(&branch.node),
            };
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

    #[derive(Component)]
    struct Hunger(f32);

    fn hungry(hunger: &Hunger) -> f32 {
        hunger.0
    }

    #[test]
    fn a_typed_score_reads_the_agent_and_one_missing_goes_last() {
        let mut app = app();
        let tree = || {
            behavior_tree! {
                Scored => {
                    score(little) => task(Work),
                    score(hungry) => task(Other),
                }
            }
        };
        let hungry = app.spawn((Hunger(3.0), BehaviorTree::new(tree())));
        let fed = app.spawn(BehaviorTree::new(tree()));
        app.tick();
        let doing = |app: &mut bevy_harness::TestApp, agent: Entity| {
            let mut tasks = app.world_mut().query::<(&TaskOf, Has<Work>)>();
            tasks
                .iter(app.world())
                .find(|(of, _)| of.agent() == agent)
                .map(|(_, work)| work)
        };
        assert_eq!(doing(&mut app, hungry), Some(false), "hunger 3 beats 1");
        assert_eq!(doing(&mut app, fed), Some(true), "no hunger goes last");
    }

    fn given(In((_, score)): In<(Entity, f32)>) -> f32 {
        score
    }

    fn add(In((_, by)): In<(Entity, u32)>, mut acts: ResMut<Acts>) -> bool {
        acts.0 += by;
        true
    }

    #[test]
    fn with_gives_a_score_or_an_act_its_parameter() {
        let mut app = app();
        agent(
            &mut app,
            behavior_tree! {
                Scored => {
                    score(with(given, 1.0)) => act(with(add, 1)),
                    score(with(given, 2.0)) => act(with(add, 10)),
                }
            },
        );
        app.tick();
        assert_eq!(acts(&app), 10, "the higher went first, and succeeded");
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
