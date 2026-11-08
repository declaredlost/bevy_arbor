//! Drawing where agents are going: each agent's [`Destination`], ringed by
//! how near counts as there, and the [`Route`] its movement says it's
//! taking. A color of `None` leaves that out. Turn it off altogether through
//! Bevy's `GizmoConfigStore`, as any gizmo group.

use bevy::color::palettes::css;
use bevy::prelude::*;
use bevy::transform::TransformSystems;
use bevy_arbor_nav::travel::{Destination, Route};
use bevy_arbor_perception::Space;

/// Draws where agents are going, as [`NavGizmos`] says.
pub(crate) fn plugin(app: &mut App) {
    app.init_gizmo_group::<NavGizmos>()
        .add_systems(PostUpdate, draw.after(TransformSystems::Propagate));
}

/// What's drawn of navigation. A color of `None` leaves that out.
#[derive(Reflect, GizmoConfigGroup, Debug, Clone)]
#[reflect(Default)]
pub struct NavGizmos {
    /// A ring round each destination, as wide as how near counts as there.
    pub destination: Option<Color>,
    /// The way each agent's taking, from it through its route's points.
    pub route: Option<Color>,
}

impl Default for NavGizmos {
    fn default() -> Self {
        Self {
            destination: Some(css::AQUA.into()),
            route: Some(css::AQUA.with_alpha(0.5).into()),
        }
    }
}

/// Draws every agent's destination and route.
fn draw(
    mut gizmos: Gizmos<NavGizmos>,
    space: Option<Res<Space>>,
    agents: Query<
        (&GlobalTransform, Option<&Destination>, Option<&Route>),
        Or<(With<Destination>, With<Route>)>,
    >,
    places: Query<&GlobalTransform>,
) {
    let space = space.map_or_else(Space::default, |space| *space);
    // Rings face the way things stand on: the plane's normal, or up.
    let rotation = match space {
        Space::Planar => Quat::IDENTITY,
        Space::Volume => Quat::from_rotation_arc(Vec3::Z, Vec3::Y),
    };
    let config = gizmos.config_ext.clone();
    for (transform, destination, route) in &agents {
        if let (Some(color), Some(route)) = (config.route, route)
            && !route.0.is_empty()
        {
            gizmos.linestrip(
                std::iter::once(transform.translation()).chain(route.0.iter().copied()),
                color,
            );
        }
        if let (Some(color), Some(destination)) = (config.destination, destination)
            && let Some(at) = destination.goal.position(&places)
        {
            // Not so small it can't be seen.
            let radius = destination.within.max(0.1);
            gizmos.circle(Isometry3d::new(at, rotation), radius, color);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;

    use bevy::gizmos::{GizmoAsset, GizmoHandles};
    use bevy::prelude::*;
    use bevy_arbor_nav::travel::{Destination, Route};
    use bevy_arbor_perception::Space;
    use bevy_harness::TestApp;

    use super::NavGizmos;
    use crate::ArborGizmosPlugin;

    fn gizmo_app(gizmos: NavGizmos) -> TestApp {
        TestApp::builder()
            .plugins((AssetPlugin::default(), ArborGizmosPlugin))
            // What Bevy's `GizmoPlugin` would add, short of its renderer's.
            .with(move |app| {
                app.init_asset::<GizmoAsset>()
                    .insert_gizmo_config(gizmos, GizmoConfig::default())
                    .insert_resource(Space::Volume);
            })
            .build()
    }

    /// How many vertices were drawn last frame: lines and strips.
    fn drawn(app: &TestApp) -> usize {
        let world = app.world();
        let Some(Some(handle)) = world
            .resource::<GizmoHandles>()
            .handles()
            .get(&TypeId::of::<NavGizmos>())
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

    fn going(app: &mut TestApp) {
        app.spawn((
            GlobalTransform::IDENTITY,
            Destination::point(Vec3::X * 5.0),
            Route(vec![Vec3::X * 2.0, Vec3::X * 5.0]),
        ));
        app.tick();
    }

    #[test]
    fn a_destination_and_its_route_are_drawn() {
        let mut app = gizmo_app(NavGizmos::default());
        app.tick();
        assert_eq!(drawn(&app), 0, "nowhere to go");
        going(&mut app);
        assert!(drawn(&app) > 0);
    }

    #[test]
    fn a_kind_without_a_color_is_left_out() {
        let mut every = gizmo_app(NavGizmos::default());
        going(&mut every);
        let mut fewer = gizmo_app(NavGizmos {
            route: None,
            ..NavGizmos::default()
        });
        going(&mut fewer);
        assert!(drawn(&fewer) < drawn(&every));
    }
}
