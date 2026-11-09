use std::f32::consts::{FRAC_PI_2, PI};
use std::time::Duration;

use bevy::ecs::system::{ReadOnlySystemParam, StaticSystemParam, SystemParamItem};
use bevy::prelude::*;

use super::{Sense, SensorOf, agent};
use crate::perceived::Candidates;
use crate::space::{Grid, Space};

/// On an agent or a sensor: what it can see. A sustained sense: what's in
/// view stays in view, until it's lost.
///
/// It looks every `interval`, at a moment of its own, so looks spread across
/// ticks. What's in `range`, within `half_angle` of ahead, it notices;
/// what it already sees it keeps seeing out to `lose_range`, so a thing at the
/// edge doesn't flicker in and out.
#[derive(Component, Reflect, Debug, Clone, Copy, PartialEq)]
#[reflect(Component, Debug, Default, PartialEq)]
pub struct Sight {
    /// How far it notices things.
    pub range: f32,
    /// How far it keeps seeing what it already sees: at least `range`.
    pub lose_range: f32,
    /// How far off ahead it sees, either side, in radians: π sees all round.
    pub half_angle: f32,
    /// Where it looks from, on the agent or sensor: its eyes.
    pub eye: Vec3,
    /// How long between looks: zero looks every tick.
    pub interval: Duration,
}

impl Sense for Sight {
    /// Ten seconds after it went out of view.
    const MEMORY: Duration = Duration::from_secs(10);
    const SUSTAINED: bool = true;
}

impl Default for Sight {
    /// Ten units, the front half, five looks a second.
    fn default() -> Self {
        Self::new(10.0)
    }
}

impl Sight {
    /// Seeing as far as `range`, losing sight there too, over the front half,
    /// from the agent's origin, five looks a second.
    #[must_use]
    pub fn new(range: f32) -> Self {
        Self {
            range,
            lose_range: range,
            half_angle: FRAC_PI_2,
            eye: Vec3::ZERO,
            interval: Duration::from_millis(200),
        }
    }

    /// Keeps seeing what it sees out to `lose_range`.
    #[must_use]
    pub fn with_lose_range(mut self, lose_range: f32) -> Self {
        self.lose_range = lose_range;
        self
    }

    /// Sees `half_angle` radians off ahead, either side.
    #[must_use]
    pub fn with_half_angle(mut self, half_angle: f32) -> Self {
        self.half_angle = half_angle;
        self
    }

    /// Looks from `eye`, on the agent.
    #[must_use]
    pub fn with_eye(mut self, eye: Vec3) -> Self {
        self.eye = eye;
        self
    }

    /// Looks every `interval`.
    #[must_use]
    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    /// The farthest it sees anything.
    pub(crate) fn reach(&self) -> f32 {
        self.range.max(self.lose_range)
    }
}

/// On anything that can be seen.
#[derive(Component, Reflect, Debug, Clone, Copy, Default)]
#[reflect(Component, Debug, Default)]
pub struct Perceivable;

/// On an agent: which way it looks, if not as its transform does.
///
/// In a side-scroller, where a sprite flipped by `Sprite::flip_x` doesn't turn
/// its transform, keep this up to date instead. Its sensors look the same way,
/// unless they have a `Facing` of their own.
#[derive(Component, Reflect, Debug, Clone, Copy, PartialEq)]
#[reflect(Component, Debug, PartialEq)]
pub struct Facing(pub Dir3);

/// Puts everything perceivable in the grid, on the ticks a sensor looks:
/// cells as large as a typical sight, the median, so one sensor seeing very
/// far doesn't make every look go through huge cells.
pub(crate) fn build_grid(
    time: Res<Time>,
    mut grid: ResMut<Grid>,
    space: Res<Space>,
    sights: Query<(&Sight, Option<&Candidates<Sight>>)>,
    things: Query<(Entity, &GlobalTransform), With<Perceivable>>,
) {
    let now = time.elapsed();
    // Not yet looked, or due: either looks this tick.
    let looks = sights.iter().any(|(_, candidates)| {
        candidates
            .and_then(|candidates| candidates.next_look)
            .is_none_or(|due| now >= due)
    });
    if !looks {
        return;
    }
    let mut reaches: Vec<f32> = sights.iter().map(|(sight, _)| sight.reach()).collect();
    let middle = reaches.len() / 2;
    let size = if reaches.is_empty() {
        0.0
    } else {
        *reaches.select_nth_unstable_by(middle, f32::total_cmp).1
    };
    let space = *space;
    grid.rebuild(
        size,
        space,
        things
            .iter()
            .map(|(entity, transform)| (entity, space.flatten(transform.translation()))),
    );
}

