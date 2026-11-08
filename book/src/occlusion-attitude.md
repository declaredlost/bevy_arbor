# Occlusion and attitude

So far the guard sees through walls, and it takes anyone on another team for
an enemy. This chapter fixes both. Walls block its sight, and you decide who
it's hostile to.

Perception doesn't depend on a physics engine. What blocks sight and who's
hostile are systems that join its stages. With avian, line of sight is
ready-made.

## Walls with avian

Enable the `avian2d` feature of `bevy_arbor`, and depend on avian as you
would anyway:

```toml
[dependencies]
bevy_arbor = { version = "0.1", features = ["avian2d"] }
avian2d = "0.8"
```

`ArborPlugins` then adds [`LineOfSightPlugin`]. It casts a ray from where each
sensor looked to each thing it might see. If the ray hits something else
first, the thing is blocked. The walls only need colliders:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
use avian2d::prelude::*;

# fn spawn(mut commands: Commands) {
commands.spawn((
    RigidBody::Static,
    Collider::rectangle(1.0, 6.0),
    Transform::from_xyz(5.0, 0.0, 0.0),
));
# }
```

Now the intruder behind the wall at x = 5 is out of the guard's sight.

- The guard's own colliders, and its sensor's, never block.
- A collider on the intruder, or on a child of it, is the intruder. It
  doesn't block.
- Trigger volumes (avian's `Sensor`) never block.
- The plugin needs avian's `PhysicsPlugins`. Without them it warns and blocks
  nothing.

### What blocks

By default, every collision layer blocks sight. Set [`Occluders`] to choose
the layers:

```rust,no_run
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
use avian2d::prelude::*;
use bevy_arbor::avian2d::sight::Occluders;

#[derive(PhysicsLayer, Default)]
enum Layer {
    #[default]
    Default,
    Walls,
    Glass,
    Guards,
}

fn main() {
    App::new()
        .add_plugins((
            MinimalPlugins,
            PhysicsPlugins::default(),
            ArborPlugins.set(PerceptionPlugin::default().with_space(Space::Planar)),
        ))
        // Walls and glass block sight. Other guards don't.
        .insert_resource(Occluders(LayerMask::from([Layer::Walls, Layer::Glass])))
        .run();
}
```

A sensor can override that with its own [`SensorOccluders`]. The lookout
camera sees through glass:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# use avian2d::prelude::*;
# #[derive(PhysicsLayer, Default)] enum Layer { #[default] Default, Walls, Glass }
use bevy_arbor::avian2d::sight::SensorOccluders;

# fn spawn(mut commands: Commands, guard: Entity) {
commands.spawn((
    Sight::new(30.0),
    SensorOf(guard),
    SensorOccluders(LayerMask::from([Layer::Walls])),
    Transform::from_xyz(-15.0, 4.0, 0.0),
));
# }
```

### Sight points

A ray goes to the middle of what it looks at. So an intruder whose head shows
over a low crate is still hidden. [`SightPoints`] lists the points it can be
seen by, relative to it. It's seen if a ray to any of them gets through:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component)] struct Intruder;
use bevy_arbor::avian2d::sight::SightPoints;

# fn spawn(mut commands: Commands) {
commands.spawn((
    Intruder,
    Perceivable,
    Team(2),
    SightPoints(vec![Vec3::new(0.0, 0.8, 0.0), Vec3::ZERO, Vec3::new(0.0, -0.8, 0.0)]),
    Transform::from_xyz(8.0, 0.0, 0.0),
));
# }
```

Head, middle and feet. A head over the crate is enough.

### Touch from contacts

The avian integration also adds [`ContactTouchPlugin`]. It turns collisions
into `Touched` messages for agents with `Touch`. A bump or a hit is felt
without code of yours. Limit it to some layers, so the guard feels a sword
and not the floor:

```rust,no_run
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# use avian2d::prelude::*;
# #[derive(PhysicsLayer, Default)] enum Layer { #[default] Default, Hitbox }
use bevy_arbor::avian2d::touch::ContactTouchPlugin;

