# Debugging

When the guard does something odd, you want to know three things. What is its
tree running? What does it perceive? What did its queries weigh? Traces answer
the first and the last as components. Gizmos draw all three in the world.

## What the tree is running

Insert a [`TreeTrace`] on the guard. Its tree then keeps it up to date:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component)] struct Guard;
# fn guard() -> Behavior { behavior_tree! { Wait(1.0) } }
fn spawn_guard(mut commands: Commands) {
    commands.spawn((Guard, BehaviorTree::new(guard()), TreeTrace::default()));
}
```

Written out with `Display`, a trace is the tree's outline. `BehaviorTree`
displays the same way, without a trace. A `>` marks what's
running. Each decorator is written above the branch it guards. Here is the
guard on patrol:

```text
> Forever
>   Select
      when (sees_intruder && cooldown(10s))
        Shout
      when sees_intruder
        Select
          when out_of_ammo
            Reload
          Sequence
            Chase
            Commit
              Shoot
            Wait(0.8s)
      when remembers_intruder
        Search
>     Patrol
```

Log it whenever it changes:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
fn log_trees(traces: Query<(Entity, &TreeTrace), Changed<TreeTrace>>) {
    for (agent, trace) in &traces {
        info!("{agent}:\n{trace}");
    }
}
```

A trace changes only when the tree does, so this logs only what's new.

### Reading a trace in code

- `nodes()` is every node, root first, each after its parent. A node's index
  is its id.
- `running()` is the running nodes with their ids, root down.
- `get(id)` is one node.

Each [`TraceNode`] has `label()`, `parent()`, `notes()` (what's written above
it), `is_running()` and `holds()` (whether it asked not to be interrupted).

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
/// What the guard is doing: the deepest node running.
fn doing(trace: &TreeTrace) -> Option<&str> {
    trace.running().last().map(|(_, node)| node.label())
}
```

A trace is a string per node. Keep it on the agents you're looking at, not on
every agent in a shipped game.

## What a query weighed

Insert a [`SurveyTrace`] on the guard. Each query run for it then records:

- `query`: the key's name, such as `"Cover"`.
- `items`: every item, as a `Weighed` with its `item`, its `score` (`None` if a
  filter dropped it) and whether it was `picked`.

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
/// How many points the last query dropped.
fn dropped(trace: &SurveyTrace) -> usize {
    trace.items.iter().filter(|weighed| weighed.score.is_none()).count()
}
```

If the cover query finds nothing, this tells you whether the filter dropped
every point, or the generator made none.

Both traces are reflected components. An inspector shows them as they change.

## Gizmos

With the `gizmos` feature, [`ArborGizmosPlugin`] draws in the world. It isn't
in `ArborPlugins`, as it needs a renderer. Add it where you have one, with
`DefaultPlugins`:

```rust,no_run
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            ArborPlugins.set(PerceptionPlugin::default().with_space(Space::Planar)),
            ArborGizmosPlugin,
        ))
        .run();
}
```

It draws each part that's on:

| Part | Draws |
|---|---|
| Perception | Sight cones out to `range`, fainter out to `lose_range`. Hearing ranges. A line to everything in view, colored by attitude. A cross where what's remembered was last sensed. |
| Queries | Each agent's last query, from its `SurveyTrace`. Kept items from red to green by score, dropped ones gray, the pick ringed. |
| Smart objects | Every slot: free, claimed with a line to its agent, or disabled. |
| Navigation | A ring round each `Destination`, as wide as how near counts as there, and each agent's `Route`. |

Each part has a gizmo group of its own: [`PerceptionGizmos`], [`EqsGizmos`],
[`SmartGizmos`] and [`NavGizmos`]. Each kind of shape has a color, or `None` to leave it out.
Change them through Bevy's `GizmoConfigStore`, as for any gizmo group. To draw
only the guard you're looking at, turn `draw_all` off and give it
[`ShowPerception`]:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component)] struct Guard;
fn watch_one(
    mut config: ResMut<GizmoConfigStore>,
    mut commands: Commands,
    guard: Single<Entity, With<Guard>>,
) {
    let (_, gizmos) = config.config_mut::<PerceptionGizmos>();
    gizmos.draw_all = false;
    gizmos.hearing = None;
    commands.entity(*guard).insert(ShowPerception);
}
```

## Pitfalls

- **No trace, no record.** `TreeTrace` and `SurveyTrace` are kept only where
  you insert them. The query gizmos draw only agents with a `SurveyTrace`.
- **The last query only.** A `SurveyTrace` holds the last query run for the
  agent. With two queries in a row, you see the second.
- **Headless apps.** `ArborGizmosPlugin` needs Bevy's gizmos, in
  `DefaultPlugins`. Leave them out of a server or a test.

[`TreeTrace`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/tree/struct.TreeTrace.html
[`TraceNode`]: https://docs.rs/bevy_arbor_bt/latest/bevy_arbor_bt/tree/struct.TraceNode.html
[`SurveyTrace`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/survey/struct.SurveyTrace.html
[`ArborGizmosPlugin`]: https://docs.rs/bevy_arbor_gizmos/latest/bevy_arbor_gizmos/struct.ArborGizmosPlugin.html
[`PerceptionGizmos`]: https://docs.rs/bevy_arbor_gizmos/latest/bevy_arbor_gizmos/perception/struct.PerceptionGizmos.html
[`ShowPerception`]: https://docs.rs/bevy_arbor_gizmos/latest/bevy_arbor_gizmos/perception/struct.ShowPerception.html
[`EqsGizmos`]: https://docs.rs/bevy_arbor_gizmos/latest/bevy_arbor_gizmos/eqs/struct.EqsGizmos.html
[`SmartGizmos`]: https://docs.rs/bevy_arbor_gizmos/latest/bevy_arbor_gizmos/smart/struct.SmartGizmos.html
[`NavGizmos`]: https://docs.rs/bevy_arbor_gizmos/latest/bevy_arbor_gizmos/nav/struct.NavGizmos.html
