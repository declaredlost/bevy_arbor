use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use super::Space;

/// Everything that can be seen, by where it is: cells as large as a typical
/// sight, rebuilt on the ticks a sensor looks, so a look only goes through
/// the cells around it.
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
    /// How large its cells are.
    #[cfg(test)]
    pub(crate) fn size(&self) -> f32 {
        self.size
    }

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
        let center = if live {
            space.cell(point, size)
        } else {
            IVec3::ZERO
        };
        let depth = space.depth(reach);
        // Further than a few cells, going through every cell there is beats
        // looking up each one round it.
        let around = (2 * i64::from(reach) + 1).pow(2) * (2 * i64::from(depth) + 1);
        let all = live && around > i64::try_from(self.cells.len()).unwrap_or(i64::MAX);
        let every = self.cells.values().filter(move |_| all).flatten().copied();
        // Round it only when it's not going through every cell: else none.
        let reach = if live && !all { reach } else { -1 };
        let local = (-reach..=reach)
            .flat_map(move |x| {
                (-reach..=reach)
                    .flat_map(move |y| (-depth..=depth).map(move |z| IVec3::new(x, y, z)))
            })
            .filter_map(move |offset| self.cells.get(&(center + offset)))
            .flatten()
            .copied();
        every.chain(local)
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use super::Grid;
    use crate::Space;

    fn found(grid: &Grid, point: Vec3, radius: f32) -> Vec<Entity> {
        let mut found: Vec<_> = grid.near(point, radius).map(|(entity, _)| entity).collect();
        found.sort();
        found
    }

    #[test]
    fn a_look_further_than_a_few_cells_goes_through_every_cell_once() {
        let things: Vec<_> = (0..4u32)
            .map(|i| {
                (
                    Entity::from_raw_u32(i + 1).unwrap(),
                    Vec3::new(f32::from(u16::try_from(i).unwrap()) * 100.0, 0.0, 0.0),
                )
            })
            .collect();
        let mut grid = Grid::default();
        grid.rebuild(1.0, Space::Volume, things.iter().copied());
        let mut all: Vec<_> = things.iter().map(|(entity, _)| *entity).collect();
        all.sort();
        // A thousand cells round, in 3D: far more than the four there are.
        assert_eq!(found(&grid, Vec3::ZERO, 1_000.0), all, "each once");
        // Fewer cells round than there are: still every one, once.
        assert_eq!(found(&grid, Vec3::ZERO, 1.0), all);
        // More cells round than there are, a grid of many: only round it.
        let many: Vec<_> = (0..100u32)
            .map(|i| {
                (
                    Entity::from_raw_u32(i + 1).unwrap(),
                    Vec3::new(f32::from(u16::try_from(i).unwrap()) * 10.0, 0.0, 0.0),
                )
            })
            .collect();
        grid.rebuild(1.0, Space::Volume, many.iter().copied());
        assert_eq!(found(&grid, Vec3::ZERO, 1.0), [many[0].0], "only near");
    }
}
