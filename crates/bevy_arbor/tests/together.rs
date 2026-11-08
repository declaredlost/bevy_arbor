//! Trees and perception together: a tree sleeps on what its agent perceives,
//! and wakes as that changes, by the sense its decorators read.
#![cfg(all(feature = "bt", feature = "perception"))]

use std::f32::consts::PI;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use bevy::prelude::*;
use bevy_arbor::prelude::*;
use bevy_harness::TestApp;

static ASKED: AtomicUsize = AtomicUsize::new(0);

fn sees_enemy(seen: &Perceived<Sight>) -> bool {
    ASKED.fetch_add(1, Ordering::Relaxed);
    seen.active().any(Percept::is_hostile)
}

#[derive(Component, Clone)]
struct Chase;

#[derive(Component, Clone)]
struct Wander;

fn asked() -> usize {
    ASKED.load(Ordering::Relaxed)
}

#[test]
fn a_tree_sleeps_on_what_its_agent_sees_and_wakes_as_that_changes() {
    let mut app = TestApp::builder().plugins(ArborPlugins).build();
    app.spawn((
        Sight::new(10.0)
            .with_half_angle(PI)
            .with_interval(Duration::ZERO),
        Hearing::new(30.0),
        Team(1),
        GlobalTransform::IDENTITY,
        BehaviorTree::new(behavior_tree! {
            Forever => {
                Select => {
                    when(sees_enemy) => task(Chase),
                    task(Wander),
                }
            }
        }),
    ));
    let enemy = app.spawn((
        Perceivable,
        Team(2),
        GlobalTransform::from_xyz(0.0, 0.0, -20.0),
    ));
    app.tick();
    assert_eq!(app.count::<With<Wander>>(), 1, "nothing in view: wander");

    // Nothing changes: the tree sleeps, its decorator isn't asked.
    let before = asked();
    app.ticks(10);
    assert_eq!(asked(), before, "asleep while nothing's seen");

    // A noise changes what it hears, not what it sees: still asleep.
    app.write(Noise::new(enemy, Vec3::new(0.0, 0.0, -20.0)));
    app.ticks(2);
    assert_eq!(asked(), before, "a noise doesn't wake a tree reading sight");

    // The enemy comes into view: woken, and chasing the same tick.
    app.entity_mut(enemy)
        .insert(GlobalTransform::from_xyz(0.0, 0.0, -5.0));
    app.tick();
    assert_eq!(app.count::<With<Chase>>(), 1, "chasing");
    assert_eq!(app.count::<With<Wander>>(), 0);

    // It moves about in view: what it sees doesn't change, so asleep again.
    let chasing = asked();
    for z in [-4.0, -6.0, -3.0] {
        app.entity_mut(enemy)
            .insert(GlobalTransform::from_xyz(0.0, 0.0, z));
        app.tick();
    }
    assert_eq!(asked(), chasing, "asleep while chasing what's in view");

    // Out of view: woken, wandering again.
    app.entity_mut(enemy)
        .insert(GlobalTransform::from_xyz(0.0, 0.0, -50.0));
    app.tick();
    assert_eq!(app.count::<With<Wander>>(), 1, "lost it: wander");
    assert_eq!(app.count::<With<Chase>>(), 0);
}
