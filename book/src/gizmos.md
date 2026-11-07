# Gizmos

With the `gizmos` feature, [`PerceptionGizmosPlugin`] draws what agents
perceive: each sensor's sight cone out to `range` and, fainter, `lose_range`;
hearing ranges; a line to everything active, coloured by attitude; and a cross
where what's remembered was last sensed.

```rust,no_run
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
use bevy_arbor::perception::gizmos::PerceptionGizmosPlugin;

# fn main() {
App::new().add_plugins((MinimalPlugins, ArborPlugins, PerceptionGizmosPlugin));
# }
```

It isn't in `ArborPlugins`, as it needs a renderer: a headless server has
none. Add it where there is one, with Bevy's `DefaultPlugins`.

## What's drawn

As Bevy's own gizmos do, [`PerceptionGizmos`] gives each kind a colour, or
`None` to leave it out, and draws every agent or, without `draw_all`, only
those with `ShowPerception`:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
use bevy_arbor::perception::gizmos::{PerceptionGizmos, ShowPerception};

fn only_the_boss(mut config: ResMut<GizmoConfigStore>, mut commands: Commands, boss: Single<Entity, With<Boss>>) {
    let (_, gizmos) = config.config_mut::<PerceptionGizmos>();
    gizmos.draw_all = false;
    gizmos.hearing = None;
    commands.entity(*boss).insert(ShowPerception);
}
# #[derive(Component)] struct Boss;
```

A sensor is drawn when it or its agent is shown. It's a gizmo group, so
`GizmoConfigStore` also turns it off altogether.

[`PerceptionGizmosPlugin`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/gizmos/struct.PerceptionGizmosPlugin.html
[`PerceptionGizmos`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/gizmos/struct.PerceptionGizmos.html
