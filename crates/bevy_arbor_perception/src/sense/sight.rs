use std::f32::consts::{FRAC_PI_2, PI};
use std::time::Duration;

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
/// its transform, keep this up to date instead.
#[derive(Component, Reflect, Debug, Clone, Copy, PartialEq)]
#[reflect(Component, Debug, PartialEq)]
pub struct Facing(pub Dir3);

/// Puts everything that can be seen in the grid, cells as large as the longest
/// sight.
pub(crate) fn build_grid(
    mut grid: ResMut<Grid>,
    space: Res<Space>,
    sights: Query<&Sight>,
    things: Query<(Entity, &GlobalTransform), With<Perceivable>>,
) {
    let size = sights.iter().map(Sight::reach).fold(0.0, f32::max);
    let space = *space;
    grid.rebuild(
        size,
        space,
        things
            .iter()
            .map(|(entity, transform)| (entity, space.flatten(transform.translation()))),
    );
}

/// Each sensor due to look, looks: what's in range and in its cone becomes a
/// candidate.
pub(crate) fn look(
    time: Res<Time>,
    space: Res<Space>,
    grid: Res<Grid>,
    mut sensors: Query<(
        Entity,
        &Sight,
        &GlobalTransform,
        Option<&Facing>,
        Option<&SensorOf>,
        &mut Candidates<Sight>,
    )>,
) {
    let (now, space) = (time.elapsed(), *space);
    sensors
        .par_iter_mut()
        .for_each(|(sensor, sight, transform, facing, of, mut candidates)| {
            let due = *candidates
                .next_look
                .get_or_insert_with(|| now + stagger(sensor, sight.interval));
            if now < due {
                return;
            }
            let candidates = candidates.as_mut();
            candidates.next_look = Some(now + sight.interval);
            let eye = space.flatten(transform.transform_point(sight.eye));
            candidates.look_from(eye);
            let ahead = ahead(space, transform, facing);
            let (agent, all_round) = (agent(sensor, of), sight.half_angle >= PI);
            let cos = sight.half_angle.cos();
            for (entity, point) in grid.near(eye, sight.reach()) {
                if entity == sensor || entity == agent {
                    continue;
                }
                let to = point - eye;
                // What it already sees, it keeps seeing further out.
                let range = if candidates.sensing.contains(&entity) {
                    sight.reach()
                } else {
                    sight.range
                };
                if to.length_squared() > range * range {
                    continue;
                }
                if all_round || to == Vec3::ZERO || to.normalize().dot(ahead) >= cos {
                    candidates.sense(entity, point);
                }
            }
        });
}

/// Which way a sensor looks: its [`Facing`], else as its transform does.
pub(crate) fn ahead(space: Space, transform: &GlobalTransform, facing: Option<&Facing>) -> Vec3 {
    facing.map_or_else(
        || space.ahead(transform),
        |facing| space.flatten(*facing.0).normalize_or_zero(),
    )
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
            assert_eq!(percept.trace().unwrap().position, lost_at, "{space:?}");
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
                percept.trace().unwrap().position,
                Space::flatten(space, ahead(space, 5.0)),
                "{space:?}"
            );
        }
    }
}
