# Queries

A tree decides *what* the guard does. An environment query decides *where*,
or *at what*. When the guard reloads, it wants a spot the intruder can't see.
When two intruders are in view, it wants the better one to shoot at. Both are
queries. Turn them on with the `eqs` feature.

## A query for cover

A query makes items, drops the ones that fail its filters, scores the rest,
and picks one. Here is the guard's cover:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
use avian2d::prelude::Collider;
use bevy_arbor::avian2d::eqs::{Clearance, LineOfSight, OccluderDistance};

/// Somewhere out of the intruder's sight.
struct Cover;

fn cover() -> EnvQuery {
    // Where the intruder is, or was last seen.
    let intruders = || Hostiles::<Sight>::remembered();
    // Points level with the guard, either side of it.
    EnvQuery::new(Row::along(Agent).extent(8.0).spacing(0.5))
        // Keep the points the guard fits in.
        .filter(Clearance::of(Collider::circle(0.4)), Filter::is(true))
        // And that the intruder can't see.
        .filter(LineOfSight::from(intruders()), Filter::is(false))
        // Prefer up against the wall, near the guard, far from the intruder.
        .score(OccluderDistance::new(), Score::lower())
        .score(Distance::to(Agent), Score::lower().weight(0.5))
        .score(Distance::to(intruders()), Score::higher().weight(0.5))
}
```

The guard's world is seen from the side, so its cover is a `Row` along the
ground at its height, not a `Donut`, whose points would be in the air.

`remembered()` is where the intruder is while it's in view, and where it was
last seen after. So the guard can still take cover once it's lost sight of
the intruder.

`Cover` is the query's *key*. It's any type of yours. The next chapter adds
the query to the app under it and runs it.

## A query for a target

Items can be things, not just places. This query weighs the intruders the
guard sees:

```rust
# use bevy::prelude::*;
# use bevy_arbor::prelude::*;
/// The intruder to shoot at.
struct Target;