/// How sight finds what's near a sensor: perception's own grid unless
/// another is set with
/// [`set_sight_index`](crate::sense::SightIndexApp::set_sight_index), as the
/// avian crates set avian's colliders.
pub trait SightIndex: Send + Sync + 'static {
    /// What it reads, as a system would.
    type Param: ReadOnlySystemParam;

    /// Calls `found` with each [`Perceivable`] thing within `radius` of
    /// `point`, and where it is in perception's [`Space`]. More is fine:
    /// sight measures each against its range and cone.
    fn near(
        param: &SystemParamItem<Self::Param>,
        point: Vec3,
        radius: f32,
        found: &mut dyn FnMut(Entity, Vec3),
    );
}

/// Perception's own index: a grid of everything perceivable, rebuilt on the
/// ticks a sensor looks.
pub(crate) struct GridIndex;

impl SightIndex for GridIndex {
    type Param = Res<'static, Grid>;

    fn near(
        grid: &SystemParamItem<Self::Param>,
        point: Vec3,
        radius: f32,
        found: &mut dyn FnMut(Entity, Vec3),
    ) {
        for (entity, at) in grid.near(point, radius) {
            found(entity, at);
        }
    }
}

/// Each sensor due to look, looks: what's in range and in its cone, of what
/// the index `I` finds near it, becomes a candidate.
pub(crate) fn look<I: SightIndex>(
    time: Res<Time>,
    space: Res<Space>,
    index: StaticSystemParam<I::Param>,
    mut sensors: Query<(
        Entity,
        &Sight,
        &GlobalTransform,
        Option<&Facing>,
        Option<&SensorOf>,
        &mut Candidates<Sight>,
    )>,
    facings: Query<&Facing>,
) where
    for<'w, 's> SystemParamItem<'w, 's, I::Param>: Sync,
{
    let (now, space) = (time.elapsed(), *space);
    let index = &*index;
    sensors
        .par_iter_mut()
        .for_each(|(sensor, sight, transform, facing, of, mut candidates)| {
            let due = *candidates
                .next_look
                .get_or_insert_with(|| now.saturating_add(stagger(sensor, sight.interval)));
            if now < due {
                return;
            }
            let candidates = candidates.as_mut();
            candidates.next_look = Some(now.saturating_add(sight.interval));
            let eye = space.flatten(transform.transform_point(sight.eye));
            candidates.look_from(eye);
            // Its own facing, else its agent's: an eye on a flipped sprite.
            let facing = facing.or_else(|| of.and_then(|of| facings.get(of.agent()).ok()));
            let ahead = space.looking(transform, facing);
            let (agent, all_round) = (agent(sensor, of), sight.half_angle >= PI);
            let cos = sight.half_angle.cos();
            I::near(index, eye, sight.reach(), &mut |entity, point| {
                if entity == sensor || entity == agent {
                    return;
                }
                let to = point - eye;
                // What it already sees, it keeps seeing further out.
                let range = if candidates.sensing.contains(&entity) {
                    sight.reach()
                } else {
                    sight.range
                };
                if to.length_squared() > range * range {
                    return;
                }
                // Looking no way it can tell, its cone holds nothing.
                let in_cone = ahead != Vec3::ZERO && to.normalize().dot(ahead) >= cos;
                if all_round || to == Vec3::ZERO || in_cone {
                    candidates.sense(entity, point);
                }
            });
        });
}

/// When `sensor` first looks, within `interval` of starting: looks spread
/// across ticks, the same way each run.
fn stagger(sensor: Entity, interval: Duration) -> Duration {
    let hash = sensor.to_bits().wrapping_mul(0x9e37_79b9_7f4a_7c15) >> 48;
    let fraction = f32::from(u16::try_from(hash).unwrap_or(0)) / 65_536.0;
    interval.mul_f32(fraction)
}

#[cfg(test)]
mod tests {
    use std::f32::consts::{FRAC_PI_4, PI};
    use std::time::Duration;

    use bevy::prelude::*;
    use bevy_harness::TestApp;

    use crate::prelude::*;
    use crate::testing::*;

    #[test]
    fn it_sees_what_is_ahead_in_range_and_not_behind_or_beyond() {
        for space in SPACES {
            let mut app = app(space);
            let agent = looker(&mut app, Sight::new(10.0));
            let near = thing(&mut app, ahead(space, 5.0));
            let far = thing(&mut app, ahead(space, 15.0));
            let back = thing(&mut app, behind(space, 5.0));
            app.tick();
            assert!(sees(&app, agent, near), "{space:?}: ahead, in range");
            assert!(!sees(&app, agent, far), "{space:?}: beyond range");
            assert!(!sees(&app, agent, back), "{space:?}: behind");
            assert!(
                perceived::<Sight>(&app, agent).get(agent).is_none(),
                "{space:?}: not itself"
            );
        }
    }

