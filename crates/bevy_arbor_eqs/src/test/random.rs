use super::Test;
use crate::query::{Item, QueryContext};

/// A number from 0 up to 1 for each item, the query's own: to score by, for
/// variety among items otherwise alike.
#[derive(Debug, Clone, Copy, Default)]
pub struct Random;

impl Test for Random {
    fn test(&self, cx: &QueryContext, _item: &Item) -> Option<f32> {
        Some(cx.roll())
    }
}
