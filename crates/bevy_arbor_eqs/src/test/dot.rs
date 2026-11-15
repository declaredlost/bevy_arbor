use bevy::prelude::*;

use super::Test;
use crate::anchor::{EnvQueryAnchor, Locate, Located};
use crate::query::{EnvQueryContext, EnvQueryItem};

/// How much an anchor faces an item: 1 straight ahead, 0 to its side, -1
/// behind.
///
/// The most, if it's at several. No answer where it's nowhere, or looks
/// nowhere.
///
/// `Dot::of(Percepts::<Sight>::all().active().hostile())` filtered to below 0
/// keeps what's behind the enemies: a flank.
#[derive(Clone)]
pub struct Dot {
    of: EnvQueryAnchor,
}

impl Dot {
    /// How much `of` faces each item.
    pub fn of(of: impl Locate) -> Self {
        Self {
            of: EnvQueryAnchor::new(of),
        }
    }
}

impl Test for Dot {
    type State = ();

    fn test(&self, (): &mut (), cx: &EnvQueryContext, items: &[EnvQueryItem]) -> Vec<Option<f32>> {
        let anchors = cx.locate(&self.of);
        items
            .iter()
            .map(|item| {
                anchors
                    .iter()
                    .filter(|Located { ahead, .. }| *ahead != Vec3::ZERO)
                    .map(
                        |Located {
                             position, ahead, ..
                         }| {
                            (item.position - *position).normalize_or_zero().dot(*ahead)
                        },
                    )
                    .max_by(f32::total_cmp)
            })
            .collect()
    }
}
