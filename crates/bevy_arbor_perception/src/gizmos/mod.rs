//! Drawing what agents perceive: sight cones, hearing ranges, lines to what's
//! in view and crosses where what's remembered was. Behind the `gizmos`
//! feature.
//!
//! As Bevy's own gizmos do: [`PerceptionGizmos`] says what's drawn, each kind
//! with a colour or none, and for every agent or only those with
//! [`ShowPerception`]. Turn it off altogether through Bevy's
//! `GizmoConfigStore`, as any gizmo group.

mod hearing;
mod percepts;
mod shape;
mod sight;

use bevy::color::palettes::css;
use bevy::prelude::*;
use bevy::transform::TransformSystems;

use crate::sense::{Hearing, Shared, Sight, Touch};

/// Draws what agents perceive, as [`PerceptionGizmos`] says. Needs Bevy's
/// gizmos (`GizmoPlugin`, in `DefaultPlugins`).
pub struct PerceptionGizmosPlugin;

impl Plugin for PerceptionGizmosPlugin {
    fn build(&self, app: &mut App) {
        app.init_gizmo_group::<PerceptionGizmos>()
            .register_type::<ShowPerception>()
            .add_systems(
                PostUpdate,
                (
                    sight::draw,
                    hearing::draw,
                    percepts::draw::<Sight>,
                    percepts::draw::<Hearing>,
                    percepts::draw::<Touch>,
                    percepts::draw::<Shared>,
                )
                    .after(TransformSystems::Propagate),
            );
    }
}

/// What's drawn of perception. A colour of `None` leaves that out.
#[derive(Reflect, GizmoConfigGroup, Debug, Clone)]
#[reflect(Default)]
pub struct PerceptionGizmos {
    /// Every agent's, or only those with [`ShowPerception`].
    pub draw_all: bool,
    /// The sight cone, out to `range`.
    pub sight: Option<Color>,
    /// The sight cone out to `lose_range`, where it's further.
    pub lose_sight: Option<Color>,
    /// The hearing range.
    pub hearing: Option<Color>,
    /// A line to what's in view and hostile.
    pub hostile: Option<Color>,
    /// A line to what's in view and friendly.
    pub friendly: Option<Color>,
    /// A line to what's in view and neutral.
    pub neutral: Option<Color>,
    /// A cross where what's remembered was last sensed.
    pub remembered: Option<Color>,
    /// How large that cross is.
    pub cross_size: f32,
}

impl Default for PerceptionGizmos {
    /// Everything, for every agent.
    fn default() -> Self {
        Self {
            draw_all: true,
            sight: Some(css::YELLOW.into()),
            lose_sight: Some(css::YELLOW.with_alpha(0.35).into()),
            hearing: Some(css::DEEP_SKY_BLUE.into()),
            hostile: Some(css::RED.into()),
            friendly: Some(css::LIME.into()),
            neutral: Some(css::WHITE.into()),
            remembered: Some(css::GRAY.into()),
            cross_size: 0.5,
        }
    }
}

impl PerceptionGizmos {
    /// Whether an agent's is drawn: every agent's, or it's [`ShowPerception`].
    fn shows(&self, shown: bool) -> bool {
        self.draw_all || shown
    }
}

/// On an agent: draw what it perceives, where [`PerceptionGizmos::draw_all`]
/// is off.
#[derive(Component, Reflect, Debug, Clone, Copy, Default)]
#[reflect(Component, Debug, Default)]
pub struct ShowPerception;

#[cfg(test)]
mod tests {
    use std::any::TypeId;
    use std::time::Duration;

    use bevy::gizmos::{GizmoAsset, GizmoHandles};
    use bevy::prelude::*;
    use bevy_harness::TestApp;

    use super::{PerceptionGizmos, PerceptionGizmosPlugin, ShowPerception};
    use crate::prelude::*;
    use crate::testing::*;

    fn gizmo_app(space: Space, gizmos: PerceptionGizmos) -> TestApp {
        TestApp::builder()
            .plugins((
                AssetPlugin::default(),
                PerceptionPlugin::default().with_space(space),
                PerceptionGizmosPlugin,
            ))
            // What Bevy's `GizmoPlugin` would add, short of its renderer's.
            .with(move |app| {
                app.init_asset::<GizmoAsset>()
                    .insert_gizmo_config(gizmos, GizmoConfig::default());
            })
            .build()
    }

    /// How many vertices perception drew last frame.
    fn drawn(app: &TestApp) -> usize {
        let world = app.world();
        let Some(Some(handle)) = world
            .resource::<GizmoHandles>()
            .handles()
            .get(&TypeId::of::<PerceptionGizmos>())
        else {
            return 0;
        };
        world
            .resource::<Assets<GizmoAsset>>()
            .get(handle)
            .map_or(0, |asset| {
                asset.buffer().list_positions.len() + asset.buffer().strip_positions.len()
            })
    }

    /// An agent that sees and hears, an enemy it sees, and something it
    /// remembers.
    fn scene(app: &mut TestApp, space: Space, shown: bool) -> Entity {
        let agent = app.spawn((
            Sight::new(10.0)
                .with_lose_range(12.0)
                .with_interval(Duration::ZERO),
            Hearing::new(8.0),
            Team(1),
            GlobalTransform::IDENTITY,
        ));
        if shown {
            app.entity_mut(agent).insert(ShowPerception);
        }
        app.spawn((
            Perceivable,
            Team(2),
            GlobalTransform::from_translation(ahead(space, 5.0)),
        ));
        let gone = thing(app, ahead(space, 6.0));
        app.tick();
        app.entity_mut(gone)
            .insert(GlobalTransform::from_translation(ahead(space, 50.0)));
        app.ticks(2);
        agent
    }

    #[test]
    fn it_draws_everything_for_every_agent_by_default() {
        for space in SPACES {
            let mut app = gizmo_app(space, PerceptionGizmos::default());
            scene(&mut app, space, false);
            assert!(drawn(&app) > 0, "{space:?}");
        }
    }

    #[test]
    fn a_colour_of_none_leaves_that_out() {
        for space in SPACES {
            let all = {
                let mut app = gizmo_app(space, PerceptionGizmos::default());
                scene(&mut app, space, false);
                drawn(&app)
            };
            let mut app = gizmo_app(
                space,
                PerceptionGizmos {
                    hearing: None,
                    ..PerceptionGizmos::default()
                },
            );
            scene(&mut app, space, false);
            assert!(drawn(&app) < all, "{space:?}: no hearing ring");
        }
    }

    #[test]
    fn without_draw_all_only_agents_shown_are_drawn() {
        for space in SPACES {
            let only_shown = PerceptionGizmos {
                draw_all: false,
                ..PerceptionGizmos::default()
            };
            let mut hidden = gizmo_app(space, only_shown.clone());
            scene(&mut hidden, space, false);
            assert_eq!(drawn(&hidden), 0, "{space:?}: not shown");
            let mut shown = gizmo_app(space, only_shown);
            scene(&mut shown, space, true);
            assert!(drawn(&shown) > 0, "{space:?}: shown");
        }
    }
}
