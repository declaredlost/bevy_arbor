use std::sync::Arc;

use bevy::prelude::*;
use bevy_arbor_perception::Space;

use super::{Generate, MOST_STEPS, steps};
use crate::anchor::{Locate, Located};
use crate::query::{Item, QueryContext};

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
        let (steps, spacing) = steps(self.extent, self.spacing).unwrap_or_else(|spread| {
            warn_once!(
                "a Grid {} out, {} apart, is too many points: spread {spread} apart instead",
                self.extent,
                self.spacing,
            );
            (MOST_STEPS, spread)
        });
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
