//! Drawing how queries weighed their items.
//!
//! Every agent with a [`SurveyTrace`] has its last query drawn: each item
//! kept as a circle, from [`low`](EqsGizmos::low) to
//! [`high`](EqsGizmos::high) by its score, each dropped as a small one in
//! [`dropped`](EqsGizmos::dropped), and the one picked ringed in
//! [`picked`](EqsGizmos::picked). A color of `None` leaves that out. Turn it
//! off altogether through Bevy's `GizmoConfigStore`, as any gizmo group.

use bevy::color::palettes::css;
use bevy::prelude::*;
use bevy::transform::TransformSystems;
use bevy_arbor_eqs::survey::SurveyTrace;
use bevy_arbor_perception::Space;

/// Draws how queries weighed their items, as [`EqsGizmos`] says.
pub(crate) fn plugin(app: &mut App) {
    app.init_gizmo_group::<EqsGizmos>()
        .add_systems(PostUpdate, draw.after(TransformSystems::Propagate));
}

/// What's drawn of queries. A color of `None` leaves that out.
#[derive(Reflect, GizmoConfigGroup, Debug, Clone)]
#[reflect(Default)]
pub struct EqsGizmos {
    /// An item kept with the lowest score.
    pub low: Option<Color>,
    /// An item kept with the highest; those between are blended.
    pub high: Option<Color>,
    /// An item a filter dropped.
    pub dropped: Option<Color>,
    /// A ring round the item picked.
    pub picked: Option<Color>,
    /// How large an item is drawn.
    pub size: f32,
}

impl Default for EqsGizmos {
    fn default() -> Self {
        Self {
            low: Some(css::RED.into()),
            high: Some(css::LIME.into()),
            dropped: Some(css::GRAY.with_alpha(0.5).into()),
            picked: Some(css::WHITE.into()),
            size: 0.25,
        }
    }
}

/// Draws every traced query's items.
fn draw(mut gizmos: Gizmos<EqsGizmos>, space: Option<Res<Space>>, traces: Query<&SurveyTrace>) {
    let space = space.map_or_else(Space::default, |space| *space);
    // Circles face the way things stand on: the plane's normal, or up.
    let rotation = match space {
        Space::Planar => Quat::IDENTITY,
        Space::Volume => Quat::from_rotation_arc(Vec3::Z, Vec3::Y),
    };
    let config = gizmos.config_ext.clone();
    for trace in &traces {
        let scores = trace.items.iter().filter_map(|weighed| weighed.score);
        let (lowest, highest) = scores
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(low, high), score| {
                (low.min(score), high.max(score))
            });
        for weighed in &trace.items {
            let at = Isometry3d::new(weighed.item.position(), rotation);
            let color = match weighed.score {
                Some(score) => {
                    let along = if highest > lowest {
                        (score - lowest) / (highest - lowest)
                    } else {
                        1.0
                    };
                    match (config.low, config.high) {
                        (Some(low), Some(high)) => Some(low.mix(&high, along)),
                        (low, high) => high.or(low),
                    }
                }
                None => config.dropped,
            };
            let size = if weighed.score.is_some() {
                config.size
            } else {
                config.size / 2.0
            };
            if let Some(color) = color {
                gizmos.circle(at, size, color);
            }
            if weighed.picked
                && let Some(picked) = config.picked
            {
                gizmos.circle(at, config.size * 2.0, picked);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;

    use bevy::gizmos::{GizmoAsset, GizmoHandles};
    use bevy::prelude::*;
    use bevy_arbor_eqs::query::Item;
    use bevy_arbor_eqs::survey::{SurveyTrace, Weighed};
    use bevy_arbor_perception::prelude::*;
    use bevy_harness::TestApp;

    use super::EqsGizmos;
    use crate::ArborGizmosPlugin;

    fn gizmo_app() -> TestApp {
        TestApp::builder()
            .plugins((AssetPlugin::default(), ArborGizmosPlugin))
            // What Bevy's `GizmoPlugin` would add, short of its renderer's.
            .with(|app| {
                app.init_asset::<GizmoAsset>()
                    .insert_resource(Space::Volume);
            })
            .build()
    }

    fn drawn(app: &TestApp) -> usize {
        let handles = app.world().resource::<GizmoHandles>();
        let Some(handle) = handles.handles().get(&TypeId::of::<EqsGizmos>()) else {
            return 0;
        };
        let assets = app.world().resource::<Assets<GizmoAsset>>();
        handle
            .as_ref()
            .and_then(|handle| assets.get(handle))
            .map_or(0, |asset| {
                asset.buffer().list_positions.len() + asset.buffer().strip_positions.len()
            })
    }

    #[test]
    fn a_traced_query_is_drawn_and_nothing_else() {
        let mut app = gizmo_app();
        app.tick();
        assert_eq!(drawn(&app), 0, "nothing traced");
        let weighed = |x: f32, score: Option<f32>, picked: bool| Weighed {
            item: Item::Point(Vec3::new(x, 0.0, 0.0)),
            score,
            picked,
        };
        app.spawn(SurveyTrace {
            query: "Spot".to_owned(),
            items: vec![
                weighed(0.0, Some(1.0), true),
                weighed(1.0, Some(0.0), false),
                weighed(2.0, None, false),
            ],
        });
        app.tick();
        assert!(drawn(&app) > 0, "drawn");
    }
}
