# Navigation

The guard found cover. Now it has to walk there. bevy_arbor finds no paths
and moves nothing: your game already has its own movement, and its own idea
of where a character can go. Navigation is the contract between the two. The
AI says where the agent should be, and your movement says how getting there
goes. Turn it on with the `nav` feature.

## The contract

Three components on the agent:

| Component | What it says | Who writes it |
|---|---|---|
| [`Destination`] | Where the agent is asked to go: a [`Goal`] (a point, or an entity to follow) and how near counts as there | A `MoveTo` task, or you |
| [`Travel`] | How it's going: `Moving`, `Arrived` or `Unreachable` | Your movement |
| [`Route`] | The way it's taking, as points, if your movement says | Your movement, for gizmos and debugging |

Your movement reads `Destination`. It finds a way with whatever pathfinding
suits the game, moves the agent its own way, and writes `Travel`.

## Moving from a tree

[`MoveTo`] is a task that asks for the agent to be somewhere:

- `MoveTo::point(post)`: a point.
- `MoveTo::entity(leader)`: an entity, wherever it goes.
- `MoveTo::found::<Cover>()`: where the query keyed by `Cover` found. It fails
  if the query found nothing.
- `.within(2.0)`: how near counts as there. Half a unit unless set.

The guard's fight, with cover it walks to:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# struct Cover;
# #[derive(Component)] struct Ammo(u32);
# #[derive(Component, Clone)] struct Reload;
# #[derive(Component, Clone)] struct Shoot;
# fn out_of_ammo(ammo: &Ammo) -> bool { ammo.0 == 0 }
fn fight() -> Behavior {
    behavior_tree! {
        Select => {
            when(out_of_ammo) => Sequence => {
                task(Survey::of(Cover)),
                task(MoveTo::found::<Cover>()),
                task(Reload),
            },
            task(Shoot),
        }
    }
}
```

As `MoveTo` starts, it sets the guard's `Destination` and clears its old
`Travel`. An `Arrived` left from the last move isn't this one's. It succeeds
once your movement writes `Travel::Arrived`, and fails on
`Travel::Unreachable`, or if the entity it's going to is despawned. However
it ends, the `Destination` it set is taken off, with `Travel` and `Route`. A
move cut short by an interruption stops the guard. A `Destination` set since,
by another move or by you, stays.

A `Graceful` `MoveTo` doesn't stop when interrupted. It carries on until it
arrives or can't get there: for a move you'd rather finish.

## Your movement

The guard lives in a side-scroller. This movement walks it along its floor,
toward its goal's x. A goal on another floor is out of reach: getting there
takes jumps, which need a jump graph (see below).

```rust,no_run
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
/// How fast the guard walks, in units a second.
const SPEED: f32 = 3.0;

/// Walks each agent with a destination along its floor, and says how it goes.
fn walk(
    mut agents: Query<(Entity, &Destination, &mut Transform, Option<&Travel>)>,
    places: Query<&GlobalTransform>,
    time: Res<Time>,
    mut commands: Commands,
) {
    for (agent, destination, mut at, travel) in &mut agents {
        let Some(goal) = destination.goal.position(&places) else {
            continue;
        };
        let away = goal.x - at.translation.x;
        let now = if (goal.y - at.translation.y).abs() > 1.0 {
            // On another floor: only a jump graph would know the way.
            Travel::Unreachable
        } else if away.abs() <= destination.within {
            Travel::Arrived
        } else {
            at.translation.x += away.signum() * (SPEED * time.delta_secs()).min(away.abs());
            Travel::Moving
        };
        // Written only when it changes.
        if travel != Some(&now) {
            let on_floor = Vec3::new(goal.x, at.translation.y, at.translation.z);
            commands.entity(agent).insert((now, Route(vec![on_floor])));
        }
    }
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            ArborPlugins.with_space(Space::Planar),
        ))
        .add_systems(FixedUpdate, walk.after(ArborSystems))
        .run();
}
```

It only sees agents with a `Destination`. So when a `MoveTo` ends and the
destination goes, the guard stops.

`MoveTo` tasks are kept in [`NavSystems`], in [`ArborSystems`] after trees
and queries. Your movement runs after `ArborSystems`. A `MoveTo` a tree
starts sets its destination that tick, and your movement starts moving the
same tick.

## Finding the way

Which pathfinding fits depends on how the game's characters move:

| Game | How characters move | Pathfinding |
|---|---|---|
| Top-down 2D | Anywhere on the floor | A navmesh: [vleue_navigator], built from avian2d colliders |
| 3D | Anywhere on walkable ground | A navmesh: [rerecast] builds it from avian3d colliders, and [landmass] finds paths, steers and avoids |
| Tiles | Tile to tile | A grid: [bevy_northstar], or A* from the [pathfinding] crate |
| Side-scroller | Along floors, then falling, dropping and jumping | A jump graph of your own, searched with the [pathfinding] crate |

The guard's world is the last kind. A navmesh is a floor plan: an agent on it
can go anywhere within its polygons, in any direction. In a side view, that
floor plan would take in the air between platforms, and the guard would walk
up into it. A walker in a side-scroller moves along a floor's surface. To go
anywhere else, it has to fall off an end, drop through a platform, or jump.
Whether a jump makes it depends on its speed, gravity, its height and the
ceilings in the way.

So a side-scroller's graph has places to stand as its points, and four kinds
of link between them: walk, fall, drop and jump. Each jump is kept only if a
jump at the walker's speed lands. Build the graph once per level and body
size, and search it with A*. Your movement then follows the path leg by leg,
jumping where a leg says to.

## Path costs in queries

The guard's cover query measures straight lines. A spot behind a wall may be
near, and still a long walk away. [`PathDistance`] measures the walk instead,
with your pathfinder. Set the pathfinder once, as a read-only system:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# struct Cover;
/// The walk to each item along the guard's floor, if it's on that floor.
fn walk_costs(batch: Batch, places: Query<&GlobalTransform>) -> Vec<Option<f32>> {
    let Ok(agent) = places.get(batch.agent) else {
        return vec![None; batch.items.len()];
    };
    let from = agent.translation();
    batch
        .items
        .iter()
        .map(|item| {
            let to = item.position();
            ((to.y - from.y).abs() <= 1.0).then(|| (to.x - from.x).abs())
        })
        .collect()
}

fn setup(app: &mut App) {
    app.set_path_costs(walk_costs).add_env_query(
        Cover,
        EnvQuery::new(Donut::around(Agent).radius(2.0, 8.0).rings(3).points(16))
            // Cheap first: no further than 10 as the crow flies.
            .filter(Distance::to(Agent), Filter::at_most(10.0))
            // Then only what's a walk of 12 or less.
            .filter(PathDistance::new(), Filter::at_most(12.0))
            .score(Distance::to(Agent), Score::lower()),
    );
}
```

