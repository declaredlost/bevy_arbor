use std::sync::Arc;

use bevy::prelude::*;
use bevy_arbor_perception::Space;

use super::Generate;
use crate::anchor::{Locate, Located};
use crate::query::{Item, QueryContext};

/// How many points a grid has each way of its anchor, at most: a grid asking
/// for more is spread further apart, so a tiny spacing can't hang the game.
const MOST_STEPS: u16 = 50;

/// Points on a square grid round an anchor, in the plane things stand on: at
/// most 101 each way, spread further apart than asked if need be.
#[derive(Clone)]
pub struct Grid {
    around: Arc<dyn Locate>,
    extent: f32,
    spacing: f32,
}

impl Grid {
    /// A grid round `around`: 5 out each way, 1 apart, unless told otherwise.
    pub fn around(around: impl Locate) -> Self {
        Self {
            around: Arc::new(around),
            extent: 5.0,
            spacing: 1.0,
        }
    }

    /// Out `extent` each way.
    #[must_use]
    pub fn extent(mut self, extent: f32) -> Self {
        self.extent = extent.max(0.0);
        self
    }

    /// Points `spacing` apart.
    #[must_use]
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing.max(f32::EPSILON);
        self
    }
}

impl Generate for Grid {
    fn generate(&self, cx: &QueryContext, out: &mut Vec<Item>) {
        let anchors = cx.locate(&self.around);
        let wanted = (self.extent / self.spacing).floor();
        let (steps, spacing) = if wanted > f32::from(MOST_STEPS) {
            warn_once!(
                "a Grid {} out, {} apart, is too many points: spread {} apart instead",
                self.extent,
                self.spacing,
                self.extent / f32::from(MOST_STEPS)
            );
            (MOST_STEPS, self.extent / f32::from(MOST_STEPS))
        } else {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "at most MOST_STEPS, and never negative"
            )]
            let steps = wanted as u16;
            (steps, self.spacing)
        };
        let steps = i32::from(steps);
        for &Located { position, .. } in anchors.iter() {
            for a in -steps..=steps {
                for b in -steps..=steps {
                    #[expect(clippy::cast_precision_loss, reason = "a step count")]
                    let (a, b) = (a as f32 * spacing, b as f32 * spacing);
                    let offset = match cx.space() {
                        Space::Planar => Vec3::new(a, b, 0.0),
                        Space::Volume => Vec3::new(a, 0.0, b),
                    };
                    out.push(Item::Point(position + offset));
                }
            }
        }
    }
}
