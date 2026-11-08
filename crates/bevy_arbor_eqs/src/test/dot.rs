use std::sync::Arc;

use bevy::prelude::*;

use super::Test;
use crate::anchor::{Locate, Located};
use crate::query::{Item, QueryContext};

/// How much an anchor faces an item: 1 straight ahead of it, 0 to its side,
/// -1 behind it. The most, if it's at several. No answer where it's nowhere,
/// or looks nowhere.
///
/// `Dot::of(Hostiles::<Sight>::active())` filtered to below 0 keeps what's
/// behind the enemies: a flank.
#[derive(Clone)]
pub struct Dot {
    of: Arc<dyn Locate>,
}

impl Dot {
    /// How much `of` faces each item.
    pub fn of(of: impl Locate) -> Self {
        Self { of: Arc::new(of) }
    }
}

impl Test for Dot {
    fn test(&self, cx: &QueryContext, item: &Item) -> Option<f32> {
        cx.locate(&self.of)
            .iter()
            .filter(|Located { ahead, .. }| *ahead != Vec3::ZERO)
            .map(
                |Located {
                     position, ahead, ..
                 }| {
                    (item.position() - *position)
                        .normalize_or_zero()
                        .dot(*ahead)
                },
            )
            .max_by(f32::total_cmp)
    }
}