fn target() -> EnvQuery {
    EnvQuery::new(Percepts::<Sight>::all().active().hostile())
        // The nearest, and then the one most in front.
        .score(Distance::to(Agent), Score::lower())
        .score(Dot::of(Agent), Score::higher().weight(0.5))
}
```

## Generators

A generator makes the items.

| Generator | Items |
|---|---|
| [`Donut::around(a)`][`Donut`] | Points on rings round an anchor: `.radius(inner, outer)`, `.rings(n)`, `.points(n)` |
| [`Grid::around(a)`][`Grid`] | Points on a square grid: `.extent(e)`, `.spacing(s)`, at most 50 steps each way |
| [`Fan::from(a)`][`Fan`] | Points on rays ahead of an anchor: `.angle(a)`, `.range(near, far)`, `.rays(rays, steps)` |
| [`Row::along(a)`][`Row`] | Points on a line through an anchor, the way it looks: `.extent(e)`, `.spacing(s)`, at most 50 steps each way |
| [`Percepts::<S>::all()`][`Percepts`] | What the agent perceives by sense `S`: `.active()`, `.hostile()` |
| [`Nearby::<C>::within(a, r)`][`Nearby`] | Things with component `C` near an anchor |

Points lie in the plane things stand on. That's perception's [`Space`]: the
XY plane in 2D, and in 3D the XZ plane at the anchor's height. In a 2D side
view, that plane is the wall, not the floor: a `Row` keeps to the guard's
height, where a `Donut` would put points in the air. Add more
generators with `.generate(..)`: the items of all of them are weighed
together.

## Anchors

An anchor is where a generator or a test measures from.

| Anchor | Where |
|---|---|
| [`Agent`] | The agent running the query |
| [`At(point)`][`At`] | A fixed point |
| [`Thing(entity)`][`Thing`] | An entity |
| [`Hostiles::<S>::active()`][`Hostiles`] | Hostiles the agent perceives by `S` now |
| [`Hostiles::<S>::remembered()`][`Hostiles`] | Hostiles where they were last sensed |

An anchor can be at several places, such as two intruders. A test then
measures from the nearest. A generator makes items round each.

## Tests

A test gives each item a number.

| Test | Number |
|---|---|
| [`Distance::to(a)`][`Distance`] | How far the item is from the anchor |
| [`Dot::of(a)`][`Dot`] | 1 straight ahead of the anchor, -1 behind |
| [`Perceives::<S>::at_all()`][`Perceives`] | 1 if the agent perceives the item by `S`, else 0 |
| [`Random`] | A random number from 0 to 1 |
| `LineOfSight::from(a)` | 1 if a ray from the anchor reaches the item (avian, with `eqs`) |
| `Clearance::of(shape)` | 1 if the shape, centered on the item, overlaps no collider: room to stand (avian, with `eqs`) |
| `OccluderDistance::new()` | How far the item is from the nearest collider that blocks sight: `.on(layers)` for others (avian, with `eqs`) |

A test can have no answer for an item: `Perceives` for a place, say. The item
then fails any filter on that test, and scores nothing by it.

## Filters

A filter keeps the items whose number is in range.

- [`Filter::at_least(min)`][`Filter`] and `Filter::at_most(max)`.
- `Filter::range(min, max)`.
- `Filter::is(yes)`, for a test that answers 1 or 0.

Filters run before scores, cheapest first. So the cover query casts a ray only
for the points that are left.

## Scores

A score turns a test's numbers into 0 for the worst item and 1 for the best.

- [`Score::higher()`][`Score`] prefers high numbers. `Score::lower()` prefers
  low ones.
- `.squared()` makes the best stand out more.
- `.weight(w)` says how much this score counts. It's 1 unless set.

An item's score is the sum of its scores. A query with no scores scores every
item kept 1.

## Picks

`.pick(..)` says what the query gives.

- [`Pick::Best`][`Pick`]: the best item. This is the default.
- `Pick::RandomOfBest(0.25)`: one item at random among the best quarter, for
  variety.
- `Pick::All`: every item kept, best first.

## Pitfalls

- **An anchor that's nowhere.** `Hostiles::<Sight>::active()` is nowhere when
  the guard sees no one. Then a line-of-sight test from it has no answer for
  any point, and every point fails. Use `remembered()` to hide from where the
  intruder was last seen.
- **Points in walls.** Generators don't know about walls. A point inside one
  is out of sight, so it passes a line-of-sight filter. Filter on
  `Clearance` first.
- **Points in mid-air.** In a side view, points level with the guard can be
  over a drop, and a `Row` doesn't follow the floor up a slope or onto a
  ledge. Rule them out with a [test of your own](running-queries.md): a
  `Measure` that casts a ray down from each point.
- **Straight lines.** `Distance` is as the crow flies. A spot behind a wall
  may be near but a long walk away.
- **Too many items.** Every item runs every test. A donut of 3 rings of 16 is
  48 points; a fine grid can be thousands. Keep generators coarse, and put
  cheap filters first.
- **Normalized scores.** Scores compare items with each other. The best item
  scores 1 even if it's poor. Use a filter to rule items out.

[`Donut`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/generate/struct.Donut.html
[`Grid`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/generate/struct.Grid.html
[`Fan`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/generate/struct.Fan.html
[`Row`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/generate/struct.Row.html
[`Percepts`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/generate/struct.Percepts.html
[`Nearby`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/generate/struct.Nearby.html
[`Agent`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/anchor/struct.Agent.html
[`At`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/anchor/struct.At.html
[`Thing`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/anchor/struct.Thing.html
[`Hostiles`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/anchor/struct.Hostiles.html
[`Distance`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/test/struct.Distance.html
[`Dot`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/test/struct.Dot.html
[`Perceives`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/test/struct.Perceives.html
[`Random`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/test/struct.Random.html
[`Filter`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/query/struct.Filter.html
[`Score`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/query/struct.Score.html
[`Pick`]: https://docs.rs/bevy_arbor_eqs/latest/bevy_arbor_eqs/query/enum.Pick.html
[`Space`]: https://docs.rs/bevy_arbor_perception/latest/bevy_arbor_perception/enum.Space.html
