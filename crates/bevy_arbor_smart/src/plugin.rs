use bevy::prelude::*;

use crate::claim::{ClaimedBy, Claims};
use crate::slot::{SlotDisabled, SlotOf, Slots};

/// Smart objects: registers their components, for queries, inspectors, scenes.
///
/// Claiming is a decorator and finding a query generator, so it has no
/// systems of its own.
#[derive(Default)]
pub struct SmartObjectPlugin;

impl Plugin for SmartObjectPlugin {
    fn build(&self, app: &mut App) {
        // Known to the world from the start, so finding slots can query them
        // before any is claimed or disabled.
        let world = app.world_mut();
        world.register_component::<SlotOf>();
        world.register_component::<SlotDisabled>();
        world.register_component::<ClaimedBy>();
        app.register_type::<SlotOf>()
            .register_type::<Slots>()
            .register_type::<SlotDisabled>()
            .register_type::<ClaimedBy>()
            .register_type::<Claims>();
    }
}
