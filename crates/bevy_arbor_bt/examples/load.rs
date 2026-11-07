//! What ticking trees costs, with a tree shaped like a monster's: a `Select` of
//! four guards, the last branch's task running.
//!
//! Three ways: system guards, asked every tick; typed guards with nothing
//! changing, so trees sleep; and typed guards with a tenth of the agents'
//! moods changing every tick, so those wake.
//!
//! `cargo run --release -p bevy_arbor_bt --example load`

use std::time::Instant;

use bevy::prelude::*;
use bevy_arbor_bt::prelude::*;

#[derive(Component)]
struct Mood(bool);

#[derive(Component, Clone)]
struct Roam;

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

fn systems() -> Behaviour {
    behaviour_tree! {
        Forever => {
            Select => {
                when(first) => task("Roam", Roam),
                when(second) => task("Roam", Roam),
                when(third) => task("Roam", Roam),
                when(fourth) => task("Roam", Roam),
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

fn typed() -> Behaviour {
    behaviour_tree! {
        Forever => {
            Select => {
                when(first_typed) => task("Roam", Roam),
                when(second_typed) => task("Roam", Roam),
                when(third_typed) => task("Roam", Roam),
                when(fourth_typed) => task("Roam", Roam),
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

fn run(label: &str, tree: fn() -> Behaviour, churning: bool) {
    const TICKS: u32 = 2_000;
    for agents in [100, 500, 2_000] {
        let mut app = App::new();
        app.init_resource::<Time>()
            .add_plugins(TreePlugin::new(Update));
        if churning {
            app.add_systems(Update, churn.before(TreeSystems));
        }
        for _ in 0..agents {
            app.world_mut()
                .spawn((Mood(false), BehaviourTree::new(tree())));
        }
        // Started, so what's measured is the steady state.
        app.update();
        let start = Instant::now();
        for _ in 0..TICKS {
            app.update();
        }
        let tick = start.elapsed() / TICKS;
        let agent = tick / agents;
        println!("{label:<24} {agents:>5} agents: {tick:>10.2?} a tick, {agent:>8.2?} an agent");
    }
}

fn main() {
    run("system guards", systems, false);
    run("typed guards, still", typed, false);
    run("typed guards, 10% churn", typed, true);
}
