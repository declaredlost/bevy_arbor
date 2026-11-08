//! Touch from contacts: avian's collisions felt by perception's `Touch`.

use bevy::prelude::*;
use bevy_arbor_perception::PerceptionSystems;
use bevy_arbor_perception::sense::{Touch, Touched};

use crate::avian::prelude::*;
use crate::plugin::schedule;

/// What touches an agent with [`Touch`] is felt: avian's `CollisionStart`,
/// written as [`Touched`], before perception gathers. An agent with `Touch`
/// gets avian's `CollisionEventsEnabled`, so its own collider reports; a
/// collider on a child of it needs one too.
///
/// Only colliders on `layers` touch, every layer unless set: a monster feels
/// a sword's hitbox, not the grass. Needs avian's `PhysicsPlugins`; without
/// them, it warns and nothing is felt.
#[derive(Debug, Clone, Copy)]
pub struct ContactTouchPlugin {
    /// The layers whose colliders are felt.
    pub layers: LayerMask,
}

impl Default for ContactTouchPlugin {
    fn default() -> Self {
        Self {
            layers: LayerMask::ALL,
        }
    }
}

/// The layers whose colliders are felt.
#[derive(Resource, Debug, Clone, Copy)]
struct Felt(LayerMask);

impl Plugin for ContactTouchPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Felt(self.layers)).add_observer(report);
    }

    fn finish(&self, app: &mut App) {
        let Some(schedule) = schedule(app) else {
            warn!("`ContactTouchPlugin` needs `PerceptionPlugin`");
            return;
        };
        // Without avian's collision events, there's nothing to feel: say so,
        // rather than fail each tick.
        if !app.world().contains_resource::<Messages<CollisionStart>>() {
            warn!("`ContactTouchPlugin` needs avian's `PhysicsPlugins`: nothing is felt");
            return;
        }
        app.add_systems(schedule, feel.before(PerceptionSystems::Gather));
    }
}

/// An agent with `Touch`: its collider reports what touches it.
fn report(added: On<Add<Touch>>, mut commands: Commands) {
    // Despawned meanwhile, it's nothing to report for.
    commands
        .entity(added.entity)
        .try_insert_if_new(CollisionEventsEnabled);
}

/// Each contact an agent with `Touch` is in: felt, if what touched it is on
/// the layers felt.
fn feel(
    felt: Res<Felt>,
    mut starts: MessageReader<CollisionStart>,
    agents: Query<(), With<Touch>>,
    layers: Query<&CollisionLayers>,
    mut touched: MessageWriter<Touched>,
) {
    for start in starts.read() {
        let one = start.body1.unwrap_or(start.collider1);
        let other = start.body2.unwrap_or(start.collider2);
        for (agent, by, collider) in [(one, other, start.collider2), (other, one, start.collider1)]
        {
            let on = layers
                .get(collider)
                .map_or(CollisionLayers::default().memberships, |layers| {
                    layers.memberships
                });
            if agent != by && agents.contains(agent) && (on & felt.0) != LayerMask::NONE {
                touched.write(Touched { agent, by });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;
    use bevy_arbor_perception::prelude::*;
    use bevy_harness::TestApp;

    use super::ContactTouchPlugin;
    use crate::avian::prelude::*;
    use crate::testing::*;

    /// An agent that feels, and something overlapping it on `layers`.
    fn bump(app: &mut TestApp, layers: CollisionLayers) -> (Entity, Entity) {
        let agent = app.spawn((Touch, RigidBody::Static, block(2.0), Transform::default()));
        // Moving, as avian finds no contacts between two static bodies.
        let by = app.spawn((
            RigidBody::Dynamic,
            GravityScale(0.0),
            block(2.0),
            layers,
            Transform::from_translation(ahead(0.5)),
        ));
        app.ticks(3);
        (agent, by)
    }

    fn felt(app: &TestApp, agent: Entity, by: Entity) -> bool {
        app.world()
            .get::<Perceived<Touch>>(agent)
            .is_some_and(|felt| felt.get(by).is_some())
    }

    #[test]
    fn a_contact_is_felt() {
        let mut app = app();
        let (agent, by) = bump(&mut app, CollisionLayers::default());
        assert!(felt(&app, agent, by));
    }

    #[test]
    fn only_colliders_on_the_layers_felt_are() {
        let sword = LayerMask(0b100);
        let mut app = TestApp::builder()
            .plugins((
                TransformPlugin,
                PhysicsPlugins::default(),
                PerceptionPlugin::default().with_space(SPACE),
                ContactTouchPlugin { layers: sword },
            ))
            .build();
        let (agent, grass) = bump(&mut app, CollisionLayers::default());
        assert!(!felt(&app, agent, grass), "not on the sword's layer");
        let (agent, blade) = bump(&mut app, CollisionLayers::new(sword, LayerMask::ALL));
        assert!(felt(&app, agent, blade), "on it");
    }

    #[test]
    fn an_agent_despawned_as_it_is_spawned_is_no_trouble() {
        // Something of the game's own despawns what's touched, the moment it's
        // added, before this crate's observer has a chance to report for it.
        let mut app = TestApp::builder()
            .with(|app| {
                app.add_observer(|added: On<Add<Touch>>, mut commands: Commands| {
                    commands.entity(added.entity).despawn();
                });
            })
            .plugins((
                TransformPlugin,
                PhysicsPlugins::default(),
                PerceptionPlugin::default().with_space(SPACE),
                crate::AvianPerceptionPlugins,
            ))
            .build();
        app.spawn(Touch);
        app.tick();
    }
}
