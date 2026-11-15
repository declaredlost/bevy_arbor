use bevy::prelude::*;

/// Which of what a query keeps it gives: the best, one of the best, or all.
#[derive(Reflect, Debug, Clone, Copy, PartialEq, Default)]
#[reflect(Debug, PartialEq, Default)]
pub enum Pick {
    /// The best.
    #[default]
    Best,
    /// One at random among the best: this share of them, from 0 to 1, at
    /// least one.
    ///
    /// A share past either end is taken as that end.
    ///
    /// `RandomOfBest(0.05)` is one of the best 5%.
    RandomOfBest(f32),
    /// All of them, best first.
    All,
}
