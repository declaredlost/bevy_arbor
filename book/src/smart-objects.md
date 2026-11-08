# Smart objects

Some behavior belongs to things, not agents. The guard's wall has a turret.
The turret knows how it's fired. The guard only needs to know it wants a free
turret. A smart object has *slots*: places an agent can claim, so no one else
takes them. Each slot can bring the behavior for using it. Turn smart objects
on with the `smart` feature.

## A turret with a gunner's slot

A slot is an entity with [`SlotOf`] pointing at its object. What it offers is
a component of yours on it. A `Plugged<Using>` on it is the behavior it
brings:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
#[derive(Component)]
struct WallTurret;

/// What the turret's slot offers.
#[derive(Component)]
struct Gunner;

/// A task: fire the turret.
#[derive(Component, Clone)]
struct FireTurret;

fn fire_turret() -> Behavior {
    behavior_tree! { task(FireTurret) }
}

fn spawn_turret(mut commands: Commands) {
    commands
        .spawn((WallTurret, Transform::from_xyz(6.0, 2.0, 0.0)))
        .with_related::<SlotOf>((
            Gunner,
            Plugged::new(Using, fire_turret),
            Transform::from_xyz(5.0, 0.0, 0.0),
        ));
}
```

The slot's transform is where the gunner stands. `SlotOf` isn't `ChildOf`, so
it's in world space.

## Finding a free slot

[`FreeSlots<A>`][`FreeSlots`] is a [query](queries.md) generator. It makes the
slots offering `A` near an anchor that the agent could claim. Score them like
any items:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component)] struct Gunner;
/// The nearest free turret.
struct Turret;

fn setup(app: &mut App) {
    app.add_env_query(
        Turret,
        EnvQuery::new(FreeSlots::<Gunner>::within(Agent, 20.0))
            .score(Distance::to(Agent), Score::lower()),
    );
}
```

## Claiming and using it

[`claim::<Turret>()`][`claim`] is a [decorator](decorators.md). It claims the
slot the `Turret` query found, for as long as its branch runs. The slot's
`Plugged<Using>` is plugged into the guard, and `Socket(Using)` runs it:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# struct Turret;
# #[derive(Component, Clone)] struct GoToSlot;
# fn sees_intruder(seen: &Perceived<Sight>) -> bool { seen.active().any(Percept::is_hostile) }
fn man_turret() -> Behavior {
    behavior_tree! {
        Select => {
            when(sees_intruder && found::<Turret> && claim::<Turret>()) => Sequence => {
                task(GoToSlot),
                Socket(Using),
            },
            when(sees_intruder) => task(Survey::of(Turret)),
        }
    }
}
```

The guard sees an intruder and looks for a turret. Once it has found one, the
first branch passes. It claims the slot, walks there and fires. `GoToSlot` is
your task. It finds the slot in the guard's [`Claims`]:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component, Clone)] struct GoToSlot;
fn go_to_slot(
    mut tasks: Tasks<&GoToSlot, (&Claims, &mut Transform)>,
    slots: Query<&GlobalTransform, With<SlotOf>>,
) {
    tasks.for_each(|_, mut task, (claims, mut at)| {
        let Some(slot) = claims.iter().find_map(|slot| slots.get(slot).ok()) else {
            task.fail();
            return;
        };
        let spot = slot.translation();
        let step = (spot - at.translation).clamp_length_max(0.1);
        at.translation += step;
        if at.translation.distance(spot) < 0.1 {
            task.succeed();
        }
    });
}
```

With the `nav` feature, `task(MoveTo::found::<Turret>())` does the same, and
your movement takes the guard there. See [Navigation](navigation.md).

When the branch ends, however it ends, the claim is given up. The plugged
behavior is taken out of the guard too. If the guard had a `Plugged<Using>`
of its own, it's set aside while the slot's runs, and put back after.

## The details

- **One claim, one agent.** `claim` passes while the slot is free or already
  the guard's. If two guards go for one turret, the first to claim it wins.
  The other's `claim` stops passing, and its `Select` moves on.
- **Lost slots.** A slot taken by someone else, given [`SlotDisabled`], or
  despawned cancels the branch.
- **Claims are a relationship.** [`ClaimedBy`] is on the slot, `Claims` on the
  agent. Observe `On<Add<ClaimedBy>>` and `On<Remove<ClaimedBy>>` to hear
  claims and releases. Despawning the agent gives its slots up.
- **Objects own their slots.** The object's [`Slots`] lists them. Despawning
  the object despawns them.
- **Disabled slots.** A slot with `SlotDisabled` can't be found or claimed: a
  broken turret.
- **[`SmartPlugin`].** It's in `ArborPlugins`. It registers the components up
  front, so slots can be found before any is claimed.

## Queries about the slot

[`ClaimedSlots`] is an anchor at the slots the agent holds. Use it to ask
about the turret, not the guard. The ammo crate nearest the turret:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
#[derive(Component)]
struct AmmoCrate;

/// The ammo crate nearest the guard's turret.
struct Resupply;

fn setup(app: &mut App) {
    app.add_env_query(
        Resupply,
        EnvQuery::new(Nearby::<AmmoCrate>::within(ClaimedSlots, 10.0))
            .score(Distance::to(ClaimedSlots), Score::lower()),
    );
}
```

`ClaimedSlots` is nowhere while the agent holds no slot.

## Pitfalls

- **Claiming without finding.** `claim::<Turret>()` claims what `Found<Turret>`
  holds. Put `found::<Turret>` before it, and run the query first.
- **Leaving the branch.** The claim lasts only while its branch runs. A task
  outside that branch can't count on the slot.

[`SlotOf`]: https://docs.rs/bevy_arbor_smart/latest/bevy_arbor_smart/slot/struct.SlotOf.html
[`Slots`]: https://docs.rs/bevy_arbor_smart/latest/bevy_arbor_smart/slot/struct.Slots.html
[`SlotDisabled`]: https://docs.rs/bevy_arbor_smart/latest/bevy_arbor_smart/slot/struct.SlotDisabled.html
[`FreeSlots`]: https://docs.rs/bevy_arbor_smart/latest/bevy_arbor_smart/query/struct.FreeSlots.html
[`ClaimedSlots`]: https://docs.rs/bevy_arbor_smart/latest/bevy_arbor_smart/query/struct.ClaimedSlots.html
[`claim`]: https://docs.rs/bevy_arbor_smart/latest/bevy_arbor_smart/claim/fn.claim.html
[`Claims`]: https://docs.rs/bevy_arbor_smart/latest/bevy_arbor_smart/claim/struct.Claims.html
[`ClaimedBy`]: https://docs.rs/bevy_arbor_smart/latest/bevy_arbor_smart/claim/struct.ClaimedBy.html
[`SmartPlugin`]: https://docs.rs/bevy_arbor_smart/latest/bevy_arbor_smart/struct.SmartPlugin.html