# fn main() {
App::new().add_plugins((
    MinimalPlugins,
    PhysicsPlugins::default(),
    ArborPlugins.set(ContactTouchPlugin { layers: LayerMask::from([Layer::Hitbox]) }),
));
# }
```

An agent with `Touch` gets avian's `CollisionEventsEnabled`, so its own
collider reports contacts. A collider on a child of the agent needs one too.

## Occlusion of your own

Without avian, write the occluder yourself. Perception runs in four stages,
[`PerceptionSystems`]:

| Stage | What runs there |
|---|---|
| `Gather` | Each sensor collects what it might sense this tick |
| `Occlude` | Blocks what's out of sight |
| `Classify` | Gives each candidate an attitude |
| `Commit` | Writes `Perceived<S>` and forgets what faded |

Systems in `Occlude` and `Classify` work on each sensor's [`Candidates<S>`].
Here's one blocking sight with a raycast of your own:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component)] struct Wall;
# fn ray_hits_wall(_: Vec3, _: Vec3, _: &Query<&GlobalTransform, With<Wall>>) -> bool { false }
fn walls(mut sensors: Query<&mut Candidates<Sight>>, walls: Query<&GlobalTransform, With<Wall>>) {
    for mut candidates in &mut sensors {
        let Some(eye) = candidates.eye() else {
            continue;
        };
        for candidate in candidates.iter_mut() {
            if ray_hits_wall(eye, candidate.position(), &walls) {
                candidate.block();
            }
        }
    }
}

# fn add(app: &mut App) {
app.add_systems(FixedUpdate, walls.in_set(PerceptionSystems::Occlude));
# }
```

`eye()` is where the sensor looked from this tick. It's `None` if the sensor
didn't look, and then there's nothing to block. Several occluders combine:
one blocking is enough.

## Attitude

Every percept has an [`Attitude`]: hostile, neutral or friendly. The guard's
conditions use it through `is_hostile`.

By default it comes from [`Team`]:

| The guard | The other | Attitude |
|---|---|---|
| `Team(1)` | `Team(1)` | Friendly |
| `Team(1)` | `Team(2)` | Hostile |
| Either has no team | | Neutral |

That's why the guard has `Team(1)` and the intruder `Team(2)`.

### Attitude of your own

A system in `PerceptionSystems::Classify` can set the attitude instead. What
it sets stands over the team. Here, a disguised intruder passes as a friend:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
#[derive(Component)]
struct Disguised;

fn disguises(mut sensors: Query<&mut Candidates<Sight>>, disguised: Query<(), With<Disguised>>) {
    for mut candidates in &mut sensors {
        for candidate in candidates.iter_mut() {
            if disguised.contains(candidate.entity()) {
                candidate.set_attitude(Attitude::Friendly);
            }
        }
    }
}

# fn add(app: &mut App) {
app.add_systems(FixedUpdate, disguises.in_set(PerceptionSystems::Classify));
# }
```

When the attitude changes, `Perceived<S>` changes too. Drop the disguise, and
the guard's tree wakes and sees an enemy.

## Pitfalls

- **Perception's schedule.** It runs in `FixedUpdate` unless you said
  otherwise. Add your stage systems to the same schedule.
- **A step behind.** Avian runs in `FixedPostUpdate`, so a ray sees colliders
  where the last physics step left them. An AI won't notice.
- **Two classifiers on one candidate.** The last to run wins, and their order
  isn't fixed. Make each handle its own case, or order them with `.after`.

[`LineOfSightPlugin`]: https://docs.rs/bevy_arbor_avian2d/latest/bevy_arbor_avian2d/sight/struct.LineOfSightPlugin.html
[`Occluders`]: https://docs.rs/bevy_arbor_avian2d/latest/bevy_arbor_avian2d/sight/struct.Occluders.html
[`SensorOccluders`]: https://docs.rs/bevy_arbor_avian2d/latest/bevy_arbor_avian2d/sight/struct.SensorOccluders.html
[`SightPoints`]: https://docs.rs/bevy_arbor_avian2d/latest/bevy_arbor_avian2d/sight/struct.SightPoints.html
[`ContactTouchPlugin`]: https://docs.rs/bevy_arbor_avian2d/latest/bevy_arbor_avian2d/touch/struct.ContactTouchPlugin.html
[`PerceptionSystems`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/enum.PerceptionSystems.html
[`Candidates<S>`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/perceived/struct.Candidates.html
[`Attitude`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/perceived/enum.Attitude.html
[`Team`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/perceived/struct.Team.html
