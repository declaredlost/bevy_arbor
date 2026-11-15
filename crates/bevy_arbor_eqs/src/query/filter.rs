use bevy::prelude::*;

/// What a filter keeps: items whose number is from `min` to `max`, each end
/// included.
#[derive(Reflect, Debug, Clone, Copy, PartialEq)]
#[reflect(Debug, PartialEq)]
pub struct Filter {
    min: f32,
    max: f32,
}

impl Filter {
    /// From `min` to `max`.
    #[must_use]
    pub fn range(min: f32, max: f32) -> Self {
        Self { min, max }
    }

    /// `min` or more.
    #[must_use]
    pub fn at_least(min: f32) -> Self {
        Self::range(min, f32::INFINITY)
    }

    /// `max` or less.
    #[must_use]
    pub fn at_most(max: f32) -> Self {
        Self::range(f32::NEG_INFINITY, max)
    }

    /// A yes or no, `yes`.
    #[must_use]
    pub fn is(yes: bool) -> Self {
        let one = f32::from(u8::from(yes));
        Self::range(one, one)
    }

    /// Whether it keeps `value`.
    pub(crate) fn keeps(self, value: f32) -> bool {
        (self.min..=self.max).contains(&value)
    }
}
