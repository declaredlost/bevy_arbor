use super::Test;
use crate::query::{EnvQueryContext, EnvQueryItem};

/// A random number from 0 up to 1 for each item, from the query's own
/// randomness.
///
/// Scored by, it gives variety among items otherwise alike.
#[derive(Debug, Clone, Copy, Default)]
pub struct Random;

impl Test for Random {
    fn test(&self, cx: &EnvQueryContext, _item: &EnvQueryItem) -> Option<f32> {
        Some(cx.roll())
    }
}
