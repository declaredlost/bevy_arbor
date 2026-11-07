//! The parts in order: what perception notices, a tree acts on the same tick.
#![cfg(all(feature = "bt", feature = "perception"))]

use std::f32::consts::PI;
use std::time::Duration;

use bevy::prelude::*;
use bevy_arbor::prelude::*;
use bevy_harness::TestApp;

#[derive(Resource, Default)]
struct Acted(u32);

fn sees_enemy(seen: &Perceived<Sight>) -> bool {
    seen.active().any(Percept::is_hostile)
}

fn attack(In(_): In<Entity>, mut acted: ResMut<Acted>) -> bool {
    acted.0 += 1;
    true
}

#[test]
fn a_tree_acts_on_what_perception_noticed_the_same_tick() {
    let mut app = TestApp::builder()
        .plugins(ArborPlugins)
        .with(|app| {
            app.init_resource::<Acted>();
        })
        .build();
    let agent = app.spawn((
        Sight::new(10.0)
            .with_half_angle(PI)
            .with_interval(Duration::ZERO),
        Team(1),
        GlobalTransform::IDENTITY,
        BehaviourTree::new(behaviour_tree! {
            Select => { when(sees_enemy) => act(attack) }
        }),
    ));
    app.spawn((
        Perceivable,
        Team(2),
        GlobalTransform::from_xyz(0.0, 0.0, -3.0),
    ));
    app.tick();
    assert_eq!(
        app.world().resource::<Acted>().0,
        1,
        "perception ran first, so the guard passed"
    );
    assert!(
        app.world()
            .get::<BehaviourTree>(agent)
            .unwrap()
            .outcome()
            .is_some()
    );
}
