use bevy::utils::prelude::ShortName;

use crate::behavior::{Context, Outcome};

/// A decorator that acts as its branch starts and ends, as a cooldown or a
/// claim does: what decides whether its branch may run, told as it starts and
/// ends.
///
/// It goes in `when(..)` after a comma, never under `!`, `&&` or `||`:
/// `when(has_target, cooldown(8.0))`. Whenever its branch runs, then, it was
/// asked and passed, so it's told the branch started, and later that it ended.
/// A decorator that only answers is a [`Condition`](super::Condition), which
/// combines with others.
///
/// It runs from [`start`](Self::start), as its branch starts, through
/// [`passes`](Self::passes), to [`end`](Self::end), as its branch ends: the
/// tree calls them, so it only says what it does at each. It reads the world,
/// and changes it with [`Context::commands`]: they're applied once its tree's
/// tick is done. Say what you wait on as you're asked, with
/// [`Context::wake_at`] or [`Context::watch`], or [`Context::wait_on_nothing`]
/// when only your branch changes you: one that says nothing is asked again
/// every tick.
///
/// ```
/// use bevy::prelude::*;
/// use bevy_arbor_bt::behavior::Context;
/// use bevy_arbor_bt::decorator::Decorate;
/// use bevy_arbor_bt::prelude::*;
///
/// /// How many are in a branch at once.
/// #[derive(Component, Default)]
/// struct Slots(u32);
///
/// /// At most `max` agents in its branch at once, counted on `pool`.
/// struct Limit {
///     max: u32,
///     pool: Entity,
/// }
///
/// impl Decorate for Limit {
///     fn start(&mut self, cx: &mut Context) {
///         cx.commands()
///             .entity(self.pool)
///             .entry::<Slots>()
///             .and_modify(|mut slots| slots.0 += 1);
///     }
///
///     fn passes(&mut self, cx: &mut Context) -> bool {
///         cx.watch::<Slots>(self.pool);
///         cx.world()
///             .get::<Slots>(self.pool)
///             .is_some_and(|slots| slots.0 < self.max)
///     }
///
///     fn end(&mut self, cx: &mut Context, _outcome: Option<Outcome>) {
///         cx.commands()
///             .entity(self.pool)
///             .entry::<Slots>()
///             .and_modify(|mut slots| slots.0 -= 1);
///     }
/// }
/// ```
pub trait Decorate: Send + Sync + 'static {
    /// Its branch starts.
    fn start(&mut self, _cx: &mut Context) {}

    /// Whether its branch may run. Asked before it starts, and by a `Select`
    /// or `RepeatWhile` again each time their tree wakes, while it runs.
    fn passes(&mut self, cx: &mut Context) -> bool;

    /// Its branch ends, with how it went, or as it's cancelled, with `None`.
    fn end(&mut self, _cx: &mut Context, _outcome: Option<Outcome>) {}

    /// How it reads in a trace: its type's short name, unless it says
    /// otherwise. Built only for a trace.
    fn label(&self) -> String {
        ShortName::of::<Self>().to_string()
    }
}
