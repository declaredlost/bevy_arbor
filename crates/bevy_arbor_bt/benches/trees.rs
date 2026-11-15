//! What ticking trees costs, for many agents with a tree shaped like a
//! monster's: a `Select` of four decorators, the last branch's task running.
//!
//! - `awake`: system conditions, asked every tick, so every tree is awake.
//! - `asleep`: typed conditions with nothing changing, so trees sleep.
//! - `churn`: typed conditions, a tenth of the agents' moods changing every
//!   tick, so those wake.
//! - `task_done`: typed conditions, the task finishing every tick, so every
//!   tree wakes, but its conditions' inputs don't change.
//!
//! Each measures one frame. `cargo bench -p bevy_arbor_bt --bench trees`.

use bevy::prelude::*;
use bevy_arbor_bt::prelude::*;
use criterion::{Criterion, criterion_group, criterion_main};

#[derive(Component)]
struct Mood(bool);

#[derive(Component, Clone)]
struct Roam;

#[derive(Component, Clone)]
struct Blink;

fn first(In(agent): In<Entity>, moods: Query<&Mood>) -> bool {
    moods.get(agent).is_ok_and(|mood| mood.0)
}

fn second(In(agent): In<Entity>, moods: Query<&Mood>) -> bool {
    moods.get(agent).is_ok_and(|mood| mood.0)
}

fn third(In(agent): In<Entity>, moods: Query<&Mood>) -> bool {
    moods.get(agent).is_ok_and(|mood| mood.0)
}

fn fourth(In(agent): In<Entity>, moods: Query<&Mood>) -> bool {
    moods.get(agent).is_ok_and(|mood| !mood.0)
}

fn systems() -> Behavior {
    behavior_tree! {
        Forever => {
            Select => {
                when(first) => task(Roam),
                when(second) => task(Roam),
                when(third) => task(Roam),
                when(fourth) => task(Roam),
            }
        }
    }
}

fn first_typed(mood: &Mood) -> bool {
    mood.0
}

fn second_typed(mood: &Mood) -> bool {
    mood.0
}

fn third_typed(mood: &Mood) -> bool {
    mood.0
}

fn fourth_typed(mood: &Mood) -> bool {
    !mood.0
}

fn typed() -> Behavior {
    behavior_tree! {
        Forever => {
            Select => {
                when(first_typed) => task(Roam),
                when(second_typed) => task(Roam),
                when(third_typed) => task(Roam),
                when(fourth_typed) => task(Roam),
            }
        }
    }
}

fn blinking() -> Behavior {
    behavior_tree! {
        Forever => {
            Select => {
                when(first_typed) => task(Roam),
                when(second_typed) => task(Roam),
                when(third_typed) => task(Roam),
                when(fourth_typed) => task(Blink),
            }
        }
    }
}

/// Touches the mood of a tenth of the agents, a different tenth each tick,
/// leaving it as it was: a change, as far as trees can tell.
fn churn(mut moods: Query<&mut Mood>, mut round: Local<usize>) {
    *round = (*round + 1) % 10;
    for (index, mut mood) in moods.iter_mut().enumerate() {
        if index % 10 == *round {
            mood.set_changed();
        }
    }
}

/// Finishes every `Blink` task the tick it's there.
fn blink(mut tasks: Query<&mut Task, With<Blink>>) {
    for mut task in &mut tasks {
        task.succeed();
    }
}

/// An app of `agents` running `tree`, past its first frame.
fn app(agents: usize, tree: fn() -> Behavior, setup: impl FnOnce(&mut App)) -> App {
    let mut app = App::new();
    app.init_resource::<Time>()
        .add_plugins(TreePlugin::new(Update));
    setup(&mut app);
    for _ in 0..agents {
        app.world_mut()
            .spawn((Mood(false), BehaviorTree::new(tree())));
    }
    // Started, so what's measured is the steady state.
    app.update();
    app.update();
    app
}

fn bench_trees(c: &mut Criterion) {
    let mut group = c.benchmark_group("trees");
    for agents in [2_000, 10_000] {
        let k = agents / 1_000;
        let mut awake = app(agents, systems, |_| {});
        group.bench_function(format!("awake_{k}k"), |b| b.iter(|| awake.update()));
        let mut asleep = app(agents, typed, |_| {});
        group.bench_function(format!("asleep_{k}k"), |b| b.iter(|| asleep.update()));
        let mut churning = app(agents, typed, |app| {
            app.add_systems(Update, churn.before(TreeSystems));
        });
        group.bench_function(format!("churn_{k}k"), |b| b.iter(|| churning.update()));
        let mut done = app(agents, blinking, |app| {
            app.add_systems(Update, blink.after(TreeSystems));
        });
        group.bench_function(format!("task_done_{k}k"), |b| b.iter(|| done.update()));
    }
    group.finish();
}

criterion_group!(benches, bench_trees);
criterion_main!(benches);
