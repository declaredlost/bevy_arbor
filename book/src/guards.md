# Guards

A guard is a condition on an agent, and comes in two kinds. Its signature
says which.

## Typed guards

A typed guard is a function of the agent's components: it takes `&T` or
`Option<&T>` of up to four, and returns `bool`.

```rust
# use bevy::prelude::*;
# #[derive(Component)] struct Health(f32);
# #[derive(Component)] struct Armour;
fn badly_hurt(health: &Health, armour: Option<&Armour>) -> bool {
    health.0 < 20.0 && armour.is_none()
}
```

It's asked again only once one of them changes, comes or goes on the agent,
so a tree under typed guards sleeps until then. One taking a `&T` the agent
hasn't got doesn't pass.

A typed guard reads only the agent. For anything else, keep it on the agent
with a system of your own, run before `ArborSystems`: a targeting system
writing `Target`, say. Write with `set_if_neq`, so only a real change wakes
trees:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
#[derive(Component, PartialEq)]
struct InReach(bool);

fn reach(mut agents: Query<(&Perceived<Sight>, &GlobalTransform, &mut InReach)>, places: Query<&GlobalTransform>) {
    for (seen, at, mut in_reach) in &mut agents {
        let near = seen.active().any(|percept| {
            places
                .get(percept.entity())
                .is_ok_and(|there| there.translation().distance(at.translation()) < 2.0)
        });
        in_reach.set_if_neq(InReach(near));
    }
}
```

## System guards

A system guard is a read-only system taking the agent, for what can't be kept
on it. It can read anything, so it's asked every tick, and its tree doesn't
sleep while it could matter.

```rust
# use bevy::prelude::*;
# #[derive(Component)] struct Target(Option<Entity>);
fn within(In((agent, reach)): In<(Entity, f32)>, targets: Query<(&Target, &GlobalTransform)>, places: Query<&GlobalTransform>) -> bool {
    let Ok((target, at)) = targets.get(agent) else {
        return false;
    };
    target
        .0
        .and_then(|target| places.get(target).ok())
        .is_some_and(|there| there.translation().distance(at.translation()) <= reach)
}
```

Read-only as a Bevy run condition is: `when` won't take a `ResMut` or a `&mut`
query, though it can queue `Commands`. `act` is for what changes things.
Either can return a `Result<bool>`, to use `?`: an error doesn't pass, and
goes to Bevy's error handler.

Systems are kept by type, so they can't capture anything: what varies goes in
as a parameter, `when(within, 5.0)`. Every node, tree and agent using a
system shares the one, `Local`s and change detection too, so keep what's per
agent on the agent.

## Combining

Guards of both kinds combine with `&&`, `||` and `!`, read as written:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component, Clone)] struct Chase;
# #[derive(Component, Clone)] struct Bite;
# #[derive(Component)] struct Calm;
# fn sees_enemy(_: &Perceived<Sight>) -> bool { true }
# fn calm(_: &Calm) -> bool { true }
# fn within(In((_, _)): In<(Entity, f32)>) -> bool { true }
# fn tree() -> Behaviour {
behaviour_tree! {
    Select => {
        when(within, 1.5) => task("Bite", Bite),
        when(sees_enemy && !calm) => task("Chase", Chase),
    }
}
# }
```

A tree sleeps on its typed guards only: one system guard that could matter
keeps it awake.
