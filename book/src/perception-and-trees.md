# Perception and trees

Perception writes `Perceived<S>` on the agent; a tree's typed guards read it.
The tree watches what its guards read, so it sleeps until the agent notices,
loses or forgets something, and wakes the tick it does.

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component, Clone)] struct Chase;
# #[derive(Component, Clone)] struct Search;
# #[derive(Component, Clone)] struct Wander;
fn sees_enemy(seen: &Perceived<Sight>) -> bool {
    seen.active().any(Percept::is_hostile)
}

fn remembers_enemy(seen: &Perceived<Sight>, told: Option<&Perceived<Shared>>) -> bool {
    seen.iter().any(Percept::is_hostile) || told.is_some_and(|told| told.iter().any(Percept::is_hostile))
}

# fn tree() -> Behaviour {
behaviour_tree! {
    Forever => {
        Select => {
            when(sees_enemy) => task("Chase", Chase),
            when(remembers_enemy) => task("Search", Search),
            task("Wander", Wander),
        }
    }
}
# }
```

The monster chases what it sees; once it's lost it, it searches where it was
last seen, or where an ally said; once it's forgotten, it wanders again. A
noise doesn't wake this tree: none of its guards reads `Perceived<Hearing>`.

## Order

[`ArborSystems`] runs perception, then trees, in each schedule they share, so
a tree acts on what was noticed the same tick. Both run in `FixedUpdate`
unless you say otherwise; given different schedules, a tree may act a tick
later. Task systems go after `ArborSystems`.

## Tasks find their target

A guard says whether, not what: a task finds its target on the agent, from
what it perceives:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
# #[derive(Component, Clone)] struct Search;
fn search(mut tasks: Tasks<&Search, (&Perceived<Sight>, &mut Transform)>) {
    tasks.for_each(|_, mut task, (seen, mut transform)| {
        let last_seen = seen
            .iter()
            .filter(|percept| percept.is_hostile() && !percept.is_active())
            .filter_map(Percept::trace)
            .max_by_key(|trace| trace.at);
        match last_seen {
            Some(trace) if transform.translation.distance(trace.position) > 0.5 => {
                transform.translation = transform.translation.lerp(trace.position, 0.1);
            }
            _ => task.succeed(),
        }
    });
}
```
