use bevy::prelude::*;
use bevy_arbor_perception::Space;

use super::Generate;
use super::shape::steps;
use crate::anchor::{EnvQueryAnchor, Locate, Located};
use crate::query::{EnvQueryContext, EnvQueryItem};

/// Points on a square grid round an anchor, in the plane things stand on.
///
/// At most 101 each way, spread further apart than asked if need be.
#[derive(Clone)]
pub struct Grid {
    around: EnvQueryAnchor,
    extent: f32,
    spacing: f32,
}

impl Grid {
    /// A grid round `around`: 5 out each way, 1 apart, unless told otherwise.
    pub fn around(around: impl Locate) -> Self {
        Self {
            around: EnvQueryAnchor::new(around),
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
    type State = ();

    fn generate(&self, (): &mut (), cx: &EnvQueryContext, out: &mut Vec<EnvQueryItem>) {
        let anchors = cx.locate(&self.around);
        let (steps, spacing) = steps("Grid", self.extent, self.spacing);
        for &Located { position, .. } in anchors.iter() {
            for a in -steps..=steps {
                for b in -steps..=steps {
                    #[expect(clippy::cast_precision_loss, reason = "a step count")]
                    let (a, b) = (a as f32 * spacing, b as f32 * spacing);
                    let offset = match cx.space() {
                        Space::Planar => Vec3::new(a, b, 0.0),
                        Space::Volume => Vec3::new(a, 0.0, b),
                    };
                    out.push(EnvQueryItem::place(position + offset));
                }
            }
        }
    }
}
