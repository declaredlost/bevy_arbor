use crate::behavior::{Context, Outcome};
use crate::inspect::Inspect;

/// A decorator acting as its branch starts and ends, as a cooldown or a claim
/// does.
///
/// It decides whether its branch may run, and is told as it starts and ends.
///
/// It goes in `when(..)` after a comma, never under `!`, `&&` or `||`:
/// `when(has_target, Limit::new(3))`. Whenever its branch runs, then, it was
/// asked and passed, so it's told the branch started, and later that it ended.
/// A decorator that only answers is a [`Condition`](super::Condition), which
/// combines with others.
///
/// It runs from [`start`](Self::start), as its branch starts, through
/// [`passes`](Self::passes), to [`end`](Self::end), as its branch ends: the
/// tree calls them, so it only says what it does at each. Say what you wait on
/// as you're asked, with [`Context::wake_at`] or [`Context::watch`], or
/// [`Context::wait_on_nothing`] when only your branch changes you: one that
/// says nothing is asked again every tick.
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
/// #[derive(Inspect)]
/// struct Limit {
///     max: u32,
///     pool: Entity,
/// }
///
/// impl Decorate for Limit {
///     fn start(&mut self, cx: &mut Context) {
///         if let Some(mut slots) = cx.world_mut().get_mut::<Slots>(self.pool)
///         {
///             slots.0 += 1;
///         }
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
///         if let Some(mut slots) = cx.world_mut().get_mut::<Slots>(self.pool)
///         {
///             slots.0 -= 1;
///         }
///     }
/// }
/// ```
pub trait Decorate: Inspect + Send + Sync + 'static {
    /// Its branch starts.
    fn start(&mut self, _cx: &mut Context) {}

    /// Whether its branch may run, asked before it starts and again while it
    /// runs.
    ///
    /// While it runs, a `Select` or `RepeatWhile` asks again once what it waits
    /// on comes.
    fn passes(&mut self, cx: &mut Context) -> bool;

    /// Its branch ends, with how it went, or as it's cancelled, with `None`.
    fn end(&mut self, _cx: &mut Context, _outcome: Option<Outcome>) {}
}
