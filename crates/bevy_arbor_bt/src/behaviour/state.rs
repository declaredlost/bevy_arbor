use bevy::prelude::{Bundle, EntityWorldMut};

use super::Behaviour;
use crate::behaviour::{Behave, Context, Status};
use crate::tree::TreeOutline;

type Insert = Box<dyn Fn(&mut EntityWorldMut) + Send + Sync>;

/// Its child, with a bundle on the agent while it runs.
pub(super) struct WithState {
    name: &'static str,
    insert: Insert,
    remove: fn(&mut EntityWorldMut),
    /// Whether the bundle is on the agent.
    on: bool,
    child: Behaviour,
}

impl WithState {
    pub(super) fn new<B: Bundle + Clone>(bundle: B, child: Behaviour) -> Self {
        Self {
            name: TreeOutline::short::<B>(),
            insert: Box::new(move |agent| {
                agent.insert(bundle.clone());
            }),
            remove: |agent| {
                agent.remove::<B>();
            },
            on: false,
            child,
        }
    }

    /// Takes the bundle off the agent, if it's on.
    fn take_off(&mut self, cx: &mut Context) {
        if !std::mem::take(&mut self.on) {
            return;
        }
        let agent = cx.agent();
        if let Ok(mut agent) = cx.world_mut().get_entity_mut(agent) {
            (self.remove)(&mut agent);
        }
    }
}

impl Behave for WithState {
    fn polls(&self) -> bool {
        false
    }

    fn tick(&mut self, cx: &mut Context) -> Status {
        if !self.on {
            let agent = cx.agent();
            if let Ok(mut agent) = cx.world_mut().get_entity_mut(agent) {
                (self.insert)(&mut agent);
                self.on = true;
            }
        }
        let status = self.child.tick(cx);
        if status != Status::Running {
            self.take_off(cx);
        }
        status
    }

    fn cancel(&mut self, cx: &mut Context) {
        self.child.cancel(cx);
        self.take_off(cx);
    }

    fn outline(&self, out: &mut TreeOutline) {
        out.line(format!("State({})", self.name));
        out.child(&self.child);
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use crate::prelude::*;
    use crate::testing::*;

    #[derive(Component, Clone)]
    struct Hunting;

    #[test]
    fn its_bundle_is_on_the_agent_while_its_branch_runs() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behaviour_tree! {
                Select => {
                    when(high) => State(Hunting) => {
                        task("Work", Work),
                    },
                    task("Other", Other),
                }
            },
        );
        set(&mut app, npc, true, false);
        app.tick();
        assert!(
            app.world().get::<Hunting>(npc).is_some(),
            "on while it runs"
        );

        set(&mut app, npc, false, false);
        app.tick();
        assert!(
            app.world().get::<Hunting>(npc).is_none(),
            "off once cancelled"
        );
        assert_eq!(app.count::<With<Other>>(), 1);
    }

    #[test]
    fn its_bundle_comes_off_when_its_child_ends() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behaviour_tree! {
                Sequence => {
                    State(Hunting) => { task("Work", (Work, Report::Succeed)) },
                    task("Other", Other),
                }
            },
        );
        app.tick();
        assert!(app.world().get::<Hunting>(npc).is_some());
        app.tick();
        assert!(app.world().get::<Hunting>(npc).is_none());
        assert_eq!(app.count::<With<Other>>(), 1);
    }
}
