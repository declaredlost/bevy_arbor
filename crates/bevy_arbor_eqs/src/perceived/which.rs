use bevy_arbor_perception::perceived::Percept;

/// Which percepts are meant: all, or only those active, hostile, or both.
#[derive(Clone, Copy, Default)]
pub(crate) struct Which {
    active: bool,
    hostile: bool,
}

impl Which {
    /// Only what's active, too.
    pub(crate) fn active(self) -> Self {
        Self {
            active: true,
            ..self
        }
    }

    /// Only what's hostile, too.
    pub(crate) fn hostile(self) -> Self {
        Self {
            hostile: true,
            ..self
        }
    }

    /// Whether `percept` is one of them.
    pub(crate) fn keeps(self, percept: &Percept) -> bool {
        (!self.active || percept.is_active()) && (!self.hostile || percept.is_hostile())
    }
}