The system gets a [`Batch`]: the agent, and every item the query still has.
It answers each, in order, with a cost or `None`. `None` means there's no way
there, and the item fails the filter. The whole batch comes at once, so one
search from the agent can answer every item.

The cheap `Distance` filter runs first, so the pathfinder sees fewer items.

## Gizmos

With the `gizmos` feature, [`ArborGizmosPlugin`] draws where agents are
going: a ring round each destination, as wide as how near counts as there,
and the route each agent is taking. Its gizmo group is [`NavGizmos`]. Set a
color to `None` to leave that out.

## Pitfalls

- **Nothing writes `Travel`.** `MoveTo` waits for your movement to say
  `Arrived` or `Unreachable`. Without it, the task never ends.
- **Moving without a destination.** When a `MoveTo` ends, however it ends,
  `Destination` is taken off. Movement that keeps going after that walks a
  cancelled guard on.
- **Two schedules.** `NavPlugin` runs in `FixedUpdate`, as trees and
  queries do. Moved to another schedule, it warns: `MoveTo` is no longer
  ordered after them.
- **One search per item.** Path costs come in one batch. Answer them with one
  search from the agent where you can, not one path an item.
- **No pathfinder set.** `PathDistance` without `set_path_costs` warns, and
  answers nothing for any item. Every item fails a filter on it.
- **Scoring by path cost.** An unreachable item has no answer, and scores
  nothing. Filter on `PathDistance` to drop it. A second `PathDistance` in a
  score searches again.

[`Destination`]: https://docs.rs/bevy_arbor_nav/latest/bevy_arbor_nav/travel/struct.Destination.html
[`Goal`]: https://docs.rs/bevy_arbor_nav/latest/bevy_arbor_nav/travel/enum.Goal.html
[`Travel`]: https://docs.rs/bevy_arbor_nav/latest/bevy_arbor_nav/travel/enum.Travel.html
[`Route`]: https://docs.rs/bevy_arbor_nav/latest/bevy_arbor_nav/travel/struct.Route.html
[`MoveTo`]: https://docs.rs/bevy_arbor_nav/latest/bevy_arbor_nav/move_to/struct.MoveTo.html
[`NavSystems`]: https://docs.rs/bevy_arbor_nav/latest/bevy_arbor_nav/struct.NavSystems.html
[`ArborSystems`]: https://docs.rs/bevy_arbor/latest/bevy_arbor/struct.ArborSystems.html
[`PathDistance`]: https://docs.rs/bevy_arbor_nav/latest/bevy_arbor_nav/path/struct.PathDistance.html
[`Batch`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/test/struct.Batch.html
[`ArborGizmosPlugin`]: https://docs.rs/bevy_arbor_gizmos/latest/bevy_arbor_gizmos/struct.ArborGizmosPlugin.html
[`NavGizmos`]: https://docs.rs/bevy_arbor_gizmos/latest/bevy_arbor_gizmos/nav/struct.NavGizmos.html
[vleue_navigator]: https://github.com/vleue/vleue_navigator
[rerecast]: https://github.com/janhohenheim/rerecast
[landmass]: https://github.com/andriyDev/landmass
[bevy_northstar]: https://github.com/jtothethree/bevy_northstar
[pathfinding]: https://crates.io/crates/pathfinding
