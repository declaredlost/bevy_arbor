use std::sync::Arc;

use super::{Generate, angles, round};
use crate::anchor::{Locate, Located};
use crate::query::{Item, QueryContext};

/// Points on rings round an anchor, in the plane things stand on: cover
/// round the agent, a spot round the target.
#[derive(Clone)]
pub struct Donut {
    around: Arc<dyn Locate>,
    inner: f32,
    outer: f32,
    rings: u32,
    points: u32,
}

impl Donut {
    /// Rings round `around`: one, 5 out, of 12 points, unless told otherwise.
    pub fn around(around: impl Locate) -> Self {
        Self {
            around: Arc::new(around),
            inner: 5.0,
            outer: 5.0,
            rings: 1,
            points: 12,
        }
    }

    /// Rings from `inner` out to `outer`, evenly spaced.
    #[must_use]
    pub fn radius(mut self, inner: f32, outer: f32) -> Self {
        self.inner = inner.min(outer);
        self.outer = outer.max(inner);
        self
    }

    /// `rings` rings.
    #[must_use]
    pub fn rings(mut self, rings: u32) -> Self {
        self.rings = rings.max(1);
        self
    }

    /// `points` points on each.
    #[must_use]
    pub fn points(mut self, points: u32) -> Self {
        self.points = points.max(1);
        self
    }
}

impl Generate for Donut {
    fn generate(&self, cx: &QueryContext, out: &mut Vec<Item>) {
        let anchors = cx.locate(&self.around);
        for &Located { position, .. } in anchors.iter() {
            for ring in 0..self.rings {
                #[expect(clippy::cast_precision_loss, reason = "a count of rings")]
                let along = if self.rings == 1 {
                    0.0
                } else {
                    ring as f32 / (self.rings - 1) as f32
                };
                let radius = self.inner + (self.outer - self.inner) * along;
                for angle in angles(self.points) {
                    out.push(Item::Point(round(cx.space(), position, angle, radius)));
                }
            }
        }
    }
}
