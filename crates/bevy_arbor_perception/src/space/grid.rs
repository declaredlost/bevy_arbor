use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use super::Space;

/// Everything that can be seen, by where it is: cells as large as the longest
/// sight, rebuilt every tick, so a look only goes through the cells around it.
///
/// Rebuilding costs about 10 ns a thing; at 2,000 agents, looking through it
/// is several times faster than going through everything.
#[derive(Resource, Default)]
pub(crate) struct Grid {
    size: f32,
    space: Space,
    cells: HashMap<IVec3, Vec<(Entity, Vec3)>>,
}

impl Grid {
    /// Puts `things` in cells `size` across, keeping the memory it had.
    pub(crate) fn rebuild(
        &mut self,
        size: f32,
        space: Space,
        things: impl Iterator<Item = (Entity, Vec3)>,
    ) {
        // Cells left empty last time are dropped, so it doesn't grow without
        // end as things move about.
        self.cells.retain(|_, bucket| !bucket.is_empty());
        for bucket in self.cells.values_mut() {
            bucket.clear();
        }
        self.size = size;
        self.space = space;
        if size <= 0.0 {
            return;
        }
        for (entity, point) in things {
            self.cells
                .entry(space.cell(point, size))
                .or_default()
                .push((entity, point));
        }
    }

    /// What's in the cells within `radius` of `point`: a superset of what's
    /// within `radius`, for the caller to measure. Nothing, if nothing can be
    /// seen.
    pub(crate) fn near(&self, point: Vec3, radius: f32) -> impl Iterator<Item = (Entity, Vec3)> {
        let (size, space) = (self.size, self.space);
        let live = size > 0.0;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a whole number of cells, clamped to a sane count first"
        )]
        let reach = if live {
            (radius / size).ceil().clamp(1.0, 1_024.0) as i32
        } else {
            0
        };
        let centre = if live {
            space.cell(point, size)
        } else {
            IVec3::ZERO
        };
        let depth = space.depth(reach);
        (-reach..=reach)
            .flat_map(move |x| {
                (-reach..=reach)
                    .flat_map(move |y| (-depth..=depth).map(move |z| IVec3::new(x, y, z)))
            })
            .filter(move |_| live)
            .filter_map(move |offset| self.cells.get(&(centre + offset)))
            .flatten()
            .copied()
    }
}
