use std::fmt;
use std::marker::PhantomData;

use bevy::prelude::*;

use crate::sense::Sense;

/// Writes the parts of a perception event: Clone, Copy, Debug and the
/// constructor, without asking anything of `S`.
macro_rules! event {
    ($name:ident) => {
        impl<S: Sense> $name<S> {
            pub(crate) fn new(entity: Entity, what: Entity) -> Self {
                Self {
                    entity,
                    what,
                    sense: PhantomData,
                }
            }
        }

        impl<S: Sense> Clone for $name<S> {
            fn clone(&self) -> Self {
                *self
            }
        }

        impl<S: Sense> Copy for $name<S> {}

        impl<S: Sense> fmt::Debug for $name<S> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct(&format!(
                    concat!(stringify!($name), "<{}>"),
                    S::short_type_path()
                ))
                .field("entity", &self.entity)
                .field("what", &self.what)
                .finish()
            }
        }
    };
}

/// Triggered on an agent as something comes to its attention by `S`: sensed
/// when nothing remembered it that way, or active again.
#[derive(EntityEvent)]
pub struct Noticed<S: Sense> {
    /// The agent.
    pub entity: Entity,
    /// What it noticed.
    pub what: Entity,
    sense: PhantomData<fn() -> S>,
}

/// Triggered on an agent as something stops being active for `S`, a
/// sustained sense: out of view, for sight.
#[derive(EntityEvent)]
pub struct Lost<S: Sense> {
    /// The agent.
    pub entity: Entity,
    /// What it lost.
    pub what: Entity,
    sense: PhantomData<fn() -> S>,
}

/// Triggered on an agent as it forgets what it sensed by `S`.
#[derive(EntityEvent)]
pub struct Forgotten<S: Sense> {
    /// The agent.
    pub entity: Entity,
    /// What it forgot.
    pub what: Entity,
    sense: PhantomData<fn() -> S>,
}

event!(Noticed);
event!(Lost);
event!(Forgotten);

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use crate::prelude::*;
    use crate::testing::*;

    #[test]
    fn noticed_lost_and_noticed_again_as_it_comes_and_goes() {
        for space in SPACES {
            let mut app = logged(space);
            remembering(&mut app, 10.0, 5.0);
            let target = thing(&mut app, ahead(space, 5.0));
            let noisy = app.spawn(GlobalTransform::IDENTITY);
            app.tick();
            app.entity_mut(target)
                .insert(GlobalTransform::from_translation(ahead(space, 50.0)));
            app.tick();
            app.entity_mut(target)
                .insert(GlobalTransform::from_translation(ahead(space, 5.0)));
            app.tick();
            // Heard twice: noticed the first time only.
            app.write(Noise::new(noisy, ahead(space, 2.0)));
            app.tick();
            app.write(Noise::new(noisy, ahead(space, 3.0)));
            app.tick();
            assert_eq!(
                log(&app),
                [
                    ("noticed", "sight", target),
                    ("lost", "sight", target),
                    ("noticed", "sight", target),
                    ("noticed", "hearing", noisy),
                ],
                "{space:?}"
            );
        }
    }
}
