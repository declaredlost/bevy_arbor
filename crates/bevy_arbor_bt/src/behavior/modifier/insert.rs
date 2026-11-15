use bevy::prelude::{Bundle, EntityWorldMut};
use bevy::utils::prelude::ShortName;

use crate::behavior::{Behave, Behavior, Context, Outcome, Status};
use crate::inspect::{Inspect, Inspection};

type Add = Box<dyn Fn(&mut EntityWorldMut) + Send + Sync>;

/// Its child, with a bundle on the agent while it runs.
pub(in crate::behavior) struct Insert {
    name: String,
    add: Add,
    remove: fn(&mut EntityWorldMut),
    child: Behavior,
}

impl Insert {
    pub(in crate::behavior) fn new<B: Bundle + Clone>(bundle: B, child: Behavior) -> Self {
        Self {
            name: ShortName::of::<B>().to_string(),
            add: Box::new(move |agent| {
                agent.insert(bundle.clone());
            }),
            remove: |agent| {
                agent.remove::<B>();
            },
            child,
        }
    }
}

impl Behave for Insert {
    fn start(&mut self, cx: &mut Context) {
        let agent = cx.agent();
        if let Ok(mut agent) = cx.world_mut().get_entity_mut(agent) {
            (self.add)(&mut agent);
        }
    }

    fn tick(&mut self, cx: &mut Context) -> Status {
        self.child.tick(cx)
    }

    fn end(&mut self, cx: &mut Context, _outcome: Option<Outcome>) {
        self.child.cancel(cx);
        let agent = cx.agent();
        if let Ok(mut agent) = cx.world_mut().get_entity_mut(agent) {
            (self.remove)(&mut agent);
        }
    }
}

impl Inspect for Insert {
    fn inspect(&self, inspection: &mut Inspection<'_>) {
        inspection
            .name("Insert")
            .param(&self.name)
            .child(&self.child);
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
        let npc = with_flags(
            &mut app,
            behavior_tree! {
                Select => {
                    when(high) => Insert(Hunting) => {
                        task(Work),
                    },
                    task(Other),
                }
            },
            true,
            false,
        );
        app.tick();
        assert!(
            app.world().get::<Hunting>(npc).is_some(),
            "on while it runs"
        );

        set(&mut app, npc, false, false);
        app.tick();
        assert!(
            app.world().get::<Hunting>(npc).is_none(),
            "off once canceled"
        );
        assert_eq!(app.count::<With<Other>>(), 1);
    }

    #[test]
    fn its_bundle_comes_off_when_its_child_ends() {
        let mut app = app();
        let npc = agent(
            &mut app,
            behavior_tree! {
                Sequence => {
                    Insert(Hunting) => { task((Work, Report::Succeed)) },
                    task(Other),
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