    #[test]
    fn it_sees_only_within_its_cone() {
        for space in SPACES {
            let mut app = app(space);
            let narrow = looker(&mut app, Sight::new(10.0).with_half_angle(FRAC_PI_4));
            let round = looker(&mut app, Sight::new(10.0).with_half_angle(PI));
            let inside = thing(&mut app, off_ahead(space, 5.0, 0.6));
            let outside = thing(&mut app, off_ahead(space, 5.0, 1.0));
            let back = thing(&mut app, behind(space, 5.0));
            app.tick();
            assert!(sees(&app, narrow, inside), "{space:?}: within 45°");
            assert!(!sees(&app, narrow, outside), "{space:?}: past 45°");
            assert!(sees(&app, round, back), "{space:?}: all round");
        }
    }

    #[test]
    fn it_keeps_seeing_out_to_lose_range_then_remembers_where_it_lost_it() {
        for space in SPACES {
            let mut app = app(space);
            let agent = looker(&mut app, Sight::new(10.0).with_lose_range(15.0));
            let target = thing(&mut app, ahead(space, 12.0));
            app.tick();
            assert!(
                !sees(&app, agent, target),
                "{space:?}: not noticed past range"
            );

            app.entity_mut(target)
                .insert(GlobalTransform::from_translation(ahead(space, 8.0)));
            app.tick();
            assert!(sees(&app, agent, target), "{space:?}: noticed");

            app.entity_mut(target)
                .insert(GlobalTransform::from_translation(ahead(space, 12.0)));
            app.tick();
            assert!(
                sees(&app, agent, target),
                "{space:?}: kept, inside lose range"
            );

            app.entity_mut(target)
                .insert(GlobalTransform::from_translation(ahead(space, 20.0)));
            app.tick();
            let percept = perceived::<Sight>(&app, agent).get(target).unwrap();
            assert!(!percept.is_active(), "{space:?}: lost");
            let lost_at = Space::flatten(space, ahead(space, 20.0));
            let trace = traces::<Sight>(&app, agent).get(target).unwrap();
            assert_eq!(trace.position, lost_at, "{space:?}");
        }
    }

    #[test]
    fn facing_turns_where_it_looks() {
        for space in SPACES {
            let mut app = app(space);
            let backwards = Dir3::new(behind(space, 1.0).with_z(match space {
                Space::Planar => 0.0,
                Space::Volume => 1.0,
            }))
            .unwrap();
            let agent = app.spawn((
                Sight::new(10.0).with_interval(Duration::ZERO),
                Facing(backwards),
                GlobalTransform::IDENTITY,
            ));
            let front = thing(&mut app, ahead(space, 5.0));
            let back = thing(&mut app, behind(space, 5.0));
            app.tick();
            assert!(sees(&app, agent, back), "{space:?}: faces back");
            assert!(!sees(&app, agent, front), "{space:?}");
        }
    }

    #[test]
    fn a_turned_transform_turns_where_it_looks() {
        for space in SPACES {
            let mut app = app(space);
            // In 2D a sprite flipped by its scale; in 3D a half turn.
            let turned = match space {
                Space::Planar => GlobalTransform::from_scale(Vec3::new(-1.0, 1.0, 1.0)),
                Space::Volume => GlobalTransform::from_rotation(Quat::from_rotation_y(PI)),
            };
            let agent = app.spawn((Sight::new(10.0).with_interval(Duration::ZERO), turned));
            let back = thing(&mut app, behind(space, 5.0));
            app.tick();
            assert!(sees(&app, agent, back), "{space:?}");
        }
    }

    #[test]
    fn in_2d_z_is_only_a_layer() {
        let mut app = app(Space::Planar);
        let agent = looker(&mut app, Sight::new(10.0));
        let layered = thing(&mut app, Vec3::new(5.0, 0.0, 500.0));
        app.tick();
        assert!(sees(&app, agent, layered));
    }

