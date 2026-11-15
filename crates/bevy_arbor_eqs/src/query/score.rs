use bevy::prelude::*;

/// How a test's numbers score: normalized over the items from 0 to 1, then
/// weighted.
///
/// The lowest is 0 and the highest 1, turned round for
/// [`lower`](Self::lower), and squared if [`squared`](Self::squared), before
/// the weight.
#[derive(Reflect, Debug, Clone, Copy, PartialEq)]
#[reflect(Debug, PartialEq)]
pub struct Score {
    higher: bool,
    squared: bool,
    weight: f32,
}

impl Score {
    /// The higher the better.
    #[must_use]
    pub fn higher() -> Self {
        Self {
            higher: true,
            squared: false,
            weight: 1.0,
        }
    }

    /// The lower the better.
    #[must_use]
    pub fn lower() -> Self {
        Self {
            higher: false,
            ..Self::higher()
        }
    }

    /// Squared, after it's normalized: what's best stands out more.
    #[must_use]
    pub fn squared(mut self) -> Self {
        self.squared = true;
        self
    }

    /// Weighted by `weight`, against the query's other scores.
    #[must_use]
    pub fn weight(mut self, weight: f32) -> Self {
        self.weight = weight;
        self
    }

    /// What `normalized`, from 0 to 1, scores.
    pub(crate) fn of(self, normalized: f32) -> f32 {
        let turned = if self.higher {
            normalized
        } else {
            1.0 - normalized
        };
        let shaped = if self.squared {
            turned * turned
        } else {
            turned
        };
        shaped * self.weight
    }
}
