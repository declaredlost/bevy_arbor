//! What ticking trees costs, with a tree shaped like a monster's: a `Select` of
//! four guards, the last branch's task running, so every guard is asked every
//! tick.
//!
//! `cargo run --release -p bevy_arbor --example load`

use std::time::Instant;

use bevy::prelude::*;
use bevy_arbor::prelude::*;

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

fn tree() -> Behaviour {
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

fn main() {
    const TICKS: u32 = 2_000;
    for agents in [100, 500, 2_000] {
        let mut app = App::new();
        app.init_resource::<Time>()
            .add_plugins(ArborPlugin::new(Update));
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
        let guard = tick / (agents * 4);
        println!("{agents:>5} agents: {tick:>10.2?} a tick, {guard:>8.2?} a guard");
    }
}
