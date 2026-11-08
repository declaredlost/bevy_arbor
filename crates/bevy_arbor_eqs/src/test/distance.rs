use std::sync::Arc;

use super::Test;
use crate::anchor::{Locate, Located};
use crate::query::{Item, QueryContext};

/// How far an item is from an anchor: from the nearest, if it's at several.
/// No answer where the anchor is nowhere.
#[derive(Clone)]
pub struct Distance {
    to: Arc<dyn Locate>,
}

impl Distance {
    /// How far from `to`.
    pub fn to(to: impl Locate) -> Self {
        Self { to: Arc::new(to) }
    }
}

impl Test for Distance {
    fn test(&self, cx: &QueryContext, item: &Item) -> Option<f32> {
        cx.locate(&self.to)
            .iter()
            .map(|Located { position, .. }| position.distance(item.position()))
            .min_by(f32::total_cmp)
    }
}
