use std::sync::Arc;

use super::{Generate, round};
use crate::anchor::{Locate, Located};
use crate::query::{Item, QueryContext};

/// Points in a cone ahead of an anchor, in the plane things stand on: rays
/// fanned across it, points along each.
#[derive(Clone)]
pub struct Fan {
    from: Arc<dyn Locate>,
    angle: f32,
    near: f32,
    far: f32,
    rays: u32,
    steps: u32,
}

impl Fan {
    /// A cone ahead of `from`: 90° across, 1 to 8 out, 5 rays of 4 points,
    /// unless told otherwise. An anchor looking nowhere has none.
    pub fn from(from: impl Locate) -> Self {
        Self {
            from: Arc::new(from),
            angle: std::f32::consts::FRAC_PI_2,
            near: 1.0,
            far: 8.0,
            rays: 5,
            steps: 4,
        }
    }

    /// `angle` across, in radians.
    #[must_use]
    pub fn angle(mut self, angle: f32) -> Self {
        self.angle = angle.max(0.0);
        self
    }

    /// From `near` out to `far`.
    #[must_use]
    pub fn range(mut self, near: f32, far: f32) -> Self {
        self.near = near.min(far).max(0.0);
        self.far = far.max(near);
        self
    }

    /// `rays` rays of `steps` points.
    #[must_use]
    pub fn rays(mut self, rays: u32, steps: u32) -> Self {
        self.rays = rays.max(1);
        self.steps = steps.max(1);
        self
    }
}

/// Where `at` of the way between `a` and `b` is, with one of `count` it's
/// in the middle.
#[expect(clippy::cast_precision_loss, reason = "counts of rays and steps")]
fn along(index: u32, count: u32, a: f32, b: f32) -> f32 {
    if count == 1 {
        return f32::midpoint(a, b);
    }
    a + (b - a) * index as f32 / (count - 1) as f32
}

impl Generate for Fan {
    fn generate(&self, cx: &QueryContext, out: &mut Vec<Item>) {
        let anchors = cx.locate(&self.from);
        let space = cx.space();
        for &Located {
            position, ahead, ..
        } in anchors.iter()
        {
            if ahead == bevy::math::Vec3::ZERO {
                continue;
            }
            // The angle ahead is at, round the plane.
            let facing = match space {
                bevy_arbor_perception::Space::Planar => ahead.y.atan2(ahead.x),
                bevy_arbor_perception::Space::Volume => (-ahead.z).atan2(ahead.x),
            };
            let half = self.angle / 2.0;
            for ray in 0..self.rays {
                let angle = facing + along(ray, self.rays, -half, half);
                for step in 0..self.steps {
                    let radius = along(step, self.steps, self.near, self.far);
                    out.push(Item::Point(round(space, position, angle, radius)));
                }
            }
        }
    }
}
