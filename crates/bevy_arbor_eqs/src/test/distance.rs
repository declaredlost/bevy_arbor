use super::Test;
use crate::anchor::{EnvQueryAnchor, Locate, Located};
use crate::query::{EnvQueryContext, EnvQueryItem};

/// How far an item is from an anchor: from the nearest, if it's at several.
///
/// No answer where the anchor is nowhere.
#[derive(Clone)]
pub struct Distance {
    to: EnvQueryAnchor,
}

impl Distance {
    /// How far from `to`.
    pub fn to(to: impl Locate) -> Self {
        Self {
            to: EnvQueryAnchor::new(to),
        }
    }
}

impl Test for Distance {
    type State = ();

    fn test(&self, (): &mut (), cx: &EnvQueryContext, items: &[EnvQueryItem]) -> Vec<Option<f32>> {
        let anchors = cx.locate(&self.to);
        items
            .iter()
            .map(|item| {
                anchors
                    .iter()
                    .map(|Located { position, .. }| position.distance(item.position))
                    .min_by(f32::total_cmp)
            })
            .collect()
    }
}