    #[test]
    fn an_agent_looks_once_an_interval_at_a_moment_of_its_own() {
        for space in SPACES {
            let mut app = app(space);
            let agent = app.spawn((
                Sight::new(10.0).with_interval(Duration::from_millis(500)),
                GlobalTransform::IDENTITY,
            ));
            let target = thing(&mut app, ahead(space, 5.0));
            app.secs(0.6);
            assert!(
                sees(&app, agent, target),
                "{space:?}: looked within the interval"
            );
            app.entity_mut(target)
                .insert(GlobalTransform::from_translation(ahead(space, 50.0)));
            app.tick();
            assert!(sees(&app, agent, target), "{space:?}: not looked again yet");
            app.secs(0.6);
            assert!(!sees(&app, agent, target), "{space:?}: looked again, lost");
        }
    }

    #[test]
    fn something_despawned_in_view_is_lost_where_it_was_last_seen() {
        for space in SPACES {
            let mut app = app(space);
            let agent = looker(&mut app, Sight::new(10.0));
            let target = thing(&mut app, ahead(space, 5.0));
            app.tick();
            app.entity_mut(target).despawn();
            app.tick();
            let percept = perceived::<Sight>(&app, agent).get(target).unwrap();
            assert!(!percept.is_active(), "{space:?}");
            assert_eq!(
                traces::<Sight>(&app, agent).get(target).unwrap().position,
                Space::flatten(space, ahead(space, 5.0)),
                "{space:?}"
            );
        }
    }

    #[test]
    fn looking_no_way_it_can_tell_its_cone_holds_nothing() {
        let mut app = app(Space::Planar);
        // Out of the plane: no way within it.
        let agent = app.spawn((
            Sight::new(10.0).with_interval(Duration::ZERO),
            Facing(Dir3::Z),
            GlobalTransform::IDENTITY,
        ));
        let behind = thing(&mut app, behind(Space::Planar, 5.0));
        let front = thing(&mut app, ahead(Space::Planar, 5.0));
        app.tick();
        assert!(!sees(&app, agent, behind), "not all round");
        assert!(!sees(&app, agent, front));
    }

    #[test]
    fn a_sensor_looks_as_its_agent_faces() {
        for space in SPACES {
            let mut app = app(space);
            let backwards = Dir3::new(behind(space, 1.0).with_z(match space {
                Space::Planar => 0.0,
                Space::Volume => 1.0,
            }))
            .unwrap();
            let agent = app.spawn((Facing(backwards), GlobalTransform::IDENTITY));
            app.spawn((
                Sight::new(10.0).with_interval(Duration::ZERO),
                SensorOf(agent),
                GlobalTransform::IDENTITY,
            ));
            let back = thing(&mut app, behind(space, 5.0));
            app.tick();
            assert!(sees(&app, agent, back), "{space:?}: turned with its agent");
        }
    }

    #[test]
    fn a_huge_interval_does_not_overflow() {
        for space in SPACES {
            let mut app = app(space);
            let agent = looker(&mut app, Sight::new(10.0));
            thing(&mut app, ahead(space, 5.0));
            app.tick();
            // Looked: its next look is now plus the interval.
            app.entity_mut(agent)
                .insert(Sight::new(10.0).with_interval(Duration::MAX));
            app.ticks(3);
            assert!(
                app.world().get::<Perceived<Sight>>(agent).is_some(),
                "{space:?}"
            );
        }
    }

    #[test]
    fn one_sight_seeing_far_does_not_size_the_grid() {
        for space in SPACES {
            let mut app = app(space);
            for _ in 0..4 {
                looker(&mut app, Sight::new(5.0));
            }
            looker(&mut app, Sight::new(500.0));
            app.tick();
            let size = app.world().resource::<crate::space::Grid>().size();
            assert!(size < 10.0, "{space:?}: a typical sight's, not 500: {size}");
        }
    }

    #[test]
    fn no_one_looking_the_grid_is_left_as_it_was() {
        for space in SPACES {
            let mut app = app(space);
            app.spawn((
                Sight::new(10.0).with_interval(Duration::from_secs(1_000)),
                GlobalTransform::IDENTITY,
            ));
            let target = thing(&mut app, ahead(space, 5.0));
            app.ticks(2);
            let listed_at = |app: &TestApp, at: Vec3| {
                app.world()
                    .resource::<crate::space::Grid>()
                    .near(at, 1.0)
                    .any(|(entity, _)| entity == target)
            };
            assert!(listed_at(&app, ahead(space, 5.0)), "{space:?}: built once");
            // Moved where no one's looking: the grid isn't rebuilt to know.
            app.entity_mut(target)
                .insert(GlobalTransform::from_translation(ahead(space, 500.0)));
            app.tick();
            assert!(listed_at(&app, ahead(space, 5.0)), "{space:?}: as it was");
        }
    }
}
