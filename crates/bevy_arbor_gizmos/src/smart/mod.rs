//! Drawing slots: free, claimed, or disabled.
//!
//! Every slot is drawn as a circle where it is: in [`free`](SmartGizmos::free),
//! [`claimed`](SmartGizmos::claimed) with a line to the agent holding it, or
//! [`disabled`](SmartGizmos::disabled). A color of `None` leaves that out.
//! Turn it off altogether through Bevy's `GizmoConfigStore`, as any gizmo
//! group.

use bevy::color::palettes::css;
use bevy::prelude::*;
use bevy::transform::TransformSystems;
use bevy_arbor_perception::Space;
use bevy_arbor_smart::claim::ClaimedBy;
use bevy_arbor_smart::slot::{SlotDisabled, SlotOf};

/// Draws slots, as [`SmartGizmos`] says.
pub(crate) fn plugin(app: &mut App) {
    app.init_gizmo_group::<SmartGizmos>()
        .add_systems(PostUpdate, draw.after(TransformSystems::Propagate));
}

/// What's drawn of slots. A color of `None` leaves that out.
#[derive(Reflect, GizmoConfigGroup, Debug, Clone)]
#[reflect(Default)]
pub struct SmartGizmos {
    /// A slot no one holds.
    pub free: Option<Color>,
    /// A slot an agent holds, and a line to it.
    pub claimed: Option<Color>,
    /// A disabled slot.
    pub disabled: Option<Color>,
    /// How large a slot is drawn.
    pub size: f32,
}

impl Default for SmartGizmos {
    fn default() -> Self {
        Self {
            free: Some(css::LIME.into()),
            claimed: Some(css::ORANGE.into()),
            disabled: Some(css::GRAY.with_alpha(0.5).into()),
            size: 0.3,
        }
    }
}

/// Draws every slot.
fn draw(
    mut gizmos: Gizmos<SmartGizmos>,
    space: Option<Res<Space>>,
    slots: Query<(&GlobalTransform, Option<&ClaimedBy>, Has<SlotDisabled>), With<SlotOf>>,
    places: Query<&GlobalTransform>,
) {
    let space = space.map_or_else(Space::default, |space| *space);
    // Circles face the way things stand on: the plane's normal, or up.
    let rotation = match space {
        Space::Planar => Quat::IDENTITY,
        Space::Volume => Quat::from_rotation_arc(Vec3::Z, Vec3::Y),
    };
    let config = gizmos.config_ext.clone();
    for (transform, claimed, disabled) in &slots {
        let at = transform.translation();
        let color = match (disabled, claimed) {
            (true, _) => config.disabled,
            (false, Some(_)) => config.claimed,
            (false, None) => config.free,
        };
        let Some(color) = color else {
            continue;
        };
        gizmos.circle(Isometry3d::new(at, rotation), config.size, color);
        if let Some(agent) = claimed.and_then(|claimed| places.get(claimed.agent()).ok()) {
            gizmos.line(agent.translation(), at, color);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;

    use bevy::gizmos::{GizmoAsset, GizmoHandles};
    use bevy::prelude::*;
    use bevy_arbor_perception::Space;
    use bevy_arbor_smart::claim::ClaimedBy;
    use bevy_arbor_smart::slot::{SlotDisabled, SlotOf};
    use bevy_harness::TestApp;

    use super::SmartGizmos;
    use crate::ArborGizmosPlugin;

    fn gizmo_app(gizmos: SmartGizmos) -> TestApp {
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
    fn drawn(app: &TestApp) -> (usize, usize) {
        let world = app.world();
        let Some(Some(handle)) = world
            .resource::<GizmoHandles>()
            .handles()
            .get(&TypeId::of::<SmartGizmos>())
        else {
            return (0, 0);
        };
        world
            .resource::<Assets<GizmoAsset>>()
            .get(handle)
            .map_or((0, 0), |asset| {
                (
                    asset.buffer().list_positions.len(),
                    asset.buffer().strip_positions.len(),
                )
            })
    }

    /// A free slot, a claimed one, and a disabled one.
    fn scene(app: &mut TestApp) {
        let object = app.spawn(GlobalTransform::IDENTITY);
        let agent = app.spawn(GlobalTransform::from_xyz(0.0, 0.0, 5.0));
        app.spawn((SlotOf(object), GlobalTransform::from_xyz(1.0, 0.0, 0.0)));
        app.spawn((
            SlotOf(object),
            ClaimedBy(agent),
            GlobalTransform::from_xyz(2.0, 0.0, 0.0),
        ));
        app.spawn((
            SlotOf(object),
            SlotDisabled,
            GlobalTransform::from_xyz(3.0, 0.0, 0.0),
        ));
        app.tick();
    }

    #[test]
    fn every_slot_is_drawn_and_a_claimed_one_has_a_line_to_its_agent() {
        let mut app = gizmo_app(SmartGizmos::default());
        scene(&mut app);
        let (lines, circles) = drawn(&app);
        assert_eq!(lines, 2, "one line: to the claiming agent");
        assert!(circles > 0, "the slots");
    }

    #[test]
    fn a_kind_without_a_color_is_left_out() {
        let mut every = gizmo_app(SmartGizmos::default());
        scene(&mut every);
        let mut fewer = gizmo_app(SmartGizmos {
            claimed: None,
            ..SmartGizmos::default()
        });
        scene(&mut fewer);
        assert_eq!(drawn(&fewer).0, 0, "no claimed slot, no line");
        assert!(drawn(&fewer).1 < drawn(&every).1, "one circle fewer");
    }
}
