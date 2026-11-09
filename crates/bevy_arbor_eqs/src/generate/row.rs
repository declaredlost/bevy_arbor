use std::sync::Arc;

use bevy::prelude::*;

use super::{Generate, MOST_STEPS, steps};
use crate::anchor::{Locate, Located};
use crate::query::{Item, QueryContext};

/// Points on a line through an anchor, the way it looks: behind it and ahead,
/// at most 101, spread further apart than asked if need be. An anchor looking
/// nowhere has none.
///
/// In a side view, an agent looks along the ground: its row is the ground
/// line at its height, where a [`Donut`](super::Donut) would put points in
/// the air too. It doesn't follow the ground up or down: points over a drop
/// or in a slope are left for a test to rule out.
#[derive(Clone)]
pub struct Row {
    along: Arc<dyn Locate>,
    extent: f32,
    spacing: f32,
}

impl Row {
    /// A row through `along`: 5 out each way, 1 apart, unless told otherwise.
    pub fn along(along: impl Locate) -> Self {
        Self {
            along: Arc::new(along),
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

impl Generate for Row {
    fn generate(&self, cx: &QueryContext, out: &mut Vec<Item>) {
        let anchors = cx.locate(&self.along);
        let (steps, spacing) = steps(self.extent, self.spacing).unwrap_or_else(|spread| {
            warn_once!(
                "a Row {} out, {} apart, is too many points: spread {spread} apart instead",
                self.extent,
                self.spacing,
            );
            (MOST_STEPS, spread)
        });
        for &Located {
            position, ahead, ..
        } in anchors.iter()
        {
            if ahead == Vec3::ZERO {
                continue;
            }
            for step in -steps..=steps {
                #[expect(clippy::cast_precision_loss, reason = "a step count")]
                let out_by = step as f32 * spacing;
                out.push(Item::Point(position + ahead * out_by));
            }
        }
    }
}
