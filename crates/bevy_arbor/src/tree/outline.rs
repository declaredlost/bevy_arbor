use std::any::type_name;
use std::fmt::{Display, Write};

use crate::behaviour::Behaviour;

/// A tree written out, with `>` on what's running.
#[derive(Debug, Default)]
pub struct TreeOutline {
    text: String,
    depth: usize,
    /// Whether the node being written is running.
    running: bool,
    /// Whether the next child has a note above it, and so goes deeper.
    noted: bool,
}

impl TreeOutline {
    /// `root` written out.
    pub(crate) fn of(root: &Behaviour) -> String {
        let mut out = Self::default();
        root.outline(&mut out);
        out.text
    }

    /// The last part of `T`'s name, without its generics.
    pub(crate) fn short<T: ?Sized>() -> &'static str {
        let name = type_name::<T>();
        let name = name.split('<').next().unwrap_or(name);
        name.rsplit("::").next().unwrap_or(name)
    }

    pub(crate) fn mark(&mut self, running: bool) {
        self.running = running;
    }

    /// Writes the line for the node itself.
    pub fn line(&mut self, label: impl Display) {
        let mark = if self.running { ">" } else { " " };
        self.write(mark, self.depth, label);
    }

    /// Writes a line about the next child, above it: a guard, say.
    pub fn note(&mut self, note: impl Display) {
        self.write(" ", self.depth + 1, note);
        self.noted = true;
    }

    /// Writes a child, under the node.
    pub fn child(&mut self, child: &Behaviour) {
        let depth = self.depth;
        self.depth += if self.noted { 2 } else { 1 };
        self.noted = false;
        child.outline(self);
        self.depth = depth;
    }

    fn write(&mut self, mark: &str, depth: usize, text: impl Display) {
        let indent = depth * 2;
        let _ = writeln!(self.text, "{mark} {:indent$}{text}", "");
    }
}
